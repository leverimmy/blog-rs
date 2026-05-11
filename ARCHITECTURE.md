# blog-rs 技术文档

Rust 驱动的静态博客引擎，兼容 Hexo 内容格式。

## 功能

- **Markdown 渲染** — pulldown-cmark（表格、删除线、任务列表）
- **LaTeX 数学** — 行内 `$...$` 和行间 `$$...$$`，KaTeX 服务端渲染
- **代码高亮** — syntect 逐行高亮，行号、复制按钮、换行切换、可选标题
- **Mermaid 图表** — mermaid.js 客户端渲染（`securityLevel: 'loose'` 支持图中 LaTeX）
- **Hexo 兼容标签** — `{% note %}`, `{% grouppicture %}`, `{% video %}`, `{% pdf %}`
- **目录** — 自动从标题生成，粘性侧边栏，圆点层级指示
- **摘要** — `<!--more-->` 分割（兼容 Hexo 空格变体）
- **分页** — 可配置每页篇数
- **标签和分类** — 索引页、标签云、按标签/分类列表
- **归档** — 按年分组，倒序排列
- **静态页面** — 关于、友链、服务（Markdown + frontmatter）
- **Giscus 评论** — 通过 `config.toml` 配置
- **全文搜索** — Fuse.js 客户端模糊搜索，搜索标题、标签、分类和全文
- **侧边栏个人资料** — 头像、作者名、座右铭、GitHub/Email 链接
- **访问统计** — 自建计数 API（SQLite），详情页/列表页阅读数、热榜
- **ICP 备案** — 可选 ICP 和公安备案链接
- **开发服务器** — tiny_http，支持 CJK URL 百分号编码
- **CJK 兼容** — 正则后处理修复中文字符旁的 `**粗体**` 和 `~~删除线~~`

## 快速开始

```bash
cargo run -- build          # 生成站点到 public/
cargo run -- serve           # 构建并在 localhost:3000 预览
cargo run -- serve -p 8080   # 指定端口
```

## 项目结构

```
blog-rs/
├── config.toml              # 站点配置
├── content/
│   ├── _posts/              # 博客文章（Markdown）
│   ├── about/               # 静态页面
│   ├── links/
│   ├── services/
│   └── gallery/             # 图片画廊
├── src/
│   ├── main.rs              # CLI 入口（build / serve）
│   ├── config.rs            # 加载 config.toml
│   ├── post.rs              # 文章解析、frontmatter、摘要分割
│   ├── utils.rs             # 共享工具（html_escape）
│   ├── render/
│   │   ├── mod.rs           # 渲染管线协调器（三遍渲染）
│   │   ├── markdown.rs      # pulldown-cmark 事件处理
│   │   ├── code_highlight.rs # syntect 代码高亮
│   │   ├── math.rs          # KaTeX 渲染
│   │   ├── hexo_tags.rs     # Hexo 标签提取与渲染
│   │   └── toc.rs           # 目录生成
│   ├── site.rs              # 站点构建器
│   ├── serve.rs             # 开发服务器
│   └── template.rs          # Tera 模板引擎设置
├── counter/
│   ├── counter.py           # 自建访问计数 API（Python + SQLite）
│   └── blog-rs-counter.service  # systemd 服务单元
├── static/
│   ├── css/main.css         # 主样式表
│   ├── css/fonts/           # KaTeX 字体
│   └── js/main.js           # 复制/换行按钮、计数 API 客户端
├── templates/               # Tera HTML 模板
│   ├── base.html            # 布局（头部、底部、CSS/JS）
│   ├── index.html           # 文章列表 + 侧边栏 + 分页 + 热榜
│   ├── post.html            # 单篇文章（侧边栏、TOC、评论、阅读数）
│   ├── page.html            # 静态页面
│   ├── search.html          # 全文搜索页（Fuse.js）
│   ├── archive.html         # 按年归档
│   ├── tags.html / tag.html
│   ├── categories.html / category.html
│   └── partials/post_item.html  # 可复用文章卡片
└── public/                  # 生成输出（gitignore）
```

## 配置

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

| 字段 | 说明 |
|---|---|
| `avatar` | 头像图片路径（侧边栏显示） |
| `github` | GitHub 用户名（侧边栏链接、底部版权链接） |
| `email` | 邮箱（侧边栏 mailto 链接） |
| `motto` | 侧边栏作者名下的标语 |
| `since` | 博客起始年份（底部版权） |
| `icp` | ICP 备案号（留空隐藏） |
| `gongan` | 公安备案号 |
| `gongan_id` | 公安备案记录码 |
| `permalink` | URL 模式：`:year`, `:month`, `:day`, `:id`（来自 frontmatter 的 `id` 字段） |
| `per_page` | 每页文章数 |
| `source_dir` | 内容根目录（文章在 `source_dir/_posts/`） |
| `theme.toc` | 目录侧边栏全局默认（文章 frontmatter 的 `toc` 可覆盖） |
| `theme.code_theme` | syntect 高亮主题（见下表） |
| `theme.accent_color` | 链接、按钮的高亮色 CSS 颜色（默认 `#428bca`） |
| `giscus.repo` | Giscus 评论 GitHub 仓库 |
| `giscus.repo_id` | GitHub 仓库 ID |
| `giscus.category` | Giscus 讨论分类名 |
| `giscus.category_id` | Giscus 讨论分类 ID |

### 可用代码高亮主题

