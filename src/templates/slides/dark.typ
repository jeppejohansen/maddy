// The dark slide theme.
//
// For talks in dark rooms, technical presentations and visually focused decks.
//
// This is designed as a dark theme rather than produced by inverting a light
// one. The background is a desaturated near-black rather than pure black, body
// text is a soft off-white rather than pure white — the pair that reduces halation
// on a projector — secondary text is genuinely muted rather than merely grey, and
// the accent is chosen for legibility against the background rather than
// inherited from a light theme.
//
// Contract: presentation(title:, subtitle:, author:, date:, body),
// title-slide(title:, subtitle:, author:, date:) and slide(title:, body).

#let background = rgb("#14171c")
#let surface = rgb("#1c2027")
#let ink = rgb("#e8eaed")
#let muted = rgb("#949aa4")
#let accent = rgb("#79b8ff")

#let slide-counter = counter("maddy-slide")

#let presentation(title: none, subtitle: none, author: none, date: none, body) = {
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "presentation-16-9",
    margin: (x: 24mm, top: 20mm, bottom: 16mm),
    fill: background,
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

  set text(font: ("New Computer Modern", "Libertinus Serif"), size: 20pt, fill: ink, lang: "en")
  show math.equation: set text(font: "New Computer Modern Math")

  set par(leading: 0.8em, spacing: 1.05em, justify: false)
  set block(spacing: 1.05em)

  set heading(numbering: none)
  show heading: set text(weight: "bold", fill: accent)
  show heading.where(level: 2): set text(size: 0.92em)
  show heading.where(level: 3): set text(size: 0.84em, style: "italic", fill: muted)
  show heading: it => block(above: 0.9em, below: 0.5em, it)

  set list(indent: 0.6em, spacing: 0.8em, marker: text(fill: accent, [•]))
  set enum(indent: 0.6em, spacing: 0.8em)

  // Equations are set in the body colour, so they do not appear to recede.
  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 0.95em, below: 0.95em)

  // Code is a strength of this theme: a raised surface, not a hole in the slide.
  show raw: set text(font: "DejaVu Sans Mono", size: 0.78em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: surface,
    stroke: 0.5pt + rgb("#2d333b"),
    radius: 3pt,
    inset: (x: 9pt, y: 8pt),
    it,
  )

  set table(stroke: none, inset: (x: 9pt, y: 6pt))
  show table: it => block(above: 1em, below: 1em, align(center, text(size: 0.92em, it)))
  show table.cell.where(y: 0): set text(fill: accent, weight: "bold")

  show quote.where(block: true): it => block(
    inset: (left: 1em),
    stroke: (left: 2pt + accent.darken(25%)),
    text(style: "italic", fill: muted, it.body),
  )

  show link: set text(fill: accent)
  show image: it => align(center, box(width: 100%, it))

  body
}

#let title-slide(title: none, subtitle: none, author: none, date: none) = {
  page(margin: (x: 28mm, y: 26mm), fill: background)[
    #align(horizon + left)[
      #block(below: 0.5em, text(size: 1.7em, weight: "bold", fill: ink, title))
      #if subtitle != none { block(below: 1.4em, text(size: 1.08em, fill: muted, subtitle)) }
      #line(length: 36%, stroke: 1pt + accent)
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
      stroke: (bottom: 0.6pt + rgb("#2d333b")),
      inset: (bottom: 0.45em),
      text(size: 1.22em, weight: "bold", fill: accent, title),
    )
  }

  body
}
