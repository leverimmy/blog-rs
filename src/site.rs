//! Site builder: generates all pages (posts, index, archive, tags, categories, static pages),
//! search index (`search.json`), and copies static assets to the output directory.

use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::config::SiteConfig;
use crate::post::Post;
use crate::render;
use crate::render::markdown;
use crate::template;

use tera::Context as TeraContext;

struct PostData {
    title: String,
    date: String,
    tags: Vec<String>,
    categories: Vec<String>,
    permalink: String,
    excerpt: String,
    has_more: bool,
}

pub fn build(config: &SiteConfig) -> Result<()> {
    let output_dir = Path::new(&config.output_dir);
    let templates_dir = Path::new("templates");

    // Clean output
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)?;
    }
    fs::create_dir_all(output_dir)?;

    // Init template engine
    let tera = template::init(templates_dir)
        .context("Failed to initialize template engine")?;

    // Read all posts
    let posts_dir = config.posts_dir();
    let mut posts: Vec<Post> = Vec::new();
    for entry in walkdir::WalkDir::new(&posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            match Post::parse(path, &config.permalink) {
                Ok(post) => posts.push(post),
                Err(e) => {
                    log::warn!("Skipping {}: {e:#}", path.display());
                }
            }
        }
    }

    // Sort by date descending
    posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

    log::info!("Loaded {} posts", posts.len());

    // Render all posts
    let rendered: Vec<(Post, render::PostHtml)> = posts
        .into_iter()
        .map(|post| {
            let html = render::render_post(&post, &config.theme.code_theme, config.theme.toc);
            (post, html)
        })
        .collect();

    // Generate post data for templates
    let post_data_list: Vec<PostData> = rendered
        .iter()
        .map(|(post, html)| PostData {
            title: post.frontmatter.title.clone(),
            date: post.frontmatter.date.format("%Y-%m-%d").to_string(),
            tags: post.frontmatter.tags.clone(),
            categories: post.frontmatter.categories.clone(),
            permalink: post.permalink.clone(),
            excerpt: html.excerpt_html.clone(),
            has_more: post.has_more,
        })
        .collect();

    // Generate individual post pages
    for (i, (post, html)) in rendered.iter().enumerate() {
        let post_dir = output_dir.join(&post.permalink);
        fs::create_dir_all(&post_dir)?;

        let mut ctx = TeraContext::new();
        ctx.insert("site", &site_json(config));
        ctx.insert("post", &serde_json::json!({
            "title": post.frontmatter.title,
            "date": post.frontmatter.date.format("%Y-%m-%d").to_string(),
            "tags": post.frontmatter.tags,
            "categories": post.frontmatter.categories,
            "permalink": post.permalink,
            "content": html.content,
            "toc_html": html.toc_html,
        }));
        ctx.insert("has_math", &html.has_math);
        ctx.insert("has_mermaid", &post.body.contains("```mermaid"));

        // Prev/next (chronological: prev = older, next = newer)
        if i + 1 < rendered.len() {
            let prev = &rendered[i + 1].0;
            ctx.insert("prev_post", &serde_json::json!({
                "title": prev.frontmatter.title,
                "permalink": prev.permalink,
            }));
        }
        if i > 0 {
            let next = &rendered[i - 1].0;
            ctx.insert("next_post", &serde_json::json!({
                "title": next.frontmatter.title,
                "permalink": next.permalink,
            }));
        }

        let page_html = tera.render("post.html", &ctx)?;
        fs::write(post_dir.join("index.html"), page_html)?;
    }

    log::info!("Generated {} post pages", rendered.len());

    // Generate index pages (paginated)
    let total_pages = (post_data_list.len() as f64 / config.per_page as f64).ceil() as usize;
    let total_pages = total_pages.max(1);

    for page in 0..total_pages {
        let start = page * config.per_page;
        let end = std::cmp::min(start + config.per_page, post_data_list.len());
        let page_posts = &post_data_list[start..end];

        let mut ctx = TeraContext::new();
        ctx.insert("site", &site_json(config));

        let posts_json: Vec<_> = page_posts
            .iter()
            .map(|p| serde_json::json!({
                "title": p.title,
                "date": p.date,
                "categories": p.categories,
                "permalink": p.permalink,
                "excerpt": p.excerpt,
                "has_more": p.has_more,
            }))
            .collect();
        ctx.insert("posts", &posts_json);

        let paginator = serde_json::json!({
            "current": page + 1,
            "total_pages": total_pages,
            "prev": if page > 1 { Some(format!("/page/{}/", page)) } else if page == 1 { Some("/".to_string()) } else { None::<String> },
            "next": if page + 1 < total_pages { Some(format!("/page/{}/", page + 2)) } else { None::<String> },
        });
        ctx.insert("paginator", &paginator);

        // Check if any excerpt on this page contains KaTeX HTML
        let has_math = page_posts.iter().any(|p| p.excerpt.contains("class=\"katex\""));
        ctx.insert("has_math", &has_math);

        let page_html = tera.render("index.html", &ctx)?;

        let dir = if page == 0 {
            output_dir.to_path_buf()
        } else {
            output_dir.join("page").join((page + 1).to_string())
        };
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), page_html)?;
    }

    // Generate archive page
    {
        let mut archive: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
        for p in &post_data_list {
            let year = p.date[..4].to_string();
            archive.entry(year).or_default().push(serde_json::json!({
                "title": p.title,
                "date": p.date,
                "permalink": p.permalink,
                "categories": p.categories,
            }));
        }
        let mut years: Vec<String> = archive.keys().cloned().collect();
        years.sort_by(|a, b| b.cmp(a));

        let archive_vec: Vec<_> = years.iter().map(|y| serde_json::json!({
            "year": y,
            "posts": archive[y],
        })).collect();

        let mut ctx = base_context(config);
        ctx.insert("archive", &archive_vec);
        let html = tera.render("archive.html", &ctx)?;
        let dir = output_dir.join("archives");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }

    // Generate tags pages
    {
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut tag_posts: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for p in &post_data_list {
            for tag in &p.tags {
                if tag.trim().is_empty() { continue; }
                *tag_counts.entry(tag.clone()).or_default() += 1;
                tag_posts.entry(tag.clone()).or_default().push(serde_json::json!({
                    "title": p.title,
                    "date": p.date,
                    "permalink": p.permalink,
                }));
            }
        }

        let tags_json: Vec<_> = tag_counts
            .iter()
            .map(|(tag, count)| serde_json::json!({
                "tag": tag,
                "count": count,
            }))
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("tags", &tags_json);
        let html = tera.render("tags.html", &ctx)?;
        let dir = output_dir.join("tags");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;

        // Individual tag pages
        for (tag, posts) in &tag_posts {
            let mut ctx = base_context(config);
            ctx.insert("tag", tag);
            ctx.insert("posts", posts);
            let html = tera.render("tag.html", &ctx)?;
            let dir = output_dir.join("tags").join(tag);
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("index.html"), html)?;
        }
    }

    // Generate category pages
    {
        let mut cat_counts: HashMap<String, usize> = HashMap::new();
        let mut cat_posts: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for p in &post_data_list {
            for cat in &p.categories {
                if cat.trim().is_empty() { continue; }
                *cat_counts.entry(cat.clone()).or_default() += 1;
                cat_posts.entry(cat.clone()).or_default().push(serde_json::json!({
                    "title": p.title,
                    "date": p.date,
                    "permalink": p.permalink,
                }));
            }
        }

        let cats_json: Vec<_> = cat_counts
            .iter()
            .map(|(cat, count)| serde_json::json!({
                "cat": cat,
                "count": count,
            }))
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("categories", &cats_json);
        let html = tera.render("categories.html", &ctx)?;
        let dir = output_dir.join("categories");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;

        for (cat, posts) in &cat_posts {
            let mut ctx = base_context(config);
            ctx.insert("category", cat);
            ctx.insert("posts", posts);
            let html = tera.render("category.html", &ctx)?;
            let dir = output_dir.join("categories").join(cat);
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("index.html"), html)?;
        }
    }

    // Copy static files
    copy_dir_recursive(Path::new("static"), output_dir)?;

    // Copy gallery if it exists
    let gallery_src = config.gallery_dir();
    if gallery_src.exists() {
        copy_dir_recursive(&gallery_src, &output_dir.join("gallery"))?;
    }

    // Generate highlight CSS
    let highlight_css = crate::render::code_highlight::generate_highlight_css(&config.theme.code_theme);
    fs::write(output_dir.join("css").join("highlight.css"), highlight_css)?;

    // Generate static pages (about, links, services)
    generate_static_pages(config, &tera, output_dir)?;

    // Generate search index
    {
        let search_index: Vec<_> = rendered
            .iter()
            .map(|(post, html)| serde_json::json!({
                "title": post.frontmatter.title,
                "permalink": post.permalink,
                "date": post.frontmatter.date.format("%Y-%m-%d").to_string(),
                "tags": post.frontmatter.tags,
                "categories": post.frontmatter.categories,
                "excerpt": strip_html_tags(&html.excerpt_html),
                "content": strip_html_tags(&html.content),
            }))
            .collect();
        let json = serde_json::to_string(&search_index)?;
        fs::write(output_dir.join("search.json"), json)?;
    }

    // Generate posts metadata (for counter hot list)
    {
        let posts_meta: Vec<_> = rendered
            .iter()
            .map(|(post, _)| serde_json::json!({
                "url": format!("/{}", post.permalink),
                "title": post.frontmatter.title,
            }))
            .collect();
        let json = serde_json::to_string(&posts_meta)?;
        fs::write(output_dir.join("posts-meta.json"), json)?;
    }

    // Generate search page
    {
        let ctx = base_context(config);
        let html = tera.render("search.html", &ctx)?;
        let dir = output_dir.join("search");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;
    }

    log::info!("Site built to {}", output_dir.display());
    Ok(())
}

