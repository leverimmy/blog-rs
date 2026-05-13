# blog-rs

Clever_Jimmy 的个人博客。

[blog-rs](https://github.com/leverimmy/blog-rs) 是一个使用 Rust 构建的静态博客，基于 [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) 渲染 Markdown，[KaTeX](https://katex.org/) 渲染 LaTeX 数学公式，[syntect](https://github.com/trishume/syntect) 代码高亮，使用 [Tera](https://keats.github.io/tera/) 作为模板引擎。

从 [Hexo](https://hexo.io/) + [NexT](https://theme-next.js.org/) 迁移而来，兼容原有文章格式和标签插件。

## Usage

```bash
cargo run -- build    # 生成到 public/
cargo run -- serve    # 构建并在 localhost:3000 预览
```

## Code Structure

详细的配置、内容格式、渲染工作流见 [ARCHITECTURE.md](ARCHITECTURE.md)。

## LICENSE

除非另有说明，本仓库的内容采用 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.zh-hans) 许可协议。在遵守许可协议的前提下，您可以自由地分享、修改本文档的内容，但不得用于商业目的。

如果您认为文档的部分内容侵犯了您的合法权益，请联系项目维护者，我们会尽快删除相关内容。
