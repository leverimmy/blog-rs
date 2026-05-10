# blog-rs

A Rust-powered static blog engine, compatible with Hexo content format.

## Features

- **Markdown rendering** via pulldown-cmark (tables, strikethrough, task lists)
- **LaTeX math** — inline `$...$` and display `$$...$$`, server-side rendered by KaTeX
- **Syntax highlighting** — per-line highlighting via syntect, with line numbers, copy button, wrap toggle, and optional title
- **Mermaid diagrams** — rendered client-side via mermaid.js (with `securityLevel: 'loose'` for LaTeX in diagrams)
- **Hexo-compatible tags** — `{% note %}`, `{% grouppicture %}`, `{% video %}`, `{% pdf %}`
- **Table of contents** — auto-generated from headings, displayed in a sticky sidebar with dot indicators
- **Post excerpts** — split by `<!--more-->` (Hexo-compatible whitespace)
- **Pagination** — configurable posts-per-page
- **Tags & categories** — index pages, tag cloud, per-tag/per-category listing
- **Archive** — posts grouped by year, sorted newest-first
- **Static pages** — about, links, services (markdown with frontmatter)
- **Giscus comments** — configurable via `config.toml`, zero setup on the template side
- **Full-text search** — client-side fuzzy search via Fuse.js, searches titles, tags, categories, and full post content
- **Sidebar profile** — avatar, author name, motto, GitHub/email links
- **Visitor stats** — site-wide and per-post view counts via [busuanzi](https://busuanzi.ibruce.info/) (no backend needed)
- **ICP footer** — optional ICP filing and public security filing links
- **Dev server** — `tiny_http` with percent-encoding support for CJK URLs
- **CJK fallback** — regex post-pass for `**bold**` and `~~strikethrough~~` adjacent to Chinese characters

## Quick Start

```bash
# Build the site
cargo run -- build

# Build and serve locally on port 3000
cargo run -- serve

# Serve on a custom port
cargo run -- serve -p 8080
```

Output goes to `public/` by default.

## Project Structure

```
blog-rs/
├── config.toml              # Site configuration
├── content/
│   ├── _posts/              # Blog posts (Markdown)
│   ├── about/               # Static pages
│   ├── links/
│   ├── services/
│   └── gallery/             # Image gallery
├── src/
│   ├── main.rs              # CLI entry point (build / serve)
│   ├── config.rs            # Loads config.toml
│   ├── post.rs              # Post parsing, frontmatter, excerpt splitting
│   ├── utils.rs             # Shared utilities (html_escape)
│   ├── render/
│   │   ├── mod.rs           # Pipeline orchestrator (3-pass rendering)
│   │   ├── markdown.rs      # pulldown-cmark event processing
│   │   ├── code_highlight.rs # syntect syntax highlighting
│   │   ├── math.rs          # KaTeX rendering
│   │   ├── hexo_tags.rs     # Hexo tag extraction & rendering
│   │   └── toc.rs           # Table of contents generation
│   ├── site.rs              # Site builder (pages, posts, archives, tags, search index)
│   ├── serve.rs             # Dev server
│   └── template.rs          # Tera template engine setup
├── static/
│   ├── css/main.css         # Main stylesheet
│   ├── css/fonts/           # KaTeX fonts
│   └── js/main.js           # Copy/wrap buttons, busuanzi post views
├── templates/               # Tera HTML templates
│   ├── base.html            # Layout shell (header, footer, CSS/JS, busuanzi)
│   ├── index.html           # Post list with sidebar + pagination
│   ├── post.html            # Single post (sidebar profile, TOC, comments)
│   ├── page.html            # Static page
│   ├── search.html          # Full-text search page (Fuse.js)
│   ├── archive.html         # Archive by year
│   ├── tags.html / tag.html
│   ├── categories.html / category.html
│   └── partials/post_item.html  # Reusable post card
└── public/                  # Generated output (gitignored)
```

## Configuration

`config.toml`:

```toml
title = "My Blog"
subtitle = ""
author = "Author"
url = "https://example.com/"
language = "zh-CN"
avatar = "/gallery/misc/avatar.jpg"
github = "your-username"
email = "you@example.com"
motto = "Your motto here"
since = 2022
icp = ""
gongan = ""
gongan_id = ""
permalink = ":year/:month/:day/:id/"
per_page = 10
source_dir = "content"
output_dir = "public"

[theme]
toc = true
code_theme = "InspiredGitHub"
accent_color = "#428bca"

[giscus]
repo = "user/repo"
repo_id = "your-repo-id"
category = "Announcements"
category_id = "your-category-id"
```

| Field | Description |
|---|---|
| `avatar` | Path to avatar image (displayed in sidebar) |
| `github` | GitHub username (sidebar link) |
| `email` | Email address (sidebar mailto link) |
| `motto` | Tagline displayed under author name in sidebar |
| `since` | Blog start year (used in copyright footer) |
| `icp` | ICP filing number (shown in footer, leave empty to hide) |
| `gongan` | Public security filing number (shown in footer) |
| `gongan_id` | Record code for the public security filing link |
| `permalink` | URL pattern: `:year`, `:month`, `:day`, `:id` (from frontmatter `id` field) |
| `per_page` | Posts per index page |
| `source_dir` | Content root (posts go in `source_dir/_posts/`) |
| `theme.toc` | Enable table of contents sidebar (global default, per-post `toc` overrides) |
| `theme.code_theme` | syntect highlight theme (see below) |
| `theme.accent_color` | CSS color for links, buttons, and highlights (default: `#428bca`) |
| `giscus.repo` | GitHub repo for Giscus comments (e.g. `"user/repo"`) |
| `giscus.repo_id` | GitHub repo ID |
| `giscus.category` | Giscus discussion category name |
| `giscus.category_id` | Giscus discussion category ID |

### Available Code Highlight Themes

These are bundled with syntect:

| Theme | Style |
|---|---|
| `InspiredGitHub` | Light, GitHub-like (default) |
| `base16-ocean-dark` | Dark, ocean palette |
| `base16-eighties` | Dark, retro neon |
| `Solarized (dark)` | Dark solarized |
| `Solarized (light)` | Light solarized |
| `Solarized (eighties)` | Dark, warm tones |

### Giscus Setup

Get your `repo`, `repo_id`, `category`, and `category_id` from [giscus.app](https://giscus.app/). Omit the `[giscus]` section to disable comments.

## Content Format

### Frontmatter

Posts use YAML frontmatter:

```yaml
---
title: Post Title
date: 2025-01-15 10:30:00
tags:
  - rust
  - static-site
categories:
  - programming
mathjax: true
toc: true
id: my-post-slug
---
```

| Field | Required | Description |
|---|---|---|
| `title` | yes | Post title |
| `date` | yes | Publication date (`YYYY-MM-DD` or `YYYY-MM-DD HH:MM:SS`) |
| `tags` | no | List of tags |
| `categories` | no | List of categories |
| `mathjax` | no | Enable KaTeX math rendering (default: false) |
| `toc` | no | Enable table of contents (falls back to `theme.toc` if unset) |
| `id` | no | URL slug (used in permalink pattern) |

### Excerpts

Insert `<!--more-->` to split the post. Content before the tag becomes the excerpt shown on index pages. The tag supports optional spaces (matching Hexo behavior): `<!--more-->`, `<!-- more -->`, `<!-- more-->`, `<!--more -->`.

### Hexo Tags

```
{% note info %}
This is an info note.
{% endnote %}

{% note default|primary|info|success|warning|danger %}
Note content here.
{% endnote %}

{% grouppicture 2-2 %}
![alt](image1.jpg)
![alt](image2.jpg)
{% endgrouppicture %}

{% video /videos/demo.mp4 %}

{% pdf https://example.com/paper.pdf %}
{% pdf https://example.com/paper.pdf [800px] %}
```

### Code Blocks

````markdown
```python hello.py
print("Hello, world!")
```
````

The info string after the language becomes the code block title. Code blocks include line numbers, a copy button, and a wrap toggle.

## Search

A full-text search page is generated at `/search/`. During build, `search.json` is generated containing all posts' titles, full content, excerpts, tags, and categories. The search page uses [Fuse.js](https://www.fusejs.io/) for client-side fuzzy matching — no backend required.

## Visitor Statistics

[Busuanzi](https://busuanzi.ibruce.info/) provides visitor counting with zero configuration:

- **Site-wide**: Total unique visitors and page views displayed in the footer
- **Per-post**: View count shown on each post page and in the post list on index/tag/category pages

Busuanzi is a free third-party service that counts via client-side script injection. It requires the site to be publicly accessible for counting to work.

## Rendering Pipeline

The rendering pipeline processes content in three passes:

1. **Extract hexo tags** — regex-based extraction of `{% note %}`, `{% video %}`, etc., replaced with `<<PLACEHOLDER_N>>` markers
2. **Render markdown** — pulldown-cmark parses the markdown (with placeholders as literal text), intercepting:
   - `InlineMath` / `DisplayMath` events → KaTeX rendering
   - `CodeBlock` events → syntect highlighting (theme from config) or mermaid passthrough
   - `Heading` events → TOC collection + ID injection
3. **Resolve placeholders** — recursively render inner content and replace placeholders with final HTML

A final regex pass fixes `**bold**` and `~~strikethrough~~` that pulldown-cmark misses when delimiters are adjacent to CJK characters.

## Deployment

The `public/` directory contains the complete static site. Deploy it to any static hosting service (GitHub Pages, Netlify, Vercel, nginx, etc.).

External resources loaded from CDN:
- **Mermaid.js** — loaded on pages containing mermaid code blocks
- **Fuse.js** — loaded on the search page
- **Busuanzi** — loaded on all pages for visitor counting
- **Giscus** — loaded on post pages for comments
- **KaTeX CSS & fonts** — bundled locally in `css/` and `css/fonts/`

## License

MIT
