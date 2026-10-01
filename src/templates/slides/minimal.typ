// The minimal slide theme.
//
// For general professional presentations, internal talks, teaching and simple
// narrative decks. Modern sans-serif typography, large headings, spacious
// margins, minimal chrome, limited accents and relatively low visual density.
//
// Contract: presentation(title:, subtitle:, author:, date:, body),
// title-slide(title:, subtitle:, author:, date:) and slide(title:, body).
//
// Typst bundles no sans-serif face, so the stack names the families that are
// usually present and ends with a bundled one, which guarantees the deck always
// renders even if it falls back to a serif.
#let sans = (
  "Helvetica Neue",
  "Helvetica",
  "Arial",
  "Liberation Sans",
  "DejaVu Sans",
  "New Computer Modern",
)

#let ink = luma(28)
#let muted = luma(130)
#let rule-colour = luma(218)

#let slide-counter = counter("maddy-slide")

#let presentation(title: none, subtitle: none, author: none, date: none, body) = {
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "presentation-16-9",
    margin: (x: 30mm, top: 26mm, bottom: 18mm),
    fill: white,
    footer: context {
      let shown = slide-counter.get().first()
      if shown > 0 {
        align(right, text(size: 10pt, fill: muted, str(shown)))
      }
    },
  )

  set text(font: sans, size: 20pt, fill: ink, lang: "en")
  // Mathematics keeps its own face: a sans equation reads poorly.
  show math.equation: set text(font: "New Computer Modern Math")

  set par(leading: 0.85em, spacing: 1.2em, justify: false)
  set block(spacing: 1.2em)

  set heading(numbering: none)
  show heading: set text(weight: "medium")
  show heading.where(level: 2): set text(size: 0.9em)
  show heading.where(level: 3): set text(size: 0.82em, fill: muted)
  show heading: it => block(above: 1em, below: 0.55em, it)

  set list(indent: 0.4em, spacing: 0.95em, marker: text(fill: muted, [—]))
  set enum(indent: 0.4em, spacing: 0.95em)

  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 1em, below: 1em)

  show raw: set text(font: "DejaVu Sans Mono", size: 0.76em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: luma(250),
    inset: (x: 10pt, y: 8pt),
    it,
  )

  set table(stroke: none, inset: (x: 10pt, y: 7pt))
  show table: it => block(above: 1.1em, below: 1.1em, align(center, text(size: 0.9em, it)))
  show table.cell.where(y: 0): set text(fill: muted, weight: "medium")

  show quote.where(block: true): it => block(
    inset: (left: 1em),
    stroke: (left: 2pt + rule-colour),
    text(fill: muted, it.body),
  )

  show link: set text(fill: ink, weight: "medium")
  show image: it => align(center, box(width: 100%, it))

  body
}

#let title-slide(title: none, subtitle: none, author: none, date: none) = {
  page(margin: (x: 30mm, y: 30mm))[
    #align(horizon + left)[
      #block(below: 0.6em, text(size: 2em, weight: "medium", title))
      #if subtitle != none { block(below: 1.6em, text(size: 1.05em, fill: muted, subtitle)) }
      #if author != none { block(below: 0.25em, text(size: 0.9em, author)) }
      #if date != none { block(text(size: 0.9em, fill: muted, date)) }
    ]
  ]
}

#let slide(title: none, body) = {
  pagebreak(weak: true)
  slide-counter.step()

  if title != none {
    block(below: 1.3em, text(size: 1.45em, weight: "medium", title))
  }

  body
}
