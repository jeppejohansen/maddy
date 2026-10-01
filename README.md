# maddy

**Write Markdown. Write mathematics the way you already know. Get a good PDF.**

`maddy` is a small, fast, opinionated command-line compiler that turns Markdown
containing LaTeX-style mathematics into high-quality PDFs — either as a typeset
**document** or as a widescreen **presentation**.

The binary is called `maddy`.

```bash
maddy paper.md      # → paper.pdf   (a typeset document)
maddy talk.md       # → talk.pdf    (a 16:9 presentation)
```

It aims to be `gcc` for Markdown documents, not a publishing framework. The
common case requires no configuration at all.

---

## Why

Writing a technical paper or a seminar talk usually means choosing between
Markdown (pleasant to write, poor mathematics, mediocre output) and LaTeX
(excellent mathematics, a multi-gigabyte toolchain and a day of yak-shaving).

`maddy` takes the Markdown you already write and the LaTeX mathematics you
already know, and renders both through [Typst](https://typst.app) — so there is
no TeX installation, compilation is fast, and the output is genuinely good.

The same source language produces both documents and slides. Separating slides
is done with `---`. That is the entire presentation syntax.

---

## Example

````markdown
# Regression

Consider the linear model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The OLS estimator is

$$
\hat\beta =
\left( \sum_{i=1}^{n} X_i X_i^\top \right)^{-1}
\sum_{i=1}^{n} X_i Y_i.
$$

Under $\mathbb E[\varepsilon_i \mid X_i] = 0$ the estimator is **consistent**.

```rust
fn estimate(x: Matrix, y: Vector) -> Vector {
    solve(x.t() * x, x.t() * y)
}
```
````

```bash
maddy regression.md
```

### Slides

Add `type: slides` to the front matter and separate slides with `---`:

```markdown
---
type: slides
title: Peer Effects
subtitle: High School Application
author: Jane Researcher
style: academic
---

# Motivation

- Friends share information
- Choices may be strategic complements

---

# Model

$$
p_i = \operatorname{logit}^{-1}\left( X_i^\top\beta + \lambda \bar p_{-i} \right).
$$
```

A title slide is generated automatically from the metadata.

---

## Installation

Requires a Rust toolchain and the [Typst](https://github.com/typst/typst) CLI on
`PATH`.

```bash
cargo install --path .
```

If Typst is not installed you can still generate Typst source:

```bash
maddy --emit typst paper.md
```

---

## Usage

```text
maddy [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Markdown source file

Options:
  -o, --output <PATH>      Output path
      --emit <FORMAT>      pdf | typst | both          [default: pdf]
      --slides             Force presentation mode
      --style <STYLE>      Rendering style
      --config <PATH>      Configuration file
      --template <PATH>    Custom Typst template
      --keep-typst         Keep generated Typst
      --strict             Treat warnings as errors
      --watch              Recompile after changes
  -q, --quiet              Suppress non-error output
  -v, --verbose            Show compilation details
  -h, --help
  -V, --version
```

### Front matter

Recognized only at the very beginning of the file:

```yaml
---
title: Peer Effects in High School Application
subtitle: Strategic Complementarities
author: Jane Researcher
date: October 2026
type: slides      # document | slides
style: academic
---
```

Unknown fields produce warnings, not errors.

### Configuration

An optional `maddy.toml`, looked for beside the document and in each parent
directory, so a project can carry one at its root. `--config` names one
explicitly.

```toml
[document]
paper = "a4"                 # any Typst paper name
font = "Libertinus Serif"
font-size = "11pt"

[page]
margin = "25mm"

[headings]
numbered = false

[code]
font = "DejaVu Sans Mono"

[slides]
style = "academic"
```

Configuration is deliberately small. For complete visual control, use
`--template`.

### Precedence

```text
built-in defaults  →  maddy.toml  →  front matter  →  CLI arguments
```

So `--style dark` overrides `style: academic` in the document, which in turn
overrides `style` in `maddy.toml`.

---

## Mathematics

Inline mathematics uses `$...$`, display mathematics uses `$$...$$`. The
accepted language is roughly **KaTeX-compatible LaTeX mathematics** — not
arbitrary TeX.

Mathematics is *parsed structurally* into an AST and translated to native Typst
equations. There are no string substitutions, and an unsupported construct is
reported as an error rather than silently mistranslated:

```text
error: unsupported math command `\foo`
  --> paper.md:17:13

17 | We have $\foo{x}$ here.
   |          ^^^^^^^
```

> Correct failure is preferable to incorrect mathematics.

Supported today: variables, numbers, symbols, groups, operators, sub/superscripts,
fractions, roots, Greek letters, relations, arrows, delimiters, large operators,
accents, functions, text in math, font variants, matrices, cases, aligned
equations and arrays.

---

## Slide themes

Five built-in themes, all pure Typst templates:

| Theme      | For                                                  |
| ---------- | ---------------------------------------------------- |
| `academic` | seminars, lectures, research talks *(default)*       |
| `minimal`  | general professional and teaching decks              |
| `dark`     | dark rooms, technical and visually focused talks     |
| `bold`     | keynotes and pitches, one idea per slide             |
| `mono`     | software, engineering and terminal-oriented talks    |

Themes affect appearance only. They never affect parsing, slide segmentation,
math interpretation or Markdown semantics.

---

## Speed

Typical documents compile in tens of milliseconds:

| | compiler | total |
| --- | --- | --- |
| a paper with a dozen equations | 15 ms | **35 ms** |
| a five-slide deck | 6 ms | **19 ms** |
| 150 KB, 1000 equations, 155 pages | 82 ms | 455 ms |

Mathematics translates at roughly 30,000 equations per second.

Embedding the compiler is most of why. Shelling out to the `typst` executable
cost about 370 ms on every single run, nearly all of it re-enumerating the
machine's installed fonts — more time than everything else put together.

### Fonts

Only the faces Typst bundles are loaded by default, which is what keeps that
cost away and makes output identical on every machine. The machine's own fonts
are scanned only when they are actually needed:

- the `minimal` and `bold` themes, whose identity is sans-serif, which Typst
  does not bundle;
- a `font` named in `maddy.toml`, or a custom `--template`;
- text containing a character the bundled faces cannot draw — Chinese, emoji —
  which is detected from the font metadata before compiling.

That last check is why correctness does not depend on the others being right.
Typst draws an uncovered character as a blank box rather than failing, so
detecting it is what stops a document from quietly coming out wrong.

## Architecture

```text
Markdown + LaTeX-style math
             │
             ▼
        Markdown parser
             │
             ▼
         Document IR
             │
       ┌─────┴─────┐
       │           │
   document     presentation
       │           │
       └─────┬─────┘
             ▼
        Typst source
             │
             ▼
       Typst compiler
             │
             ▼
            PDF
```

Three boundaries are maintained deliberately:

| Layer           | Owns                      |
| --------------- | ------------------------- |
| Markdown        | document structure        |
| LaTeX syntax    | mathematics               |
| Typst           | the rendering backend     |

A semantic intermediate representation sits between parsing and rendering, so
the Markdown parser, the math parser and the PDF backend can each be replaced
without touching the others. Parser-specific types never leak outward, theme
decisions never leak into parsing, and Typst syntax never leaks into Markdown
semantics.

### Source layout

```text
src/
├── main.rs          cli entry point
├── cli.rs           argument parsing
├── compiler.rs      orchestration
├── config.rs        maddy.toml
├── metadata.rs      front-matter metadata
├── diagnostics.rs   errors, warnings, spans
├── markdown/        front matter + Markdown → IR
├── ir/              document, block, inline, presentation
├── math/            math parser abstraction + Typst math emitter
├── typst/           Typst emission, escaping, PDF backend
└── templates/       Typst templates (1 document, 5 slide themes)
```

---

## Development

```bash
cargo test            # unit + integration + snapshot tests
cargo clippy          # lints
cargo fmt             # formatting
```

The project is built test-first. Four levels of testing are used:

1. **Markdown parsing tests** — source → expected IR
2. **Math translation tests** — LaTeX → AST → expected Typst, over a corpus
   organized by topic (`tests/math/`)
3. **Typst snapshot tests** — fixtures → committed `.typ` snapshots, to catch
   renderer regressions
4. **End-to-end tests** — real `typst` invocation, asserting a non-empty PDF
   with the expected page count

Coverage is 92% of lines, against a target of 80%. Run it with:

```bash
cargo llvm-cov --all-features --workspace
```

Where a claim is really about what *Typst* does — escaping, symbol translation,
every theme — the test asks the real compiler rather than asserting from memory,
and skips cleanly when Typst is not installed.

---

## Non-goals

`maddy` is a Markdown compiler with excellent mathematics and good PDF
rendering. It is not a universal document conversion framework. Out of scope:
arbitrary LaTeX or TeX packages, LaTeX document classes, TeX macro programming,
Pandoc compatibility, DOCX/HTML/PPTX output, a GUI, arbitrary raw HTML, arbitrary
raw Typst inside Markdown, a plugin ecosystem, bibliography management and
Beamer compatibility.

The compiler is deliberately narrow. That narrowness is the feature.

---

## Status

v0.1 is feature complete. Implemented in the milestone order set out in
[`spec.md`](spec.md):

- [x] **1** — basic document compiler
- [x] **2** — mathematics
- [x] **3** — slides
- [x] **4** — five slide themes
- [x] **5** — richer Markdown
- [x] **6** — diagnostics
- [x] **7** — configuration
- [x] **8** — convenience (`--watch`, `--keep-typst`, `--emit both`)
- [x] **9** — embedded Typst compiler

Compilation sits behind the `PdfBackend` trait. `EmbeddedTypstBackend` is the
default and needs nothing installed; `ExternalTypstBackend`, which shells out to
a `typst` executable, remains available and is what the two-backend agreement
tests compare against.

Test coverage is 92% of lines across 438 tests.

---

## License

MIT
