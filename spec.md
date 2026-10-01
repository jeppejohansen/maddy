# Markdown + Math to PDF Compiler

## Implementation Specification — v0.1

## 1. Overview

Build a small, fast, opinionated command-line compiler written in Rust that transforms Markdown containing LaTeX-style mathematics into high-quality PDFs.

The compiler supports two first-class output modes:

```text
Markdown
   │
   ├── document ─────► PDF document
   │
   └── slides ───────► PDF presentation
```

Both modes share the same:

- Markdown parser
- mathematical syntax
- document representation
- diagnostics
- image handling
- table handling
- code-block handling
- configuration system

Only the final Typst rendering layer differs.

The basic user experience should be:

```bash
mdpdf paper.md
```

producing:

```text
paper.pdf
```

and:

```bash
mdpdf talk.md
```

where `talk.md` contains:

```yaml
---
type: slides
---
```

producing a presentation PDF.

The central architecture is:

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

---

# 2. Product philosophy

The product should feel like:

```text
gcc for Markdown documents
```

rather than:

```text
a publishing framework
```

The common case should require essentially no configuration.

A user writes:

```markdown
# Regression

Consider

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The estimator is

$$
\hat\beta = (X^\top X)^{-1}X^\top Y.
$$
```

and runs:

```bash
mdpdf regression.md
```

The compiler produces a professionally typeset PDF.

For slides:

```markdown
---
type: slides
title: Regression
author: Jane Researcher
---

# Motivation

Why should we care?

---

# Model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

---

# Results

- Estimate is positive
- Standard errors are small
```

and:

```bash
mdpdf talk.md
```

produces a widescreen presentation.

---

# 3. Core design goals

Prioritize:

1. extremely simple CLI
2. good typography by default
3. conventional Markdown syntax
4. broad KaTeX-style math compatibility
5. native Typst output
6. useful source-level errors
7. very fast compilation
8. small and understandable Rust codebase
9. no LaTeX installation
10. deterministic output
11. identical math syntax in documents and slides
12. separation between parsing, semantics, and rendering

---

# 4. Non-goals

v0.1 is not intended to provide:

- arbitrary LaTeX document compilation
- TeX package compatibility
- LaTeX document classes
- arbitrary TeX macro programming
- complete Pandoc compatibility
- DOCX output
- HTML output
- PowerPoint output
- a GUI
- arbitrary raw HTML
- arbitrary raw Typst inside Markdown
- a plugin ecosystem
- bibliography management
- complex cross-document references
- Beamer compatibility

The program is:

> a Markdown compiler with excellent mathematics and good PDF rendering.

It is not:

> a universal document conversion framework.

---

# 5. Implementation language

Rust.

Initial external components:

```text
CLI             clap
Markdown        pulldown-cmark
Math            ratex-parser
Serialization   serde
Errors          thiserror
Diagnostics     miette or equivalent
Config          TOML parser
Front matter    serde-compatible YAML parser
PDF backend     Typst CLI initially
```

Keep dependencies replaceable behind small internal interfaces where practical.

---

# 6. Command-line interface

Working binary name:

```text
mdpdf
```

The name is provisional and can be changed later.

Basic compilation:

```bash
mdpdf paper.md
```

Output:

```text
paper.pdf
```

Explicit output:

```bash
mdpdf paper.md -o result.pdf
```

or:

```bash
mdpdf paper.md --output result.pdf
```

Compile as slides:

```bash
mdpdf talk.md --slides
```

Choose slide style:

```bash
mdpdf talk.md --slides --style academic
```

Emit Typst instead of PDF:

```bash
mdpdf paper.md --emit typst
```

Emit both:

```bash
mdpdf paper.md --emit both
```

Keep intermediate Typst:

```bash
mdpdf paper.md --keep-typst
```

Target CLI:

```text
mdpdf [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Markdown source file

Options:
  -o, --output <PATH>      Output path
      --emit <FORMAT>      pdf | typst | both
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

Default:

```text
--emit pdf
```

---

# 7. Document types

Two document types exist:

```rust
pub enum DocumentType {
    Document,
    Slides,
}
```

Default:

```text
Document
```

Slides can be enabled through front matter:

```yaml
---
type: slides
---
```

or the CLI:

```bash
mdpdf foo.md --slides
```

CLI options override document metadata.

---

# 8. Configuration precedence

Configuration precedence is:

```text
built-in defaults
        ↓
configuration file
        ↓
document front matter
        ↓
CLI arguments
```

Example:

```yaml
---
type: slides
style: academic
---
```

can be overridden by:

```bash
mdpdf talk.md --style dark
```

Likewise:

```yaml
type: document
```

can be overridden by:

```bash
--slides
```

---

# 9. Front matter

The compiler recognizes YAML-style front matter only when it appears at the very beginning of the file.

Example:

```yaml
---
title: Peer Effects in High School Application
subtitle: Strategic Complementarities
author: Jane Researcher
date: October 2026
type: slides
style: academic
---
```

Initial supported fields:

```yaml
title:
subtitle:
author:
date:
type:
style:
```

Internally:

```rust
pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub document_type: DocumentType,
    pub style: Option<String>,
}
```

Unknown metadata fields should generate warnings, not errors.

Do not let YAML metadata handling become entangled with the Markdown parser.

Recommended pipeline:

```text
source
  ↓
front-matter pre-pass
  ↓
metadata + body
  ↓
Markdown parser
```

This also resolves the ambiguity between YAML delimiters and slide separators.

---

# 10. Markdown dialect

Use CommonMark-style Markdown plus explicit extensions.

Supported blocks:

- paragraphs
- headings
- unordered lists
- ordered lists
- nested lists
- blockquotes
- fenced code blocks
- indented code blocks
- horizontal rules in document mode
- tables
- images
- display mathematics

Supported inline constructs:

- plain text
- emphasis
- strong emphasis
- strikethrough
- inline code
- links
- images
- inline mathematics
- soft breaks
- hard breaks

Enable relevant `pulldown-cmark` extensions.

Math should use the Markdown parser's dedicated math events rather than trying to identify `$...$` manually.

---

# 11. Mathematics syntax

Inline mathematics:

```markdown
The coefficient is $\beta_1$.
```

Display mathematics:

```markdown
$$
\hat\beta =
(X^\top X)^{-1}X^\top Y
$$
```

The accepted mathematical language is approximately:

```text
KaTeX-compatible LaTeX mathematics
```

rather than arbitrary TeX.

The compiler is not required to understand full LaTeX documents.

---

# 12. Mathematics pipeline

Mathematics must be parsed structurally.

Never implement mathematics by string substitutions such as:

```rust
source.replace("\\frac", "frac")
```

The pipeline is:

```text
LaTeX-style math source
          ↓
      ratex-parser
          ↓
      ParseNode AST
          ↓
   normalized math layer
        or adapter
          ↓
    Typst math emitter
          ↓
       Typst math
```

The rest of the compiler should not depend directly on RaTeX internals.

---

# 13. Math parser abstraction

Hide the selected parser behind an interface.

Conceptually:

```rust
pub trait MathParser {
    type Ast;

    fn parse(
        &self,
        source: &str,
        mode: MathMode,
    ) -> Result<Self::Ast, MathError>;
}
```

Possible modes:

```rust
pub enum MathMode {
    Inline,
    Display,
}
```

This means the math parser can later be changed without modifying:

- Markdown parsing
- document IR
- slide rendering
- document rendering
- CLI logic

---

# 14. Supported mathematics

Priority-one support should include:

- variables
- numbers
- ordinary symbols
- groups
- operators
- subscripts
- superscripts
- fractions
- roots
- Greek letters
- relations
- arrows
- delimiters
- large operators
- accents
- functions
- text inside math
- mathematical font variants

Examples:

```latex
x_i

x^2

x_i^2

\frac{x}{y}

\sqrt{x}

\sqrt[n]{x}

\alpha + \beta

\sum_{i=1}^n x_i

\prod_{i=1}^n x_i

\int_0^1 f(x)\,dx

\mathbb{R}

\mathbf{x}

\hat\beta

\bar{x}

\left(\frac{x}{y}\right)

\operatorname{Var}(X)
```

Priority-two support:

- matrices
- cases
- aligned equations
- arrays
- overset
- underset
- brace constructions
- advanced delimiters

Examples:

```latex
\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
```

and:

```latex
\begin{aligned}
y &= x + 1 \\
z &= y + 2
\end{aligned}
```

---

# 15. Unsupported mathematics

Never silently render mathematically incorrect output.

If the parser recognizes a construct but the Typst emitter cannot translate it, emit an explicit diagnostic.

Example:

```text
error: LaTeX construct is not yet supported

  --> paper.md:25:5

