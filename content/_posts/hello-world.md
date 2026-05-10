---
title: Hello World
date: 2024-01-01 12:00:00
tags:
  - blog
categories:
  - 随笔
mathjax: true
toc: true
id: hello-world
---

Welcome to your new blog powered by **blog-rs**!

This is a sample post demonstrating the supported features.

<!--more-->

## Markdown

You can write standard Markdown: **bold**, *italic*, `inline code`, [links](https://example.com), and more.

## Code Blocks

```python hello.py
def greet(name: str) -> str:
    return f"Hello, {name}!"

if __name__ == "__main__":
    print(greet("World"))
```

Code blocks support syntax highlighting, line numbers, a copy button, and a wrap toggle. The text after the language name becomes the block title.

## Math

Inline math: $E = mc^2$, and display math:

$$
\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
$$

## Note Boxes

{% note info %}
This is an info note. Supported types: `default`, `primary`, `info`, `success`, `warning`, `danger`.
{% endnote %}

## Blockquotes

> This is a blockquote. Use it for quotes or callouts.

## Table

| Feature | Status |
|---|---|
| Markdown | Supported |
| LaTeX | Supported |
| Code Highlight | Supported |

---

That's it! Start writing your own posts in `content/_posts/`.
