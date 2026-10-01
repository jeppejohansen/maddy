// The bold slide theme.
//
// For keynote-style presentations, pitches, and talks where one idea at a time
// should dominate the slide. Oversized headings, larger body text, a strong
// typographic scale, accent blocks and strong section breaks — assertive without
// becoming decorative clutter.
//
// Contract: presentation(title:, subtitle:, author:, date:, body),
// title-slide(title:, subtitle:, author:, date:) and slide(title:, body).
//
// Typst bundles no sans-serif face, so the stack names the families that are
// usually present and ends with a bundled one.
#let sans = (
  "Helvetica Neue",
  "Helvetica",
  "Arial",
  "Liberation Sans",
  "DejaVu Sans",
  "New Computer Modern",
)

#let accent = rgb("#c2410c")
#let ink = luma(20)
#let muted = luma(110)

#let slide-counter = counter("maddy-slide")

#let presentation(title: none, subtitle: none, author: none, date: none, body) = {
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "presentation-16-9",
    margin: (x: 26mm, top: 24mm, bottom: 18mm),
    fill: white,
    footer: context {
      let shown = slide-counter.get().first()
      if shown > 0 {
        align(right, text(size: 11pt, weight: "bold", fill: accent, str(shown)))
      }
    },
  )

  // Larger body text than the other themes: fewer words, read from further away.
  set text(font: sans, size: 24pt, fill: ink, lang: "en")
  show math.equation: set text(font: "New Computer Modern Math")

  set par(leading: 0.8em, spacing: 1.1em, justify: false)
  set block(spacing: 1.1em)

  set heading(numbering: none)
  show heading: set text(weight: "bold")
  show heading.where(level: 2): set text(size: 0.95em, fill: accent)
  show heading.where(level: 3): set text(size: 0.85em, fill: muted)
  show heading: it => block(above: 1em, below: 0.5em, it)

  set list(indent: 0.5em, spacing: 1em, marker: text(fill: accent, weight: "bold", [▪]))
  set enum(indent: 0.5em, spacing: 1em)

  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 1.05em, below: 1.05em)

  show raw: set text(font: "DejaVu Sans Mono", size: 0.7em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: luma(247),
    stroke: (left: 3pt + accent),
    inset: (x: 10pt, y: 8pt),
    it,
  )

  set table(stroke: none, inset: (x: 10pt, y: 8pt))
  show table: it => block(above: 1.1em, below: 1.1em, align(center, text(size: 0.82em, it)))
  show table.cell.where(y: 0): set text(fill: accent, weight: "bold")

  // A quotation is an accent block here, not a marginal note.
  show quote.where(block: true): it => block(
    width: 100%,
    fill: accent.lighten(90%),
    inset: (x: 14pt, y: 12pt),
    text(weight: "medium", it.body),
  )

  show link: set text(fill: accent, weight: "bold")
  show image: it => align(center, box(width: 100%, it))

  body
}

#let title-slide(title: none, subtitle: none, author: none, date: none) = {
  page(margin: 0pt)[
    #block(width: 100%, height: 100%, fill: accent, inset: (x: 30mm, y: 28mm))[
      #align(horizon + left)[
        #block(below: 0.45em, text(size: 2.4em, weight: "bold", fill: white, title))
        #if subtitle != none {
          block(below: 1.5em, text(size: 1.05em, fill: white.darken(12%), subtitle))
        }
        #if author != none {
          block(below: 0.25em, text(size: 0.85em, fill: white, author))
        }
        #if date != none { block(text(size: 0.8em, fill: white.darken(18%), date)) }
      ]
    ]
  ]
}

#let slide(title: none, body) = {
  pagebreak(weak: true)
  slide-counter.step()

  if title != none {
    block(below: 1.2em)[
      #text(size: 1.6em, weight: "bold", title)
      #block(above: 0.4em, line(length: 12%, stroke: 4pt + accent))
    ]
  }

  body
}