| 主题 | 风格 |
|---|---|
| `InspiredGitHub` | 浅色，GitHub 风格（默认） |
| `base16-ocean-dark` | 深色，海洋调色板 |
| `base16-eighties` | 深色，复古霓虹 |
| `Solarized (dark)` | 深色 solarized |
| `Solarized (light)` | 浅色 solarized |

### Giscus 配置

从 [giscus.app](https://giscus.app/) 获取 `repo`、`repo_id`、`category`、`category_id`。省略 `[giscus]` 段可禁用评论。

## 内容格式

### Frontmatter

文章使用 YAML frontmatter：

```yaml
---
title: 文章标题
date: 2025-01-15 10:30:00
tags:
  - rust
categories:
  - 编程
mathjax: true
toc: true
id: my-post-slug
---
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `title` | 是 | 文章标题 |
| `date` | 是 | 发布日期（`YYYY-MM-DD` 或 `YYYY-MM-DD HH:MM:SS`） |
| `tags` | 否 | 标签列表 |
| `categories` | 否 | 分类列表 |
| `mathjax` | 否 | 启用 KaTeX 数学渲染（默认 false） |
| `toc` | 否 | 启用目录（默认跟随 `theme.toc`） |
| `id` | 否 | URL slug（用于 permalink 模式） |

### 摘要

插入 `<!--more-->` 分割文章。标签前的内容成为首页摘要。支持可选空格（兼容 Hexo）：`<!--more-->`、`<!-- more -->`、`<!-- more-->`、`<!--more -->`。

### Hexo 标签

#### Note 提示框

两种格式：

```markdown
{% note info %}
普通提示框内容（展开显示）。
{% endnote %}

{% note info no-icon 标题文字 %}
默认折叠的提示框，点击标题展开。
{% endnote %}
```

类型：`default`、`primary`、`info`、`success`、`warning`、`danger`。

`no-icon` 可选。如果提供了标题文字，渲染为 `<details>` 折叠块；否则渲染为 `<div>` 展开块。

#### 其他标签

```markdown
{% grouppicture 2-2 %}
![alt](image1.jpg)
![alt](image2.jpg)
{% endgrouppicture %}

{% video /videos/demo.mp4 %}

{% pdf https://example.com/paper.pdf %}
{% pdf https://example.com/paper.pdf [800px] %}
```

### 代码块

````markdown
```python hello.py
print("Hello, world!")
```
````

语言名后的文字成为代码块标题。代码块包含行号、复制按钮和换行切换。

## 搜索

构建时生成 `search.json`（标题、全文、摘要、标签、分类），搜索页使用 [Fuse.js](https://www.fusejs.io/) 进行客户端模糊匹配。

## 访问统计

自建计数 API，基于 Python + SQLite，部署在站点服务器上。

### 架构

- **counter.py** — 轻量 HTTP API 服务，监听 `127.0.0.1:8123`
- **SQLite 数据库** — `counter.db`，存储每篇文章的 URL、标题和阅读数
- **Nginx 反向代理** — `/api/` 请求转发到计数服务

### API

| 端点 | 方法 | 说明 |
|---|---|---|
| `/api/count` | POST | 增加指定 URL 的阅读数并返回当前值。Body: `{"url": "/path/", "title": "标题"}` |
| `/api/counts` | GET | 返回所有文章阅读数，按阅读数降序。支持 `?top=N` 限制返回数量 |

### 功能

- **文章详情页** — 访问时自动调用 API 增加阅读数并显示
- **列表页** — 从 API 获取每篇文章的阅读数并显示
- **热榜** — 首页侧边栏显示阅读数前 5 的文章

### 部署

1. 将 `counter/counter.py` 放到服务器 `/home/www/blog-rs-counter/`
2. 安装 systemd 服务：
   ```bash
   cp counter/blog-rs-counter.service /etc/systemd/system/
   systemctl daemon-reload
   systemctl enable --now blog-rs-counter
   ```
3. Nginx 添加反向代理：
   ```nginx
   location /api/ {
       proxy_pass http://127.0.0.1:8123;
   }
   ```

## 渲染管线

三遍渲染流程：

1. **提取 Hexo 标签** — 正则提取 `{% note %}`、`{% video %}` 等，替换为 `<<PLACEHOLDER_N>>` 占位符
2. **渲染 Markdown** — pulldown-cmark 解析（占位符作为字面文本），拦截：
   - `InlineMath` / `DisplayMath` → KaTeX 渲染
   - `CodeBlock` → syntect 高亮（主题来自配置）或 mermaid 直通
   - `Heading` → TOC 收集 + ID 注入
3. **解析占位符** — 递归渲染内部内容并替换为最终 HTML

最后正则后处理修复中文字符旁的 `**粗体**` 和 `~~删除线~~`。

## 部署

`public/` 目录包含完整静态站点，部署到任何静态托管服务即可。

### GitHub Actions CI/CD

推送到 `main` 分支自动部署：

1. 检出代码 → 安装 Rust → 缓存 Cargo
2. `cargo run -- build` 构建站点
3. ImageMagick 压缩 gallery 图片
4. rsync 部署 `public/` 到服务器（`--delete`）
5. rsync 部署 `counter/` 到服务器（无 `--delete`，保留 `counter.db`）
6. SSH 重启计数服务

### CDN 加载的外部资源

- **Mermaid.js** — 含 mermaid 代码块的页面加载
- **Fuse.js** — 搜索页加载
- **Giscus** — 文章页加载
- **KaTeX CSS & 字体** — 本地打包在 `css/` 和 `css/fonts/`

## 许可

博客文章采用 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.zh-hans) 协议发布。
