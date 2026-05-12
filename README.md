# blog-rs

Clever_Jimmy 的个人博客，使用 Rust 构建的静态博客。

基于 [pulldown-cmark](https://github.com/pulldown-cmark/pulldown-cmark) 渲染 Markdown，[KaTeX](https://katex.org/) 渲染 LaTeX 数学公式，[syntect](https://github.com/trishume/syntect) 代码高亮，使用 [Tera](https://keats.github.io/tera/) 作为模板引擎。

从 [Hexo](https://hexo.io/) + [NexT](https://theme-next.js.org/) 迁移而来，兼容原有文章格式和标签插件。

## Usage

```bash
cargo run -- build    # 生成到 public/
cargo run -- serve    # 构建并在 localhost:3000 预览
```

## Code Structure

详细的配置、内容格式、渲染工作流见 [ARCHITECTURE.md](ARCHITECTURE.md)。

## LICENSE

博客中的所有文章采用 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.zh-hans) 协议发布。