fn site_json(config: &SiteConfig) -> serde_json::Value {
    serde_json::json!({
        "title": config.title,
        "subtitle": config.subtitle,
        "author": config.author,
        "language": config.language,
        "since": config.since,
        "avatar": config.avatar,
        "github": config.github,
        "email": config.email,
        "motto": config.motto,
        "icp": config.icp,
        "gongan": config.gongan,
        "gongan_id": config.gongan_id,
        "current_year": chrono::Local::now().format("%Y").to_string(),
        "theme": {
            "accent_color": config.theme.accent_color,
        },
        "giscus": {
            "enabled": config.giscus.is_enabled(),
            "repo": config.giscus.repo,
            "repo_id": config.giscus.repo_id,
            "category": config.giscus.category,
            "category_id": config.giscus.category_id,
        },
    })
}

fn base_context(config: &SiteConfig) -> TeraContext {
    let mut ctx = TeraContext::new();
    ctx.insert("site", &site_json(config));
    ctx
}

/// Remove HTML tags and decode common entities, used for search index text.
fn strip_html_tags(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    let text = re.replace_all(html, "").into_owned();
    // Also decode common HTML entities
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let src_path = entry.path();
        let rel = src_path.strip_prefix(src)?;
        let dst_path = dst.join(rel);
        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else {
            fs::copy(src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn generate_static_pages(
    config: &SiteConfig,
    tera: &tera::Tera,
    output_dir: &Path,
) -> Result<()> {
    let pages = [
        ("about", "关于"),
        ("links", "友情链接"),
        ("services", "服务"),
    ];

    for (slug, _default_title) in &pages {
        let md_path = config.source_dir.join(slug).join("index.md");
        if !md_path.exists() {
            continue;
        }

        let content = fs::read_to_string(&md_path)?;
        let (fm, body) = crate::post::split_frontmatter_raw(&content)?;

        let opts = markdown::RenderOptions {
            enable_math: fm.mathjax,
            enable_toc: fm.toc,
            code_theme: config.theme.code_theme.clone(),
        };
        let rendered = crate::render::render_with_hexo_tags(&body, &opts);

        let mut ctx = TeraContext::new();
        ctx.insert("site", &site_json(config));
        ctx.insert("page", &serde_json::json!({
            "title": fm.title,
            "slug": slug,
        }));
        ctx.insert("content", &rendered);
        ctx.insert("has_math", &fm.mathjax);
        ctx.insert("has_mermaid", &body.contains("```mermaid"));

        let html = tera.render("page.html", &ctx)?;
        let dir = output_dir.join(slug);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), html)?;

        log::info!("Generated static page: {slug}");
    }

    Ok(())
}
