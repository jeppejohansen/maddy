// The default document theme.
//
// Contract: a document template defines `article`, taking the metadata fields
// as named arguments and the body as its final positional argument. The Rust
// renderer applies it with `#show: article.with(...)` and never inspects what it
// does, so appearance can be changed here alone.

#let article(
  title: none,
  subtitle: none,
  author: none,
  date: none,
  body,
) = {
  // PDF metadata.
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "a4",
    margin: (x: 25mm, top: 28mm, bottom: 30mm),
    numbering: "1",
    number-align: center,
  )

  // Only fonts Typst bundles are named, so output is identical on every
  // machine and no font-fallback warnings appear. New Computer Modern pairs
  // with the bundled math font, which is what makes inline mathematics sit
  // correctly in a line of prose.
  set text(font: ("New Computer Modern", "Libertinus Serif"), size: 11pt, lang: "en")
  show math.equation: set text(font: "New Computer Modern Math")

  set par(justify: true, leading: 0.72em, spacing: 1.15em)
  set block(spacing: 1.15em)

  // Headings: a clear hierarchy without numbering, which v0.1 does not offer.
  set heading(numbering: none)
  show heading: set text(weight: "bold")
  show heading.where(level: 1): set text(size: 1.35em)
  show heading.where(level: 2): set text(size: 1.15em)
  show heading.where(level: 3): set text(size: 1.0em, style: "italic")
  show heading: it => block(above: 1.6em, below: 0.85em, it)

  // Displayed equations get room to breathe; inline ones must not disturb
  // leading.
  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 1.1em, below: 1.1em)

  set list(indent: 1em, spacing: 0.72em)
  set enum(indent: 1em, spacing: 0.72em)

  // Code.
  show raw: set text(font: "DejaVu Sans Mono", size: 0.92em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: luma(249),
    stroke: 0.5pt + luma(225),
    radius: 2pt,
    inset: (x: 8pt, y: 7pt),
    it,
  )

  // Tables: horizontal rules only, which is the convention for scientific
  // tables and far quieter than a full grid.
  set table(
    stroke: none,
    inset: (x: 7pt, y: 5pt),
  )
  show table: it => block(above: 1.3em, below: 1.3em, align(center, it))
  show table.cell.where(y: 0): strong

  show quote.where(block: true): it => block(
    inset: (left: 1.2em),
    stroke: (left: 1.5pt + luma(200)),
    text(style: "italic", it.body),
  )

  show link: set text(fill: rgb("#1a4f8a"))

  // Title block, present only when the document gives a title.
  if title != none {
    align(center)[
      #block(above: 0pt, below: 0.6em, text(size: 1.7em, weight: "bold", title))
      #if subtitle != none {
        block(below: 0.9em, text(size: 1.15em, style: "italic", subtitle))
      }
      #if author != none { block(below: 0.35em, text(size: 1.05em, author)) }
      #if date != none { block(below: 0pt, text(size: 0.95em, date)) }
    ]
    v(1.6em)
  }

  body
}