25 | $$\begin{CD} ... \end{CD}$$
       ^^^^^^^^^^^^^^^^^^^^^^

Parsed as: CD environment
```

Correct failure is preferable to incorrect mathematics.

---

# 16. Future mathematics fallback

A later version may support:

```text
math input
   ↓
native Typst translation
   │
   ├── supported
   │      ↓
   │   Typst math
   │
   └── unsupported
          ↓
       RaTeX
          ↓
        SVG
          ↓
    embedded in Typst
```

This could provide very broad KaTeX compatibility while retaining native Typst equations for normal mathematics.

Do not implement this fallback in the first milestone.

---

# 17. Intermediate representation

Do not render Typst directly from the Markdown event stream.

Create a small semantic document IR.

Top level:

```rust
pub struct Document {
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
}
```

Blocks:

```rust
pub enum Block {
    Paragraph(Vec<Inline>),

    Heading {
        level: u8,
        content: Vec<Inline>,
    },

    BlockQuote(Vec<Block>),

    UnorderedList(Vec<ListItem>),

    OrderedList {
        start: u64,
        items: Vec<ListItem>,
    },

    CodeBlock {
        language: Option<String>,
        source: String,
    },

    Table(Table),

    Image(Image),

    Math(MathSource),

    Rule,

    SlideBreak,
}
```

Inline representation:

```rust
pub enum Inline {
    Text(String),

    Emphasis(Vec<Inline>),

    Strong(Vec<Inline>),

    Strike(Vec<Inline>),

    Code(String),

    Link {
        destination: String,
        content: Vec<Inline>,
    },

    Image(Image),

    Math(MathSource),

    SoftBreak,

    HardBreak,
}
```

Math source:

```rust
pub struct MathSource {
    pub source: String,
    pub mode: MathMode,
    pub span: SourceSpan,
}
```

---

# 18. Why the IR exists

Maintain:

```text
Markdown syntax
      ↓
document semantics
      ↓
Typst syntax
```

rather than:

```text
Markdown parser
      ↓
ad-hoc Typst strings
```

The IR provides:

- renderer independence
- cleaner tests
- source mapping
- easier diagnostics
- simpler slide support
- simpler future output formats
- ability to replace Markdown parser
- ability to replace math parser

The IR should nevertheless remain deliberately small.

Do not construct a huge abstract publishing model.

---

# 19. Presentation representation

Slides are a rendering of the same underlying blocks.

Conceptually:

```rust
pub struct Presentation {
    pub metadata: Metadata,
    pub slides: Vec<Slide>,
}

pub struct Slide {
    pub blocks: Vec<Block>,
    pub span: Option<SourceSpan>,
}
```

The compiler may either:

1. parse directly into `Presentation`, or
2. parse blocks containing `SlideBreak` and perform a segmentation pass.

The second approach is preferable because it preserves a common Markdown parser.

Pipeline:

```text
Markdown
   ↓
Block IR containing SlideBreak
   ↓
presentation segmentation
   ↓
Vec<Slide>
```

---

# 20. Slide separator

When compiling in slides mode, a standalone:

```markdown
---
```

means:

```text
new slide
```

Example:

```markdown
# Motivation

Some text.

---

# Model

Some mathematics.

---

# Results

Some results.
```

In document mode, `---` keeps its ordinary Markdown meaning as a horizontal/thematic rule.

The parser therefore needs to know document type before interpreting slide separators.

Front matter is removed before this stage.

---

# 21. Slide titles

A level-one heading at the beginning of a slide is interpreted as its title.

Example:

```markdown
# Identification

We require

$$
E[m(\psi,\theta)\mid\psi] = 0.
$$
```

renders with:

```text
Identification
```

as the slide title.

A slide does not require a title.

For example:

```markdown
$$
Y = \alpha_i + \psi_j + m(\alpha_i,\psi_j) + \varepsilon
$$
```

is valid.

Lower-level headings remain body content:

```markdown
# Results

## Main result

...

## Robustness

...
```

Multiple level-one headings on the same slide should produce a warning.

---

# 22. Title slide

When slide metadata contains:

```yaml
title:
```

the renderer automatically generates a title slide.

Example:

```yaml
---
type: slides
title: Peer Effects in High School Application
subtitle: Strategic Complementarities in Educational Choice
author: Jane Researcher
date: October 2026
style: academic
---
```

creates a title slide containing those fields.

The first Markdown slide follows it.

A future option may support:

```yaml
title-slide: false
```

but this is not necessary for the first release.

---

# 23. Slide dimensions

Default slide aspect ratio:

```text
16:9
```

The presentation renderer should produce a standard widescreen PDF.

Future versions may support:

```yaml
aspect-ratio: "4:3"
```

but only 16:9 is required initially.

---

# 24. Slide styles

The compiler ships with five built-in slide themes:

```text
academic
minimal
dark
bold
mono
```

Default:

```text
academic
```

A style is selected using:

```yaml
---
type: slides
style: academic
---
```

or:

```bash
mdpdf talk.md --slides --style academic
```

Styles affect appearance only.

They must not affect:

- parsing
- slide segmentation
- math interpretation
- Markdown semantics

---

# 25. Academic slide style

Purpose:

- economics seminars
- scientific presentations
- lectures
- research conferences
- technical talks

Characteristics:

- light background
- restrained typography
- generous whitespace
- excellent equation rendering
- clear tables
- unobtrusive slide numbers
- minimal decoration
- designed to tolerate moderately dense content

This is the default slide theme.

---

# 26. Minimal slide style

Purpose:

- general professional presentations
- internal talks
- teaching
- simple narrative decks

Characteristics:

- modern sans-serif typography
- large headings
- spacious margins
- minimal chrome
- limited use of accents
- strong readability
- relatively low visual density

---

# 27. Dark slide style

Purpose:

- talks in dark rooms
- technical presentations
- visually focused decks

Characteristics:

- intentionally designed dark background
- high-contrast text
- muted secondary typography
- restrained accent colors
- excellent code-block rendering
- highly legible equations

Do not implement this as a naïve inversion of another theme.

---

# 28. Bold slide style

Purpose:

- keynote-style presentations
- pitches
- talks where individual ideas should dominate

Characteristics:

- oversized headings
- larger body text
- strong typographic scale
- accent blocks
- strong section breaks
- lower information density
- visually assertive without becoming decorative clutter

---

# 29. Mono slide style

Purpose:

- software talks
- engineering talks
- terminal-oriented presentations
- technical demos

Characteristics:

- monospaced or semi-monospaced identity
- visible grid structure
- strong code presentation
- restrained decoration
- technically oriented aesthetic
- professional rather than retro gimmick

---

# 30. Style architecture

Themes should be Typst templates.

Suggested layout:

```text
templates/
├── document/
│   └── default.typ
│
└── slides/
    ├── academic.typ
    ├── minimal.typ
    ├── dark.typ
    ├── bold.typ
    └── mono.typ
```

The Rust renderer should never contain theme-specific formatting logic such as:

```rust
if style == "dark" {
    ...
}
```

except for selecting the template.

Instead:

```text
Presentation IR
      ↓
semantic Typst output
      ↓
selected theme
      ↓
