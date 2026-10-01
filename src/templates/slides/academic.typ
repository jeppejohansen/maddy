// The academic slide theme.
//
// For economics seminars, scientific presentations, lectures, research
// conferences and technical talks. Light background, restrained typography,
// generous whitespace, excellent equations, clear tables, unobtrusive slide
// numbers, and enough room to tolerate moderately dense content.
//
// Contract: a slide template defines three functions.
//
//   presentation(title:, subtitle:, author:, date:, body)  global setup
//   title-slide(title:, subtitle:, author:, date:)         the opening slide
//   slide(title:, body)                                    one ordinary slide
//
// The Rust renderer applies `presentation` with a show rule and then calls
// `title-slide` and `slide`. It never inspects what they do.

#let accent = rgb("#1f3864")
#let muted = luma(95)

// Slides are counted separately from pages, so that a title slide can be
// excluded from the numbering without disturbing it.
#let slide-counter = counter("maddy-slide")

#let presentation(
  title: none,
  subtitle: none,
  author: none,
  date: none,
  body,
) = {
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "presentation-16-9",
    margin: (x: 24mm, top: 20mm, bottom: 16mm),
    fill: white,
    footer: context {
      let shown = slide-counter.get().first()
      if shown > 0 {
        set text(size: 11pt, fill: muted)
        grid(
          columns: (1fr, auto),
          align(left, if author != none { author } else { [] }),
          align(right, str(shown)),
        )
      }
    },
  )

  // Only fonts Typst bundles are named, so a deck looks the same everywhere.
  set text(font: ("New Computer Modern", "Libertinus Serif"), size: 20pt, lang: "en")
  show math.equation: set text(font: "New Computer Modern Math")

  set par(leading: 0.78em, spacing: 1.05em, justify: false)
  set block(spacing: 1.05em)

  // Headings inside a slide are subsections of its title.
  set heading(numbering: none)
  show heading: set text(weight: "bold", fill: accent)
  show heading.where(level: 2): set text(size: 0.92em)
  show heading.where(level: 3): set text(size: 0.84em, style: "italic")
  show heading: it => block(above: 0.9em, below: 0.5em, it)

  set list(indent: 0.6em, spacing: 0.78em, marker: text(fill: accent, [•]))
  set enum(indent: 0.6em, spacing: 0.78em)

  // Equations are the point of this theme: give them room.
  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 0.9em, below: 0.9em)

  show raw: set text(font: "DejaVu Sans Mono", size: 0.78em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: luma(248),
    stroke: 0.5pt + luma(220),
    radius: 2pt,
    inset: (x: 8pt, y: 7pt),
    it,
  )

  // Tables: larger text than a document would use, and horizontal rules only.
  set table(stroke: none, inset: (x: 9pt, y: 6pt))
  show table: it => block(above: 1em, below: 1em, align(center, text(size: 0.92em, it)))
  show table.cell.where(y: 0): strong

  show quote.where(block: true): it => block(
    inset: (left: 1em),
    stroke: (left: 2pt + accent.lighten(55%)),
    text(style: "italic", it.body),
  )

  show link: set text(fill: accent)

  // Images must fit the slide rather than overflow it.
  show image: it => align(center, box(width: 100%, it))

  body
}

#let title-slide(title: none, subtitle: none, author: none, date: none) = {
  // Unnumbered: the slide counter has not started yet.
  page(margin: (x: 28mm, y: 26mm))[
    #align(horizon + left)[
      #block(below: 0.5em, text(size: 1.7em, weight: "bold", fill: accent, title))
      #if subtitle != none {
        block(below: 1.4em, text(size: 1.08em, fill: muted, subtitle))
      }
      #line(length: 36%, stroke: 1pt + accent.lighten(40%))
      #if author != none { block(above: 1.1em, below: 0.3em, text(size: 1em, author)) }
      #if date != none { block(text(size: 0.84em, fill: muted, date)) }
    ]
  ]
}

#let slide(title: none, body) = {
  pagebreak(weak: true)
  slide-counter.step()

  if title != none {
    block(
      width: 100%,
      below: 1.1em,
      stroke: (bottom: 0.6pt + accent.lighten(55%)),
      inset: (bottom: 0.45em),
      text(size: 1.22em, weight: "bold", fill: accent, title),
    )
  }

  body
}
