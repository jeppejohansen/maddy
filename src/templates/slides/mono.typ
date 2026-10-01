// The mono slide theme.
//
// For software talks, engineering talks, terminal-oriented presentations and
// technical demos. A monospaced identity, a visible grid structure, strong code
// presentation and restrained decoration — technical rather than a retro gimmick.
//
// Contract: presentation(title:, subtitle:, author:, date:, body),
// title-slide(title:, subtitle:, author:, date:) and slide(title:, body).

// DejaVu Sans Mono is bundled with Typst, so this theme's identity is the same
// on every machine.
#let mono = "DejaVu Sans Mono"

#let ink = rgb("#1d2328")
#let muted = rgb("#6b7680")
#let accent = rgb("#0f766e")
#let rule-colour = rgb("#d8dee3")

#let slide-counter = counter("maddy-slide")

#let presentation(title: none, subtitle: none, author: none, date: none, body) = {
  set document(
    title: if title != none { title } else { "" },
    author: if author != none { (author,) } else { () },
  )

  set page(
    paper: "presentation-16-9",
    margin: (x: 22mm, top: 18mm, bottom: 15mm),
    fill: white,
    // The footer rule is the visible grid line that frames the slide.
    footer: context {
      let shown = slide-counter.get().first()
      if shown > 0 {
        set text(font: mono, size: 10pt, fill: muted)
        block(stroke: (top: 0.5pt + rule-colour), inset: (top: 6pt), width: 100%)[
          #grid(
            columns: (1fr, auto),
            align(left, if title != none { title } else { [] }),
            align(right, [#str(shown)]),
          )
        ]
      }
    },
  )

  // Monospaced body at a slightly smaller size, since the face runs wide.
  set text(font: mono, size: 17pt, fill: ink, lang: "en")
  // Mathematics keeps a proper math face: monospaced equations are unreadable.
  show math.equation: set text(font: "New Computer Modern Math")

  set par(leading: 0.85em, spacing: 1.1em, justify: false)
  set block(spacing: 1.1em)

  set heading(numbering: none)
  show heading: set text(weight: "bold", fill: accent)
  show heading.where(level: 2): set text(size: 0.92em)
  show heading.where(level: 3): set text(size: 0.86em, fill: muted)
  // Sub-headings are prefixed, which is the convention this audience reads.
  // The hashes are escaped: an unescaped `#` opens code mode even in content.
  show heading.where(level: 2): it => block(above: 1em, below: 0.5em)[\#\# #it.body]
  show heading.where(level: 3): it => block(above: 0.9em, below: 0.45em)[\#\#\# #it.body]

  // The asterisk is escaped, or it would open strong emphasis.
  set list(indent: 0.5em, spacing: 0.8em, marker: text(fill: accent, [\*]))
  set enum(indent: 0.5em, spacing: 0.8em)

  set math.equation(numbering: none)
  show math.equation.where(block: true): set block(above: 1em, below: 1em)

  // Code is the point of this theme.
  show raw: set text(font: mono, size: 0.88em)
  show raw.where(block: true): it => block(
    width: 100%,
    fill: rgb("#f6f8f9"),
    stroke: 0.5pt + rule-colour,
    inset: (x: 10pt, y: 9pt),
    it,
  )

  // A visible grid: ruled rows rather than floating text.
  set table(
    stroke: (x, y) => (bottom: 0.5pt + rule-colour),
    inset: (x: 9pt, y: 6pt),
  )
  show table: it => block(above: 1em, below: 1em, align(center, text(size: 0.86em, it)))
  show table.cell.where(y: 0): set text(fill: accent, weight: "bold")

  show quote.where(block: true): it => block(
    inset: (left: 0.9em),
    stroke: (left: 2pt + accent),
    text(fill: muted, it.body),
  )

  show link: set text(fill: accent, weight: "bold")
  show image: it => align(center, box(width: 100%, it))

  body
}

#let title-slide(title: none, subtitle: none, author: none, date: none) = {
  page(margin: (x: 22mm, y: 22mm))[
    #set text(font: mono)
    #align(horizon + left)[
      #block(below: 0.6em, text(size: 0.8em, fill: muted, [\/\/ presentation]))
      #block(below: 0.5em, text(size: 1.8em, weight: "bold", fill: ink, title))
      #if subtitle != none { block(below: 1.3em, text(size: 0.95em, fill: muted, subtitle)) }
      #block(below: 1em, line(length: 100%, stroke: 0.5pt + rule-colour))
      #if author != none { block(below: 0.25em, text(size: 0.85em, author)) }
      #if date != none { block(text(size: 0.8em, fill: muted, date)) }
    ]
  ]
}

#let slide(title: none, body) = {
  pagebreak(weak: true)
  slide-counter.step()

  if title != none {
    block(below: 1.1em)[
      #text(size: 1.3em, weight: "bold", fill: accent)[\# #title]
    ]
  }

  body
}