PDF
```

---

# 31. Theme contract

All slide themes should implement the same conceptual interface.

Every theme must define rendering for:

- presentation metadata
- title slide
- ordinary slide
- slide title
- body
- headings
- lists
- equations
- images
- tables
- code
- quotes
- footer
- slide number

Custom templates can later implement the same contract.

---

# 32. Document styles

v0.1 only needs one excellent default document style.

It should provide:

- A4 paper
- good serif body font
- strong math typography
- sensible margins
- heading hierarchy
- comfortable line spacing
- code formatting
- good tables
- page numbering where appropriate

Do not create five document themes just because slides have five themes.

---

# 33. Typst emission

Renderer interface:

```rust
pub fn emit_document(
    document: &Document,
    options: &RenderOptions,
) -> Result<String, EmitError>;
```

Presentation equivalent:

```rust
pub fn emit_presentation(
    presentation: &Presentation,
    options: &RenderOptions,
) -> Result<String, EmitError>;
```

Generated Typst should be human-readable.

Do not minify generated source.

This is useful for:

- debugging
- tests
- user inspection
- diagnosing renderer bugs

---

# 34. Escaping

Ordinary Markdown text must not accidentally become Typst syntax.

Centralize escaping.

For example:

```rust
fn escape_typst_text(input: &str) -> String;
```

Different contexts need different escaping rules:

```text
normal text
math
code/raw
URLs
```

Do not scatter escaping logic throughout rendering functions.

---

# 35. Images

Support local images.

Markdown:

```markdown
![Results](figures/results.png)
```

Initial formats should include at least:

- PNG
- JPEG
- SVG
- PDF where Typst can directly handle it appropriately

Paths are resolved relative to the source Markdown file.

Do not initially download remote images.

For:

```markdown
![](https://example.com/plot.png)
```

emit an explicit unsupported-feature diagnostic.

---

# 36. Images in slides

Images use ordinary Markdown syntax.

The slide renderer should:

- preserve aspect ratio
- constrain images to the available slide area
- avoid overflowing slide boundaries
- use presentation-appropriate spacing

Do not initially invent custom image sizing syntax unless it becomes necessary.

---

# 37. Tables

Markdown tables should compile in both modes.

Example:

```markdown
| Model | Estimate |
|---|---:|
| OLS | 0.42 |
| IV | 0.51 |
```

Documents use document-appropriate spacing.

Slides use presentation-appropriate spacing and larger text.

Do not silently shrink a table to unreadable font sizes.

If reliable overflow detection becomes available, prefer a warning.

---

# 38. Code blocks

Markdown:

````markdown
```rust
fn main() {
    println!("hello");
}
```
````

should become Typst code/raw markup.

Pass the language identifier through when available.

Unknown languages still render as plain code.

Syntax highlighting is Typst's responsibility.

The `mono` slide theme may visually emphasize code more than other themes.

---

# 39. Blockquotes

Markdown:

```markdown
> Economic models are abstractions.
```

should map to semantic quotation rendering.

Do not hard-code quotation styling in Markdown parsing.

---

# 40. Raw HTML

Raw HTML is unsupported.

Example:

```html
<div>hello</div>
```

should produce:

```text
warning: raw HTML is not supported
  --> paper.md:42:1

42 | <div>hello</div>
   | ^^^^^^^^^^^^^^^^
```

Under:

```text
--strict
```

warnings become errors.

Never silently reinterpret HTML.

---

# 41. Raw Typst

Do not allow arbitrary Typst in Markdown for v0.1.

This keeps language boundaries clear:

```text
Markdown        document structure
LaTeX syntax    math
Typst           compiler backend
```

A future explicit Typst block extension can be considered later.

---

# 42. PDF compilation

The first implementation should invoke the installed Typst executable.

Pipeline:

```text
Markdown
    ↓
your compiler
    ↓
temporary .typ
    ↓
typst compile
    ↓
PDF
```

Run directly:

```bash
typst compile generated.typ output.pdf
```

Do not run through:

```bash
sh -c
```

Use Rust's direct process API.

If Typst is unavailable:

```text
error: Typst executable was not found

Install Typst or run:

    mdpdf --emit typst document.md
```

---

# 43. Embedded Typst compiler

Embedding Typst in the Rust binary is a later milestone.

Typst exposes a Rust compilation API, so this should remain architecturally possible.

Define an abstraction:

```rust
pub trait PdfBackend {
    fn compile(
        &self,
        typst_source: &str,
        context: &CompileContext,
    ) -> Result<Vec<u8>, PdfError>;
}
```

Initial backend:

```text
ExternalTypstBackend
```

Future backend:

```text
EmbeddedTypstBackend
```

No upstream code should care which implementation is selected.

---

# 44. Diagnostics

Diagnostics are a major feature.

Whenever possible, errors should point back to Markdown rather than generated Typst.

Example:

```text
error: unsupported math command `\foo`
  --> paper.md:17:13

17 | We have $\foo{x}$ here.
   |          ^^^^^^^
```

Diagnostic levels:

```text
error
warning
note
```

Main error categories:

```rust
MarkdownError
FrontMatterError
MathParseError
MathEmitError
UnsupportedFeature
IoError
ConfigError
TypstCompileError
InternalError
```

Avoid generic errors like:

```text
Compilation failed
```

when a useful source position is available.

---

# 45. Source spans

Preserve source locations from the beginning.

Use byte ranges into the UTF-8 input:

```rust
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
```

Line and column numbers can be calculated when displaying diagnostics.

For math:

```text
Markdown source
      ↓
$ mathematical substring $
      ↓
math parser offset
      ↓
translate offset back into Markdown source
```

This allows parser errors inside equations to point to the original file.

---

# 46. Generated Typst errors

Most user-facing syntax errors should be caught before Typst compilation.

A Typst compile failure therefore usually means either:

1. a bug in the emitter, or
2. an invalid user template.

Display:

```text
error: generated Typst failed to compile

Caused by:
    ...

Run with --keep-typst to inspect generated source.
```

Do not dump pages of subprocess output by default.

Verbose mode may provide more detail.

---

# 47. Configuration

Optional project configuration:

```text
mdpdf.toml
```

Example:

```toml
[document]
paper = "a4"
font = "Libertinus Serif"
font_size = "11pt"

[page]
margin = "25mm"

[headings]
numbered = false

[code]
font = "JetBrains Mono"
```

Keep configuration deliberately small in v0.1.

Avoid turning every Typst property into a configuration option.

Users needing complete visual control should eventually use a custom template.

---

# 48. Custom templates

Support:

```bash
mdpdf paper.md --template custom.typ
```

and:

```bash
mdpdf talk.md --slides --template presentation.typ
```

Built-in and external templates should obey stable renderer contracts.

The Markdown compiler should not need modifications when a user changes visual appearance.

---

# 49. Suggested source layout

```text
src/
├── main.rs
├── cli.rs
├── compiler.rs
├── config.rs
├── metadata.rs
├── diagnostics.rs
│
├── markdown/
│   ├── mod.rs
│   ├── parser.rs
│   └── frontmatter.rs
│
├── ir/
│   ├── mod.rs
│   ├── document.rs
│   ├── block.rs
│   ├── inline.rs
│   └── presentation.rs
│
├── math/
│   ├── mod.rs
│   ├── parser.rs
│   ├── ratex.rs
│   ├── emit.rs
│   └── error.rs
│
├── typst/
│   ├── mod.rs
│   ├── document.rs
│   ├── presentation.rs
│   ├── escape.rs
│   └── backend.rs
│
└── templates/
    ├── document/
    │   └── default.typ
    └── slides/
        ├── academic.typ
        ├── minimal.typ
        ├── dark.typ
        ├── bold.typ
        └── mono.typ
```

Tests:

```text
tests/
├── markdown/
├── math/
├── documents/
├── slides/
├── snapshots/
└── integration/
```

---

# 50. Compiler orchestration

Conceptually:

```rust
pub fn compile(
    input: &Path,
    options: CompileOptions,
) -> Result<CompileResult, CompileError> {

    let source = read_source(input)?;

    let (metadata, body) =
        markdown::parse_frontmatter(&source)?;

    let effective_options =
        resolve_options(metadata, options)?;

    let document =
        markdown::parse(body, effective_options.document_type)?;

    let typst = match effective_options.document_type {
        DocumentType::Document => {
            typst::emit_document(&document, &effective_options)?
        }

        DocumentType::Slides => {
            let presentation =
                Presentation::from_document(document)?;

            typst::emit_presentation(
                &presentation,
                &effective_options,
            )?
        }
    };

    emit_outputs(typst, effective_options)
}
```

The orchestration layer should remain boring.

That is a positive design goal.

---

# 51. Output handling

For:

```bash
mdpdf paper.md
```

produce:

```text
paper.pdf
```

For:

```bash
mdpdf paper.md --emit typst
```

produce:

```text
paper.typ
```

For:

```bash
mdpdf paper.md --emit both
```

produce:

```text
paper.typ
paper.pdf
```

For:

```bash
mdpdf paper.md --keep-typst
```

preserve intermediate Typst even when PDF is the selected output.

---

# 52. Logging

Default successful compilation:

```text
Compiled paper.md → paper.pdf
```

Slides:

```text
Compiled talk.md → talk.pdf
```

Quiet:

```bash
mdpdf -q paper.md
```

prints nothing unless an error occurs.

Verbose:

```bash
mdpdf -v paper.md
```

may show:

```text
Reading metadata...
Parsing Markdown...
Parsing 17 math expressions...
Generating Typst...
Running Typst...
Compiled paper.md → paper.pdf
```

---

# 53. Exit codes

Use predictable Unix behavior.

Suggested:

```text
0    success
1    document compilation error
2    invalid CLI/configuration
3    I/O or environment error
```

Exact non-zero numbers matter less than consistency.

---

# 54. Security

Do not execute commands contained in Markdown.

Do not evaluate Markdown as Typst.

Do not download network resources in v0.1.

Do not construct shell command strings.

Use:

```rust
Command::new("typst")
```

rather than:

```rust
Command::new("sh").arg("-c")
```

If parsing untrusted math, keep the math parser dependency updated.

---

# 55. Testing strategy

Use four primary levels of testing.

## Markdown parsing tests

Input:

```markdown
This is **bold**.
```

Expected IR conceptually:

```text
Paragraph
 ├─ Text("This is ")
 ├─ Strong
 │   └─ Text("bold")
 └─ Text(".")
```

---

## Math translation tests

Input:

```latex
\frac{x_i^2}{\sqrt{n}}
```

Test:

```text
LaTeX
  ↓
RaTeX parser
  ↓
AST
  ↓
Typst emitter
  ↓
valid Typst
```

Compare with expected output.

---

## Typst snapshot tests

Fixture:

```text
tests/fixtures/basic.md
```

should produce:

```text
tests/snapshots/basic.typ
```

Use snapshots to catch renderer regressions.

---

## End-to-end tests

Run:

```text
Markdown
  ↓
compiler
  ↓
Typst
  ↓
real Typst compiler
  ↓
PDF
```

Verify at least:

- process succeeds
- output PDF exists
- PDF is non-empty
- expected page count where appropriate

Do not initially compare raw PDF bytes.

---

# 56. Math compatibility corpus

Create a dedicated math corpus.

Suggested:

```text
tests/math/
├── basic.txt
├── fractions.txt
├── scripts.txt
├── operators.txt
├── greek.txt
├── fonts.txt
├── delimiters.txt
├── accents.txt
├── matrices.txt
├── aligned.txt
├── calculus.txt
├── sets.txt
├── probability.txt
└── statistics.txt
```

Representative expressions:

```latex
E[Y \mid X]

\mathbb E[Y \mid X]

\operatorname{Var}(X)

X^\top X

(X^\top X)^{-1}

\hat\beta

\frac{\partial f(x)}{\partial x}

\sum_{i=1}^{N}

\int_{-\infty}^{\infty}

\mathbf{x}'\beta

\Pr(Y = 1 \mid X)

\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
```

Every supported expression should:

1. parse
2. translate
3. produce valid Typst
4. compile successfully

---

# 57. Document acceptance test

The following source should compile:

```markdown
# Regression

Consider the linear model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The OLS estimator is

$$
\hat\beta =
\left(
\sum_{i=1}^{n} X_i X_i^\top
\right)^{-1}
\sum_{i=1}^{n} X_i Y_i.
$$

Under

$$
\mathbb E[\varepsilon_i \mid X_i] = 0,
$$

the estimator is **consistent**.

## Notes

- $X_i \in \mathbb R^K$.
- $\beta$ is a $K \times 1$ vector.
- Standard errors can be *heteroskedasticity robust*.

```rust
fn estimate(x: Matrix, y: Vector) -> Vector {
    solve(x.t() * x, x.t() * y)
}
```
```

with:

```bash
mdpdf regression.md
```

and produce a professional document PDF without manual intervention.

---

# 58. Presentation acceptance test

The following source should compile:

```markdown
---
type: slides
title: Peer Effects
subtitle: High School Application
author: Jane Researcher
style: academic
---

# Motivation

Students make educational decisions in social environments.

- Friends share information
- Choices may be strategic complements
- Policy can change peer composition

---

# Model

Student $i$ chooses

$$
a_i \in \{0,1\}.
$$

Application probability is

$$
p_i =
\operatorname{logit}^{-1}
\left(
X_i^\top\beta +
\lambda \bar p_{-i}
\right).
$$

---

# Policy experiment

Suppose a GPA cutoff is introduced.

The direct effect changes eligibility.

The equilibrium effect additionally changes peer behavior.

---

# Results

| Model | Effect |
|---|---:|
| No peers | 3.2% |
| Peer equilibrium | 4.0% |
```

Running:

```bash
mdpdf talk.md
```

must produce:

- a 16:9 PDF
- an automatically generated title slide
- four content slides
- correctly rendered mathematics
- correctly rendered table
- slide numbers
- consistent typography

Changing:

```yaml
style: academic
```

to:

```yaml
style: dark
```

must alter presentation appearance without altering parsing or semantics.

---

# 59. Recommended implementation order

Do not build everything simultaneously.

## Milestone 1 — basic document compiler

Implement:

1. CLI
2. read Markdown file
3. paragraphs
4. headings
5. emphasis
6. strong
7. lists
8. simple IR
9. Typst text emitter
10. external `typst compile`

At this point:

```bash
mdpdf simple.md
```

should generate a PDF.

---

## Milestone 2 — mathematics

Implement:

1. inline math events
2. display math events
3. RaTeX parser adapter
4. fundamental math-node conversion
5. fractions
6. scripts
7. roots
8. symbols
9. operators
10. delimiters
11. matrices
12. math compatibility tests

At this point the project already has its main distinctive value.

---

## Milestone 3 — slides

Implement:

1. front matter
2. `type: slides`
3. `--slides`
4. `SlideBreak`
5. slide segmentation
6. 16:9 Typst presentation template
7. title slides
8. slide titles
9. slide numbers
10. `academic` theme

At this point:

```bash
mdpdf talk.md
```

should produce a usable research presentation.

---

## Milestone 4 — five slide themes

Implement:

```text
academic
minimal
dark
bold
mono
```

Do not add theme-specific parsing.

All differences live in Typst templates.

---

## Milestone 5 — richer Markdown

Add:

- links
- blockquotes
- tables
- images
- code blocks
- strikethrough
- better nested lists

---

## Milestone 6 — diagnostics

Add:

- full source spans
- pretty Markdown errors
- math parser span mapping
- unsupported-feature diagnostics
- `--strict`

---

## Milestone 7 — configuration

Add:

- `mdpdf.toml`
- custom templates
- CLI/config/front-matter precedence

---

## Milestone 8 — convenience

Add:

```text
--watch
--keep-typst
--emit both
```

and improved logging.

---

## Milestone 9 — embedded Typst

Replace the subprocess implementation optionally with direct Typst library compilation.

This should not require architectural changes.

---

# 60. First coding-session stopping point

The first implementation session should stop once all of the following work:

```markdown
# Heading

Some **bold** and *italic* text.

- First
- Second

Inline mathematics: $x_i^2$.

$$
\frac{x}{y}
$$
```

and:

```bash
mdpdf test.md
```

produces:

```text
test.pdf
```

Do not begin configuration, themes, advanced tables, watch mode, or embedded Typst before this works.

---

# 61. First slides stopping point

The first slide implementation should stop once:

```markdown
---
type: slides
title: Example Presentation
author: Jane Researcher
---

# Question

Why does this matter?

---

# Model

$$
Y_i = X_i^\top \beta + \varepsilon_i
$$

---

# Result

The coefficient is **positive**.
```

successfully produces a three-content-slide presentation plus title slide.

Only the `academic` theme needs to exist at that stage.

---

# 62. Future slide layouts

Do not block v0.1 on layout syntax.

However, preserve room for a small later extension supporting common presentation layouts such as:

```text
single column
two columns
image left / text right
text left / image right
full-slide image
```

Any future syntax should remain small and explicit rather than evolving into arbitrary Typst embedded in Markdown.

Do not design this until ordinary slide generation is working.

---

# 63. Future features

Potential later features include:

- citations and bibliography
- equation numbering
- cross-references
- figure captions
- table captions
- section slides
- speaker notes
- slide fragments/reveals
- two-column layouts
- theme customization
- embedded Typst compiler
- HTML output
- standalone SVG math fallback
- additional document styles
- custom Markdown extensions

None of these should delay a usable v0.1.

---

# 64. Architectural invariants

These boundaries should remain intact throughout the project.

## Document pipeline

```text
Markdown syntax
      ↓
Document semantics / IR
      ↓
Typst semantics
      ↓
PDF
```

## Mathematics pipeline

```text
LaTeX math syntax
      ↓
math parser / AST
      ↓
Typst math representation
      ↓
PDF
```

## Presentation pipeline

```text
Document IR
     ↓
slide segmentation
     ↓
Presentation IR
     ↓
theme-independent renderer
     ↓
Typst slide theme
     ↓
PDF
```

Do not let parser-specific structures leak into unrelated layers.

Do not let theme-specific decisions leak into parsing.

Do not let Typst syntax leak into Markdown semantics.

---

# 65. Definition of success

A successful v0.1 lets a user install one small CLI and write either:

```bash
mdpdf paper.md
```

or:

```bash
mdpdf talk.md
```

using ordinary Markdown plus familiar LaTeX mathematics.

Documents should look like professional technical documents.

Slides should look like professional presentations.

The user should normally not need to know that Typst is the rendering backend.
# Markdown + Math to PDF Compiler

## Implementation Specification — v0.1

## 1. Overview

Build a small, fast, opinionated command-line compiler written in Rust that transforms Markdown containing LaTeX-style mathematics into high-quality PDFs.

The compiler supports two first-class output modes:

```text
Markdown
   │
   ├── document ─────► PDF document
   │
   └── slides ───────► PDF presentation
```

Both modes share the same:

- Markdown parser
- mathematical syntax
- document representation
- diagnostics
- image handling
- table handling
- code-block handling
- configuration system

Only the final Typst rendering layer differs.

The basic user experience should be:

```bash
mdpdf paper.md
```

producing:

```text
paper.pdf
```

and:

```bash
mdpdf talk.md
```

where `talk.md` contains:

```yaml
---
type: slides
---
```

producing a presentation PDF.

The central architecture is:

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

---

# 2. Product philosophy

The product should feel like:

```text
gcc for Markdown documents
```

rather than:

```text
a publishing framework
```

The common case should require essentially no configuration.

A user writes:

```markdown
# Regression

Consider

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The estimator is

$$
\hat\beta = (X^\top X)^{-1}X^\top Y.
$$
```

and runs:

```bash
mdpdf regression.md
```

The compiler produces a professionally typeset PDF.

For slides:

```markdown
---
type: slides
title: Regression
author: Jane Researcher
---

# Motivation

Why should we care?

---

# Model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

---

# Results

- Estimate is positive
- Standard errors are small
```

and:

```bash
mdpdf talk.md
```

produces a widescreen presentation.

---

# 3. Core design goals

Prioritize:

1. extremely simple CLI
2. good typography by default
3. conventional Markdown syntax
4. broad KaTeX-style math compatibility
5. native Typst output
6. useful source-level errors
7. very fast compilation
8. small and understandable Rust codebase
9. no LaTeX installation
10. deterministic output
11. identical math syntax in documents and slides
12. separation between parsing, semantics, and rendering

---

# 4. Non-goals

v0.1 is not intended to provide:

- arbitrary LaTeX document compilation
- TeX package compatibility
- LaTeX document classes
- arbitrary TeX macro programming
- complete Pandoc compatibility
- DOCX output
- HTML output
- PowerPoint output
- a GUI
- arbitrary raw HTML
- arbitrary raw Typst inside Markdown
- a plugin ecosystem
- bibliography management
- complex cross-document references
- Beamer compatibility

The program is:

> a Markdown compiler with excellent mathematics and good PDF rendering.

It is not:

> a universal document conversion framework.

---

# 5. Implementation language

Rust.

Initial external components:

```text
CLI             clap
Markdown        pulldown-cmark
Math            ratex-parser
Serialization   serde
Errors          thiserror
Diagnostics     miette or equivalent
Config          TOML parser
Front matter    serde-compatible YAML parser
PDF backend     Typst CLI initially
```

Keep dependencies replaceable behind small internal interfaces where practical.

---

# 6. Command-line interface

Working binary name:

```text
mdpdf
```

The name is provisional and can be changed later.

Basic compilation:

```bash
mdpdf paper.md
```

Output:

```text
paper.pdf
```

Explicit output:

```bash
mdpdf paper.md -o result.pdf
```

or:

```bash
mdpdf paper.md --output result.pdf
```

Compile as slides:

```bash
mdpdf talk.md --slides
```

Choose slide style:

```bash
mdpdf talk.md --slides --style academic
```

Emit Typst instead of PDF:

```bash
mdpdf paper.md --emit typst
```

Emit both:

```bash
mdpdf paper.md --emit both
```

Keep intermediate Typst:

```bash
mdpdf paper.md --keep-typst
```

Target CLI:

```text
mdpdf [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Markdown source file

Options:
  -o, --output <PATH>      Output path
      --emit <FORMAT>      pdf | typst | both
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

Default:

```text
--emit pdf
```

---

# 7. Document types

Two document types exist:

```rust
pub enum DocumentType {
    Document,
    Slides,
}
```

Default:

```text
Document
```

Slides can be enabled through front matter:

```yaml
---
type: slides
---
```

or the CLI:

```bash
mdpdf foo.md --slides
```

CLI options override document metadata.

---

# 8. Configuration precedence

Configuration precedence is:

```text
built-in defaults
        ↓
configuration file
        ↓
document front matter
        ↓
CLI arguments
```

Example:

```yaml
---
type: slides
style: academic
---
```

can be overridden by:

```bash
mdpdf talk.md --style dark
```

Likewise:

```yaml
type: document
```

can be overridden by:

```bash
--slides
```

---

# 9. Front matter

The compiler recognizes YAML-style front matter only when it appears at the very beginning of the file.

Example:

```yaml
---
title: Peer Effects in High School Application
subtitle: Strategic Complementarities
author: Jane Researcher
date: October 2026
type: slides
style: academic
---
```

Initial supported fields:

```yaml
title:
subtitle:
author:
date:
type:
style:
```

Internally:

```rust
pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub document_type: DocumentType,
    pub style: Option<String>,
}
```

Unknown metadata fields should generate warnings, not errors.

Do not let YAML metadata handling become entangled with the Markdown parser.

Recommended pipeline:

```text
source
  ↓
front-matter pre-pass
  ↓
metadata + body
  ↓
Markdown parser
```

This also resolves the ambiguity between YAML delimiters and slide separators.

---

# 10. Markdown dialect

Use CommonMark-style Markdown plus explicit extensions.

Supported blocks:

- paragraphs
- headings
- unordered lists
- ordered lists
- nested lists
- blockquotes
- fenced code blocks
- indented code blocks
- horizontal rules in document mode
- tables
- images
- display mathematics

Supported inline constructs:

- plain text
- emphasis
- strong emphasis
- strikethrough
- inline code
- links
- images
- inline mathematics
- soft breaks
- hard breaks

Enable relevant `pulldown-cmark` extensions.

Math should use the Markdown parser's dedicated math events rather than trying to identify `$...$` manually.

---

# 11. Mathematics syntax

Inline mathematics:

```markdown
The coefficient is $\beta_1$.
```

Display mathematics:

```markdown
$$
\hat\beta =
(X^\top X)^{-1}X^\top Y
$$
```

The accepted mathematical language is approximately:

```text
KaTeX-compatible LaTeX mathematics
```

rather than arbitrary TeX.

The compiler is not required to understand full LaTeX documents.

---

# 12. Mathematics pipeline

Mathematics must be parsed structurally.

Never implement mathematics by string substitutions such as:

```rust
source.replace("\\frac", "frac")
```

The pipeline is:

```text
LaTeX-style math source
          ↓
      ratex-parser
          ↓
      ParseNode AST
          ↓
   normalized math layer
        or adapter
          ↓
    Typst math emitter
          ↓
       Typst math
```

The rest of the compiler should not depend directly on RaTeX internals.

---

# 13. Math parser abstraction

Hide the selected parser behind an interface.

Conceptually:

```rust
pub trait MathParser {
    type Ast;

    fn parse(
        &self,
        source: &str,
        mode: MathMode,
    ) -> Result<Self::Ast, MathError>;
}
```

Possible modes:

```rust
pub enum MathMode {
    Inline,
    Display,
}
```

This means the math parser can later be changed without modifying:

- Markdown parsing
- document IR
- slide rendering
- document rendering
- CLI logic

---

# 14. Supported mathematics

Priority-one support should include:

- variables
- numbers
- ordinary symbols
- groups
- operators
- subscripts
- superscripts
- fractions
- roots
- Greek letters
- relations
- arrows
- delimiters
- large operators
- accents
- functions
- text inside math
- mathematical font variants

Examples:

```latex
x_i

x^2

x_i^2

\frac{x}{y}

\sqrt{x}

\sqrt[n]{x}

\alpha + \beta

\sum_{i=1}^n x_i

\prod_{i=1}^n x_i

\int_0^1 f(x)\,dx

\mathbb{R}

\mathbf{x}

\hat\beta

\bar{x}

\left(\frac{x}{y}\right)

\operatorname{Var}(X)
```

Priority-two support:

- matrices
- cases
- aligned equations
- arrays
- overset
- underset
- brace constructions
- advanced delimiters

Examples:

```latex
\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
```

and:

```latex
\begin{aligned}
y &= x + 1 \\
z &= y + 2
\end{aligned}
```

---

# 15. Unsupported mathematics

Never silently render mathematically incorrect output.

If the parser recognizes a construct but the Typst emitter cannot translate it, emit an explicit diagnostic.

Example:

```text
error: LaTeX construct is not yet supported

  --> paper.md:25:5

25 | $$\begin{CD} ... \end{CD}$$
       ^^^^^^^^^^^^^^^^^^^^^^

Parsed as: CD environment
```

Correct failure is preferable to incorrect mathematics.

---

# 16. Future mathematics fallback

A later version may support:

```text
math input
   ↓
native Typst translation
   │
   ├── supported
   │      ↓
   │   Typst math
   │
   └── unsupported
          ↓
       RaTeX
          ↓
        SVG
          ↓
    embedded in Typst
```

This could provide very broad KaTeX compatibility while retaining native Typst equations for normal mathematics.

Do not implement this fallback in the first milestone.

---

# 17. Intermediate representation

Do not render Typst directly from the Markdown event stream.

Create a small semantic document IR.

Top level:

```rust
pub struct Document {
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
}
```

Blocks:

```rust
pub enum Block {
    Paragraph(Vec<Inline>),

    Heading {
        level: u8,
        content: Vec<Inline>,
    },

    BlockQuote(Vec<Block>),

    UnorderedList(Vec<ListItem>),

    OrderedList {
        start: u64,
        items: Vec<ListItem>,
    },

    CodeBlock {
        language: Option<String>,
        source: String,
    },

    Table(Table),

    Image(Image),

    Math(MathSource),

    Rule,

    SlideBreak,
}
```

Inline representation:

```rust
pub enum Inline {
    Text(String),

    Emphasis(Vec<Inline>),

    Strong(Vec<Inline>),

    Strike(Vec<Inline>),

    Code(String),

    Link {
        destination: String,
        content: Vec<Inline>,
    },

    Image(Image),

    Math(MathSource),

    SoftBreak,

    HardBreak,
}
```

Math source:

```rust
pub struct MathSource {
    pub source: String,
    pub mode: MathMode,
    pub span: SourceSpan,
}
```

---

# 18. Why the IR exists

Maintain:

```text
Markdown syntax
      ↓
document semantics
      ↓
Typst syntax
```

rather than:

```text
Markdown parser
      ↓
ad-hoc Typst strings
```

The IR provides:

- renderer independence
- cleaner tests
- source mapping
- easier diagnostics
- simpler slide support
- simpler future output formats
- ability to replace Markdown parser
- ability to replace math parser

The IR should nevertheless remain deliberately small.

Do not construct a huge abstract publishing model.

---

# 19. Presentation representation

Slides are a rendering of the same underlying blocks.

Conceptually:

```rust
pub struct Presentation {
    pub metadata: Metadata,
    pub slides: Vec<Slide>,
}

pub struct Slide {
    pub blocks: Vec<Block>,
    pub span: Option<SourceSpan>,
}
```

The compiler may either:

1. parse directly into `Presentation`, or
2. parse blocks containing `SlideBreak` and perform a segmentation pass.

The second approach is preferable because it preserves a common Markdown parser.

Pipeline:

```text
Markdown
   ↓
Block IR containing SlideBreak
   ↓
presentation segmentation
   ↓
Vec<Slide>
```

---

# 20. Slide separator

When compiling in slides mode, a standalone:

```markdown
---
```

means:

```text
new slide
```

Example:

```markdown
# Motivation

Some text.

---

# Model

Some mathematics.

---

# Results

Some results.
```

In document mode, `---` keeps its ordinary Markdown meaning as a horizontal/thematic rule.

The parser therefore needs to know document type before interpreting slide separators.

Front matter is removed before this stage.

---

# 21. Slide titles

A level-one heading at the beginning of a slide is interpreted as its title.

Example:

```markdown
# Identification

We require

$$
E[m(\psi,\theta)\mid\psi] = 0.
$$
```

renders with:

```text
Identification
```

as the slide title.

A slide does not require a title.

For example:

```markdown
$$
Y = \alpha_i + \psi_j + m(\alpha_i,\psi_j) + \varepsilon
$$
```

is valid.

Lower-level headings remain body content:

```markdown
# Results

## Main result

...

## Robustness

...
```

Multiple level-one headings on the same slide should produce a warning.

---

# 22. Title slide

When slide metadata contains:

```yaml
title:
```

the renderer automatically generates a title slide.

Example:

```yaml
---
type: slides
title: Peer Effects in High School Application
subtitle: Strategic Complementarities in Educational Choice
author: Jane Researcher
date: October 2026
style: academic
---
```

creates a title slide containing those fields.

The first Markdown slide follows it.

A future option may support:

```yaml
title-slide: false
```

but this is not necessary for the first release.

---

# 23. Slide dimensions

Default slide aspect ratio:

```text
16:9
```

The presentation renderer should produce a standard widescreen PDF.

Future versions may support:

```yaml
aspect-ratio: "4:3"
```

but only 16:9 is required initially.

---

# 24. Slide styles

The compiler ships with five built-in slide themes:

```text
academic
minimal
dark
bold
mono
```

Default:

```text
academic
```

A style is selected using:

```yaml
---
type: slides
style: academic
---
```

or:

```bash
mdpdf talk.md --slides --style academic
```

Styles affect appearance only.

They must not affect:

- parsing
- slide segmentation
- math interpretation
- Markdown semantics

---

# 25. Academic slide style

Purpose:

- economics seminars
- scientific presentations
- lectures
- research conferences
- technical talks

Characteristics:

- light background
- restrained typography
- generous whitespace
- excellent equation rendering
- clear tables
- unobtrusive slide numbers
- minimal decoration
- designed to tolerate moderately dense content

This is the default slide theme.

---

# 26. Minimal slide style

Purpose:

- general professional presentations
- internal talks
- teaching
- simple narrative decks

Characteristics:

- modern sans-serif typography
- large headings
- spacious margins
- minimal chrome
- limited use of accents
- strong readability
- relatively low visual density

---

# 27. Dark slide style

Purpose:

- talks in dark rooms
- technical presentations
- visually focused decks

Characteristics:

- intentionally designed dark background
- high-contrast text
- muted secondary typography
- restrained accent colors
- excellent code-block rendering
- highly legible equations

Do not implement this as a naïve inversion of another theme.

---

# 28. Bold slide style

Purpose:

- keynote-style presentations
- pitches
- talks where individual ideas should dominate

Characteristics:

- oversized headings
- larger body text
- strong typographic scale
- accent blocks
- strong section breaks
- lower information density
- visually assertive without becoming decorative clutter

---

# 29. Mono slide style

Purpose:

- software talks
- engineering talks
- terminal-oriented presentations
- technical demos

Characteristics:

- monospaced or semi-monospaced identity
- visible grid structure
- strong code presentation
- restrained decoration
- technically oriented aesthetic
- professional rather than retro gimmick

---

# 30. Style architecture

Themes should be Typst templates.

Suggested layout:

```text
templates/
├── document/
│   └── default.typ
│
└── slides/
    ├── academic.typ
    ├── minimal.typ
    ├── dark.typ
    ├── bold.typ
    └── mono.typ
```

The Rust renderer should never contain theme-specific formatting logic such as:

```rust
if style == "dark" {
    ...
}
```

except for selecting the template.

Instead:

```text
Presentation IR
      ↓
semantic Typst output
      ↓
selected theme
      ↓
PDF
```

---

# 31. Theme contract

All slide themes should implement the same conceptual interface.

Every theme must define rendering for:

- presentation metadata
- title slide
- ordinary slide
- slide title
- body
- headings
- lists
- equations
- images
- tables
- code
- quotes
- footer
- slide number

Custom templates can later implement the same contract.

---

# 32. Document styles

v0.1 only needs one excellent default document style.

It should provide:

- A4 paper
- good serif body font
- strong math typography
- sensible margins
- heading hierarchy
- comfortable line spacing
- code formatting
- good tables
- page numbering where appropriate

Do not create five document themes just because slides have five themes.

---

# 33. Typst emission

Renderer interface:

```rust
pub fn emit_document(
    document: &Document,
    options: &RenderOptions,
) -> Result<String, EmitError>;
```

Presentation equivalent:

```rust
pub fn emit_presentation(
    presentation: &Presentation,
    options: &RenderOptions,
) -> Result<String, EmitError>;
```

Generated Typst should be human-readable.

Do not minify generated source.

This is useful for:

- debugging
- tests
- user inspection
- diagnosing renderer bugs

---

# 34. Escaping

Ordinary Markdown text must not accidentally become Typst syntax.

Centralize escaping.

For example:

```rust
fn escape_typst_text(input: &str) -> String;
```

Different contexts need different escaping rules:

```text
normal text
math
code/raw
URLs
```

Do not scatter escaping logic throughout rendering functions.

---

# 35. Images

Support local images.

Markdown:

```markdown
![Results](figures/results.png)
```

Initial formats should include at least:

- PNG
- JPEG
- SVG
- PDF where Typst can directly handle it appropriately

Paths are resolved relative to the source Markdown file.

Do not initially download remote images.

For:

```markdown
![](https://example.com/plot.png)
```

emit an explicit unsupported-feature diagnostic.

---

# 36. Images in slides

Images use ordinary Markdown syntax.

The slide renderer should:

- preserve aspect ratio
- constrain images to the available slide area
- avoid overflowing slide boundaries
- use presentation-appropriate spacing

Do not initially invent custom image sizing syntax unless it becomes necessary.

---

# 37. Tables

Markdown tables should compile in both modes.

Example:

```markdown
| Model | Estimate |
|---|---:|
| OLS | 0.42 |
| IV | 0.51 |
```

Documents use document-appropriate spacing.

Slides use presentation-appropriate spacing and larger text.

Do not silently shrink a table to unreadable font sizes.

If reliable overflow detection becomes available, prefer a warning.

---

# 38. Code blocks

Markdown:

````markdown
```rust
fn main() {
    println!("hello");
}
```
````

should become Typst code/raw markup.

Pass the language identifier through when available.

Unknown languages still render as plain code.

Syntax highlighting is Typst's responsibility.

The `mono` slide theme may visually emphasize code more than other themes.

---

# 39. Blockquotes

Markdown:

```markdown
> Economic models are abstractions.
```

should map to semantic quotation rendering.

Do not hard-code quotation styling in Markdown parsing.

---

# 40. Raw HTML

Raw HTML is unsupported.

Example:

```html
<div>hello</div>
```

should produce:

```text
warning: raw HTML is not supported
  --> paper.md:42:1

42 | <div>hello</div>
   | ^^^^^^^^^^^^^^^^
```

Under:

```text
--strict
```

warnings become errors.

Never silently reinterpret HTML.

---

# 41. Raw Typst

Do not allow arbitrary Typst in Markdown for v0.1.

This keeps language boundaries clear:

```text
Markdown        document structure
LaTeX syntax    math
Typst           compiler backend
```

A future explicit Typst block extension can be considered later.

---

# 42. PDF compilation

The first implementation should invoke the installed Typst executable.

Pipeline:

```text
Markdown
    ↓
your compiler
    ↓
temporary .typ
    ↓
typst compile
    ↓
PDF
```

Run directly:

```bash
typst compile generated.typ output.pdf
```

Do not run through:

```bash
sh -c
```

Use Rust's direct process API.

If Typst is unavailable:

```text
error: Typst executable was not found

Install Typst or run:

    mdpdf --emit typst document.md
```

---

# 43. Embedded Typst compiler

Embedding Typst in the Rust binary is a later milestone.

Typst exposes a Rust compilation API, so this should remain architecturally possible.

Define an abstraction:

```rust
pub trait PdfBackend {
    fn compile(
        &self,
        typst_source: &str,
        context: &CompileContext,
    ) -> Result<Vec<u8>, PdfError>;
}
```

Initial backend:

```text
ExternalTypstBackend
```

Future backend:

```text
EmbeddedTypstBackend
```

No upstream code should care which implementation is selected.

---

# 44. Diagnostics

Diagnostics are a major feature.

Whenever possible, errors should point back to Markdown rather than generated Typst.

Example:

```text
error: unsupported math command `\foo`
  --> paper.md:17:13

17 | We have $\foo{x}$ here.
   |          ^^^^^^^
```

Diagnostic levels:

```text
error
warning
note
```

Main error categories:

```rust
MarkdownError
FrontMatterError
MathParseError
MathEmitError
UnsupportedFeature
IoError
ConfigError
TypstCompileError
InternalError
```

Avoid generic errors like:

```text
Compilation failed
```

when a useful source position is available.

---

# 45. Source spans

Preserve source locations from the beginning.

Use byte ranges into the UTF-8 input:

```rust
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
```

Line and column numbers can be calculated when displaying diagnostics.

For math:

```text
Markdown source
      ↓
$ mathematical substring $
      ↓
math parser offset
      ↓
translate offset back into Markdown source
```

This allows parser errors inside equations to point to the original file.

---

# 46. Generated Typst errors

Most user-facing syntax errors should be caught before Typst compilation.

A Typst compile failure therefore usually means either:

1. a bug in the emitter, or
2. an invalid user template.

Display:

```text
error: generated Typst failed to compile

Caused by:
    ...

Run with --keep-typst to inspect generated source.
```

Do not dump pages of subprocess output by default.

Verbose mode may provide more detail.

---

# 47. Configuration

Optional project configuration:

```text
mdpdf.toml
```

Example:

```toml
[document]
paper = "a4"
font = "Libertinus Serif"
font_size = "11pt"

[page]
margin = "25mm"

[headings]
numbered = false

[code]
font = "JetBrains Mono"
```

Keep configuration deliberately small in v0.1.

Avoid turning every Typst property into a configuration option.

Users needing complete visual control should eventually use a custom template.

---

# 48. Custom templates

Support:

```bash
mdpdf paper.md --template custom.typ
```

and:

```bash
mdpdf talk.md --slides --template presentation.typ
```

Built-in and external templates should obey stable renderer contracts.

The Markdown compiler should not need modifications when a user changes visual appearance.

---

# 49. Suggested source layout

```text
src/
├── main.rs
├── cli.rs
├── compiler.rs
├── config.rs
├── metadata.rs
├── diagnostics.rs
│
├── markdown/
│   ├── mod.rs
│   ├── parser.rs
│   └── frontmatter.rs
│
├── ir/
│   ├── mod.rs
│   ├── document.rs
│   ├── block.rs
│   ├── inline.rs
│   └── presentation.rs
│
├── math/
│   ├── mod.rs
│   ├── parser.rs
│   ├── ratex.rs
│   ├── emit.rs
│   └── error.rs
│
├── typst/
│   ├── mod.rs
│   ├── document.rs
│   ├── presentation.rs
│   ├── escape.rs
│   └── backend.rs
│
└── templates/
    ├── document/
    │   └── default.typ
    └── slides/
        ├── academic.typ
        ├── minimal.typ
        ├── dark.typ
        ├── bold.typ
        └── mono.typ
```

Tests:

```text
tests/
├── markdown/
├── math/
├── documents/
├── slides/
├── snapshots/
└── integration/
```

---

# 50. Compiler orchestration

Conceptually:

```rust
pub fn compile(
    input: &Path,
    options: CompileOptions,
) -> Result<CompileResult, CompileError> {

    let source = read_source(input)?;

    let (metadata, body) =
        markdown::parse_frontmatter(&source)?;

    let effective_options =
        resolve_options(metadata, options)?;

    let document =
        markdown::parse(body, effective_options.document_type)?;

    let typst = match effective_options.document_type {
        DocumentType::Document => {
            typst::emit_document(&document, &effective_options)?
        }

        DocumentType::Slides => {
            let presentation =
                Presentation::from_document(document)?;

            typst::emit_presentation(
                &presentation,
                &effective_options,
            )?
        }
    };

    emit_outputs(typst, effective_options)
}
```

The orchestration layer should remain boring.

That is a positive design goal.

---

# 51. Output handling

For:

```bash
mdpdf paper.md
```

produce:

```text
paper.pdf
```

For:

```bash
mdpdf paper.md --emit typst
```

produce:

```text
paper.typ
```

For:

```bash
mdpdf paper.md --emit both
```

produce:

```text
paper.typ
paper.pdf
```

For:

```bash
mdpdf paper.md --keep-typst
```

preserve intermediate Typst even when PDF is the selected output.

---

# 52. Logging

Default successful compilation:

```text
Compiled paper.md → paper.pdf
```

Slides:

```text
Compiled talk.md → talk.pdf
```

Quiet:

```bash
mdpdf -q paper.md
```

prints nothing unless an error occurs.

Verbose:

```bash
mdpdf -v paper.md
```

may show:

```text
Reading metadata...
Parsing Markdown...
Parsing 17 math expressions...
Generating Typst...
Running Typst...
Compiled paper.md → paper.pdf
```

---

# 53. Exit codes

Use predictable Unix behavior.

Suggested:

```text
0    success
1    document compilation error
2    invalid CLI/configuration
3    I/O or environment error
```

Exact non-zero numbers matter less than consistency.

---

# 54. Security

Do not execute commands contained in Markdown.

Do not evaluate Markdown as Typst.

Do not download network resources in v0.1.

Do not construct shell command strings.

Use:

```rust
Command::new("typst")
```

rather than:

```rust
Command::new("sh").arg("-c")
```

If parsing untrusted math, keep the math parser dependency updated.

---

# 55. Testing strategy

Use four primary levels of testing.

## Markdown parsing tests

Input:

```markdown
This is **bold**.
```

Expected IR conceptually:

```text
Paragraph
 ├─ Text("This is ")
 ├─ Strong
 │   └─ Text("bold")
 └─ Text(".")
```

---

## Math translation tests

Input:

```latex
\frac{x_i^2}{\sqrt{n}}
```

Test:

```text
LaTeX
  ↓
RaTeX parser
  ↓
AST
  ↓
Typst emitter
  ↓
valid Typst
```

Compare with expected output.

---

## Typst snapshot tests

Fixture:

```text
tests/fixtures/basic.md
```

should produce:

```text
tests/snapshots/basic.typ
```

Use snapshots to catch renderer regressions.

---

## End-to-end tests

Run:

```text
Markdown
  ↓
compiler
  ↓
Typst
  ↓
real Typst compiler
  ↓
PDF
```

Verify at least:

- process succeeds
- output PDF exists
- PDF is non-empty
- expected page count where appropriate

Do not initially compare raw PDF bytes.

---

# 56. Math compatibility corpus

Create a dedicated math corpus.

Suggested:

```text
tests/math/
├── basic.txt
├── fractions.txt
├── scripts.txt
├── operators.txt
├── greek.txt
├── fonts.txt
├── delimiters.txt
├── accents.txt
├── matrices.txt
├── aligned.txt
├── calculus.txt
├── sets.txt
├── probability.txt
└── statistics.txt
```

Representative expressions:

```latex
E[Y \mid X]

\mathbb E[Y \mid X]

\operatorname{Var}(X)

X^\top X

(X^\top X)^{-1}

\hat\beta

\frac{\partial f(x)}{\partial x}

\sum_{i=1}^{N}

\int_{-\infty}^{\infty}

\mathbf{x}'\beta

\Pr(Y = 1 \mid X)

\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
```

Every supported expression should:

1. parse
2. translate
3. produce valid Typst
4. compile successfully

---

# 57. Document acceptance test

The following source should compile:

```markdown
# Regression

Consider the linear model

$$
Y_i = X_i^\top \beta + \varepsilon_i.
$$

The OLS estimator is

$$
\hat\beta =
\left(
\sum_{i=1}^{n} X_i X_i^\top
\right)^{-1}
\sum_{i=1}^{n} X_i Y_i.
$$

Under

$$
\mathbb E[\varepsilon_i \mid X_i] = 0,
$$

the estimator is **consistent**.

## Notes

- $X_i \in \mathbb R^K$.
- $\beta$ is a $K \times 1$ vector.
- Standard errors can be *heteroskedasticity robust*.

```rust
fn estimate(x: Matrix, y: Vector) -> Vector {
    solve(x.t() * x, x.t() * y)
}
```
```

with:

```bash
mdpdf regression.md
```

and produce a professional document PDF without manual intervention.

---

# 58. Presentation acceptance test

The following source should compile:

```markdown
---
type: slides
title: Peer Effects
subtitle: High School Application
author: Jane Researcher
style: academic
---

# Motivation

Students make educational decisions in social environments.

- Friends share information
- Choices may be strategic complements
- Policy can change peer composition

---

# Model

Student $i$ chooses

$$
a_i \in \{0,1\}.
$$

Application probability is

$$
p_i =
\operatorname{logit}^{-1}
\left(
X_i^\top\beta +
\lambda \bar p_{-i}
\right).
$$

---

# Policy experiment

Suppose a GPA cutoff is introduced.

The direct effect changes eligibility.

The equilibrium effect additionally changes peer behavior.

---

# Results

| Model | Effect |
|---|---:|
| No peers | 3.2% |
| Peer equilibrium | 4.0% |
```

Running:

```bash
mdpdf talk.md
```

must produce:

- a 16:9 PDF
- an automatically generated title slide
- four content slides
- correctly rendered mathematics
- correctly rendered table
- slide numbers
- consistent typography

Changing:

```yaml
style: academic
```

to:

```yaml
style: dark
```

must alter presentation appearance without altering parsing or semantics.

---

# 59. Recommended implementation order

Do not build everything simultaneously.

## Milestone 1 — basic document compiler

Implement:

1. CLI
2. read Markdown file
3. paragraphs
4. headings
5. emphasis
6. strong
7. lists
8. simple IR
9. Typst text emitter
10. external `typst compile`

At this point:

```bash
mdpdf simple.md
```

should generate a PDF.

---

## Milestone 2 — mathematics

Implement:

1. inline math events
2. display math events
3. RaTeX parser adapter
4. fundamental math-node conversion
5. fractions
6. scripts
7. roots
8. symbols
9. operators
10. delimiters
11. matrices
12. math compatibility tests

At this point the project already has its main distinctive value.

---

## Milestone 3 — slides

Implement:

1. front matter
2. `type: slides`
3. `--slides`
4. `SlideBreak`
5. slide segmentation
6. 16:9 Typst presentation template
7. title slides
8. slide titles
9. slide numbers
10. `academic` theme

At this point:

```bash
mdpdf talk.md
```

should produce a usable research presentation.

---

## Milestone 4 — five slide themes

Implement:

```text
academic
minimal
dark
bold
mono
```

Do not add theme-specific parsing.

All differences live in Typst templates.

---

## Milestone 5 — richer Markdown

Add:

- links
- blockquotes
- tables
- images
- code blocks
- strikethrough
- better nested lists

---

## Milestone 6 — diagnostics

Add:

- full source spans
- pretty Markdown errors
- math parser span mapping
- unsupported-feature diagnostics
- `--strict`

---

## Milestone 7 — configuration

Add:

- `mdpdf.toml`
- custom templates
- CLI/config/front-matter precedence

---

## Milestone 8 — convenience

Add:

```text
--watch
--keep-typst
--emit both
```

and improved logging.

---

## Milestone 9 — embedded Typst

Replace the subprocess implementation optionally with direct Typst library compilation.

This should not require architectural changes.

---

# 60. First coding-session stopping point

The first implementation session should stop once all of the following work:

```markdown
# Heading

Some **bold** and *italic* text.

- First
- Second

Inline mathematics: $x_i^2$.

$$
\frac{x}{y}
$$
```

and:

```bash
mdpdf test.md
```

produces:

```text
test.pdf
```

Do not begin configuration, themes, advanced tables, watch mode, or embedded Typst before this works.

---

# 61. First slides stopping point

The first slide implementation should stop once:

```markdown
---
type: slides
title: Example Presentation
author: Jane Researcher
---

# Question

Why does this matter?

---

# Model

$$
Y_i = X_i^\top \beta + \varepsilon_i
$$

---

# Result

The coefficient is **positive**.
```

successfully produces a three-content-slide presentation plus title slide.

Only the `academic` theme needs to exist at that stage.

---

# 62. Future slide layouts

Do not block v0.1 on layout syntax.

However, preserve room for a small later extension supporting common presentation layouts such as:

```text
single column
two columns
image left / text right
text left / image right
full-slide image
```

Any future syntax should remain small and explicit rather than evolving into arbitrary Typst embedded in Markdown.

Do not design this until ordinary slide generation is working.

---

# 63. Future features

Potential later features include:

- citations and bibliography
- equation numbering
- cross-references
- figure captions
- table captions
- section slides
- speaker notes
- slide fragments/reveals
- two-column layouts
- theme customization
- embedded Typst compiler
- HTML output
- standalone SVG math fallback
- additional document styles
- custom Markdown extensions

None of these should delay a usable v0.1.

---

# 64. Architectural invariants

These boundaries should remain intact throughout the project.

## Document pipeline

```text
Markdown syntax
      ↓
Document semantics / IR
      ↓
Typst semantics
      ↓
PDF
```

## Mathematics pipeline

```text
LaTeX math syntax
      ↓
math parser / AST
      ↓
Typst math representation
      ↓
PDF
```

## Presentation pipeline

```text
Document IR
     ↓
slide segmentation
     ↓
Presentation IR
     ↓
theme-independent renderer
     ↓
Typst slide theme
     ↓
PDF
```

Do not let parser-specific structures leak into unrelated layers.

Do not let theme-specific decisions leak into parsing.

Do not let Typst syntax leak into Markdown semantics.

---

# 65. Definition of success

A successful v0.1 lets a user install one small CLI and write either:

```bash
mdpdf paper.md
```

or:

```bash
mdpdf talk.md
```

using ordinary Markdown plus familiar LaTeX mathematics.

Documents should look like professional technical documents.

Slides should look like professional presentations.

The user should normally not need to know that Typst is the rendering backend.

The compiler should be small enough that one developer can understand the complete pipeline.

The essential promise is:

> Write Markdown. Write mathematics the way you already know. Get a good PDF.

For presentations:

> Use the same language, separate slides with `---`, and get a good deck.

The compiler is deliberately narrow.

That narrowness is the feature.
The compiler should be small enough that one developer can understand the complete pipeline.

The essential promise is:

> Write Markdown. Write mathematics the way you already know. Get a good PDF.

For presentations:

> Use the same language, separate slides with `---`, and get a good deck.

The compiler is deliberately narrow.

That narrowness is the feature.
