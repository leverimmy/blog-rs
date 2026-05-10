# blog-rs

Clever_Jimmy 的个人博客，使用 Rust 构建的静态站点生成器。

基于 [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) 渲染 Markdown，[KaTeX](https://katex.org/) 渲染 LaTeX 数学公式，[syntect](https://github.com/trishume/syntect) 代码高亮，[Tera](https://keats.github.io/tera/) 模板引擎。

从 [Hexo](https://hexo.io/) + [NexT](https://theme-next.js.org/) 迁移而来，兼容原有文章格式和标签插件。

## 构建

```bash
cargo run -- build    # 生成到 public/
cargo run -- serve    # 构建并在 localhost:3000 预览
```

## 技术文档

详细的配置参考、内容格式、渲染管线说明见 [ARCHITECTURE.md](ARCHITECTURE.md)。
