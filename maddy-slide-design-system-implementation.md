# Maddy slide design system — implementation brief

## First: make the example slides so we agree on the visual system

Before changing Maddy's built-in slide implementation, **make a compiled Typst specimen deck that demonstrates the proposed system**.

The purpose of this first step is to agree on the visual language before locking it into the compiler.

Create a single example deck that shows:

1. Every proposed `style`
2. Every proposed `palette`
3. The full heading hierarchy for each style
4. Body text, lists, equations, tables, quotes, code, captions, and links
5. Title slides and ordinary content slides
6. At least one dense academic slide and one sparse presentation slide
7. The available `ratio` options
8. The available `text-size` options
9. A few representative style × palette combinations
10. A dark-background example if we decide to support dark palette variants

Compile the specimen to PDF and keep the `.typ` source.

**Do not begin by mechanically modifying the existing themes.** First make the visual specimen, inspect it, and iterate until the design feels right.

A key visual reference is the specimen slide we already liked:

- dark desaturated navy background
- warm ochre eyebrow/accent
- very large white sans-serif title
- restrained subtitle
- lots of whitespace
- thin border and rules
- small specimen grid at the bottom
- no decorative clutter
- strong hierarchy created primarily through typography, spacing, and alignment

The default Maddy slide system should aim for that level of polish.

---

# Goal

Maddy's standard slide appearance should be controlled by four independent user-facing choices:

```yaml
---
type: slides
style: modern
palette: midnight
ratio: 16:9
text-size: medium
---
```

Conceptually:

```text
style     → typography + hierarchy + spacing + layout grammar
palette   → colors only
ratio     → slide geometry
text-size → global typographic scale
```

These concerns should remain independent.

For example:

```yaml
style: academic
palette: bottle
```

and

```yaml
style: academic
palette: ochre
```

must use the same typography, spacing, layout, heading hierarchy, etc. Only the color system should change.

Likewise:

```yaml
style: editorial
palette: bottle
```

and

```yaml
style: modern
palette: bottle
```

should share the same colors but have visibly different typography and composition.

This avoids an explosion of separate templates such as:

```text
academic-blue
academic-green
academic-orange
minimal-blue
minimal-green
...
```

Instead, a small number of styles and palettes compose into many coherent combinations.

---

# 1. Styles

A `style` controls the full typographic and compositional language of the deck.

It should control:

- title/display font
- slide-title font
- H2 font
- H3 font
- body font
- caption font
- code font
- math font
- font weights
- relative type scale
- line height / leading
- paragraph spacing
- heading spacing
- slide margins
- list spacing
- bullet appearance
- title-slide composition
- section-slide composition
- ordinary slide composition
- quote treatment
- table treatment
- code-block treatment
- rule thickness and placement
- image spacing
- footer / slide-number treatment
- maximum useful text width where appropriate

A style **must not contain literal theme colors** beyond semantic references such as:

```typst
palette.primary
palette.text
palette.border
```

## Proposed built-in styles

Start with these five.

### `modern`

This should probably become the default.

Typography:

```text
Display/title: Instrument Sans Bold
Slide title:   Instrument Sans Bold
H2:            Instrument Sans Semibold
H3:            Instrument Sans Semibold
Body:          Instrument Sans Regular
Caption:       Instrument Sans Regular
Code:          IBM Plex Mono
Math:          New Computer Modern Math
```

Character:

- contemporary
- restrained
- highly legible
- substantial whitespace
- strong size hierarchy
- relatively little decoration
- suitable for teaching, research, professional talks, and general use

The default should feel designed, not like a document projected onto a wall.

### `academic`

Typography:

```text
Display/title: Newsreader Bold
Slide title:   Newsreader Bold
H2:            Newsreader Semibold
H3:            Instrument Sans Semibold
Body:          Instrument Sans Regular
Caption:       Instrument Sans Regular
Code:          IBM Plex Mono
Math:          New Computer Modern Math
```

Character:

- academic/editorial
- suitable for economics, science, seminars, and lectures
- slightly denser than `modern`
- excellent equations and tables
- serif at the top of the hierarchy, sans-serif lower down

The serif → sans transition is intentional.

### `editorial`

Typography:

```text
Display/title: Instrument Serif
Slide title:   Instrument Serif
H2:            Instrument Sans Semibold
H3:            Instrument Sans Medium
Body:          Instrument Sans Regular
Caption:       Instrument Sans Regular
Code:          IBM Plex Mono
Math:          New Computer Modern Math
```

Character:

- expressive
- magazine/editorial feel
- strong title and section slides
- display serif used sparingly
- large typography and whitespace
- should remain suitable for serious presentations

Instrument Serif should generally be limited to:

- title slides
- section slides
- large statements
- slide titles where appropriate

Do not use it for ordinary body text.

### `bold`

Typography:

```text
Display/title: Manrope ExtraBold
Slide title:   Manrope Bold
H2:            Manrope Bold
H3:            Manrope Semibold
Body:          Manrope Regular
Caption:       Manrope Regular
Code:          IBM Plex Mono
Math:          New Computer Modern Math
```

Character:

- keynote-like
- large body text
- short line lengths
- oversized headings
- one main idea per slide
- strong visual hierarchy
- suitable for talks, pitches, and highly visual presentations

This style should differ from `modern` in layout and scale, not merely in font.

### `technical`

Typography:

```text
Display/title: Geist Bold
Slide title:   Geist Bold
H2:            Geist Semibold
H3:            IBM Plex Mono
Body:          Geist Regular
Caption:       IBM Plex Mono
Code:          IBM Plex Mono
Math:          New Computer Modern Math
```

Character:

- technical
- software / methods / engineering
- structured
- restrained
- strong code presentation
- occasional monospaced accents

Do **not** make the entire deck monospaced.

The existing `mono` theme can eventually be replaced or reinterpreted by this style.

---

# 2. Full typographic hierarchy

Do not model typography as merely:

```text
heading font
body font
```

Each style should define a complete hierarchy.

A useful internal representation is something conceptually like:

```typst
#let typography = (
  eyebrow: (
    family: "Instrument Sans",
    weight: 700,
    scale: 0.62,
  ),

  display: (
    family: "Instrument Sans",
    weight: 700,
    scale: 2.15,
  ),

  title: (
    family: "Instrument Sans",
    weight: 700,
    scale: 1.65,
  ),

  h2: (
    family: "Instrument Sans",
    weight: 650,
    scale: 1.25,
  ),

  h3: (
    family: "Instrument Sans",
    weight: 650,
    scale: 1.00,
  ),

  body: (
    family: "Instrument Sans",
    weight: 400,
    scale: 1.00,
  ),

  caption: (
    family: "Instrument Sans",
    weight: 400,
    scale: 0.72,
  ),

  code: (
    family: "IBM Plex Mono",
    weight: 400,
    scale: 0.78,
  ),
)
```

The exact scale differs by style.

## Eyebrow / kicker text

The visual system should support a small text level above the main title.

Example:

```text
Maddy slide typography
Font hierarchy for generated slides
```

where the first line is:

- small
- bold
- accent colored
- optionally letter-spaced
- visually separated from the main title

This can initially exist as an internal style primitive even if Markdown syntax for it is not yet exposed.

---

# 3. Text-size

Add:

```yaml
text-size: small
```

```yaml
text-size: medium
```

```yaml
text-size: large
```

Default:

```yaml
text-size: medium
```

This should control the **base body size**, while every other size is derived from style-specific ratios.

Do not hard-code every heading size independently for every size preset.

Conceptually:

```text
effective size = body-size × style-specific multiplier
```

Example baseline:

| text-size | body |
|---|---:|
| `small` | ~18 pt |
| `medium` | ~21 pt |
| `large` | ~24 pt |

A style then defines multipliers.

For example `modern` might approximately use:

```text
body      1.00
h3        1.10
h2        1.25
title     1.65
display   2.15
caption   0.72
code      0.78
```

With a `21pt` body:

```text
body      21 pt
h3        23 pt
h2        26 pt
title     35 pt
display   45 pt
caption   15 pt
code      16 pt
```

The exact values should be tuned from the specimen slides.

Different styles can use different ratios.

For example `bold` should use a larger display/title multiplier than `academic`.

## Optional numeric override

Ideally also support:

```yaml
text-size: 22pt
```

This means:

```text
body size = 22pt
```

while the rest of the hierarchy continues to follow the selected style's scale.

This can be implemented after the named presets if necessary.

Use the name `text-size`, not simply `size`, because `size` is ambiguous with paper/page size.

---

# 4. Ratio

Add:

```yaml
ratio: 16:9
```

Default:

```yaml
ratio: 16:9
```

Initial supported ratios:

```text
16:9
4:3
3:2
```

Possible later additions:

```text
16:10
1:1
```

Do not make ratio part of a style.

It should purely control page geometry.

The style should adapt naturally to the geometry through:

- margin logic
- available width
- title widths
- text measure
- grid widths
- image constraints

Avoid separate duplicated templates for every ratio.

---

# 5. Palettes

A palette controls colors only.

Start with these six:

```text
midnight
bottle
ochre
claret
aubergine
petrol
```

Every palette should expose the **same semantic token interface**.

Recommended internal tokens:

```typst
(
  primary: ...,
  primary-dark: ...,
  secondary: ...,
  accent: ...,

  background: ...,
  surface: ...,

  text: ...,
  text-muted: ...,
  border: ...,
)
```

Styles reference semantic tokens only.

For example:

```typst
text(fill: palette.primary)[...]
```

and:

```typst
block(
  fill: palette.surface,
  stroke: palette.border,
)
```

## Midnight

Core colors:

```text
primary       #18324A
primary-dark  #0E1F2F
secondary     #6F92B0
accent        #D2A86A
```

Light background system should be derived coherently from these.

Suggested feel:

- dark navy
- warm brass
- serious
- scientific
- elegant

This is a strong candidate for the default palette.

## Bottle

Core colors:

```text
primary       #1F4D3A
primary-dark  #102E24
secondary     #94AD9D
accent        #C27A4A
```

Suggested background:

```text
background    #F8F6F0
surface       #F3F0E7
```

Suggested feel:

- bottle green
- cream
- copper
- slightly bookish/editorial
- especially good for lectures and academic material

## Ochre

Core colors:

```text
primary       #A45A12
primary-dark  #5A3216
secondary     #D29A57
accent        #24475B
```

Suggested feel:

- burnt ochre
- cognac
- dark blue counter-accent
- warm but not saturated
- especially good for `bold`

## Claret

Core colors:

```text
primary       #7C3044
primary-dark  #431C28
secondary     #C98B99
accent        #73816B
```

Suggested feel:

- wine/burgundy
- restrained
- editorial
- good for social science / humanities / serious professional decks

## Aubergine

Core colors:

```text
primary       #5B3A64
primary-dark  #2D1F34
secondary     #B392B7
accent        #BA945B
```

Suggested feel:

- dark plum
- muted violet
- brass
- distinctive but not flashy

## Petrol

Core colors:

```text
primary       #1D5C63
primary-dark  #12383D
secondary     #91B4B2
accent        #D07C68
```

Suggested feel:

- petrol / teal
- dusty coral
- contemporary
- particularly useful for data and technical slides

---

# 6. Dark palettes

Darkness should be a **palette concern**, not a style.

Do not preserve the current conceptual model where:

```yaml
style: dark
```

means a specific combination of dark colors, typography, and layout.

Prefer something along the lines of:

```yaml
style: modern
palette: midnight-dark
```

or:

```yaml
style: academic
palette: bottle-dark
```

This means any style can have a dark presentation.

A dark palette should be designed deliberately rather than mechanically inverted.

Principles:

- do not use pure black unless necessary
- do not use pure white for body text
- reduce projector halation
- maintain strong contrast
- muted text must still be readable
- rules/borders should be subtle
- accent colors may need separate dark-background values

The reference slide's dark navy + off-white + ochre combination is a good target.

Dark variants can be postponed until the light system is stable, but architecture should permit them cleanly.

---

# 7. Fonts and determinism

Maddy should render the same presentation on every machine.

Therefore, strongly prefer bundling the small curated set of open fonts required by the built-in styles rather than relying on:

```text
Helvetica Neue
Helvetica
Arial
system fallbacks
```

Target font set:

```text
Instrument Sans
Newsreader
Instrument Serif
Manrope
Geist
IBM Plex Mono
New Computer Modern Math
```

Only ship the weights/styles actually used if binary size matters.

The built-in style output should be deterministic.

A custom user template or explicit user font configuration may still use machine fonts.

---

# 8. Visual principles

The specimen slide we liked captures several design rules that should become general Maddy slide principles.

## Strong hierarchy

Slides should have much stronger scale differences than documents.

A slide title should not look like a slightly enlarged section heading.

Use:

- large titles
- smaller subtitles
- clearly subordinate H2/H3 levels
- small captions
- optionally tiny eyebrow text

## Whitespace is structural

Do not fill the page merely because space exists.

Whitespace should separate:

- title from body
- major conceptual regions
- text from figures
- captions from primary content

Sparse slides should remain sparse.

## Avoid decorative chrome

Prefer:

- typography
- spacing
- alignment
- thin rules
- subtle blocks

Avoid:

- unnecessary rounded cards
- gradients by default
- shadows
- excessive borders
- decorative icons
- large areas of accent color without purpose

## Rules and borders are quiet

Rules should usually be:

- thinner than text strokes
- lower contrast than text
- semantic separators rather than decoration

## Prefer warm/cool balance

The palettes intentionally often pair a cool base with a warm accent, or vice versa.

Accent colors should be used sparingly for:

- eyebrow text
- highlighted result
- small chart emphasis
- rule
- bullet marker
- section identifier

Do not color every heading and every graphical element with the accent.

## Avoid pure black and pure white where practical

Especially for dark slides.

Use softened near-black backgrounds and off-white text where it improves projection.

---

# 9. Slide components to design explicitly

The specimen deck should include and the final styles should define each of these deliberately.

## Title slide

Needs:

- eyebrow/kicker support
- display title
- subtitle
- author
- date
- optional rule or accent
- strong whitespace

## Section slide

A section divider should not just be an ordinary slide with less content.

Give styles an intentional section-slide treatment.

Possible pattern:

```text
small section number / eyebrow
large section title
one-line descriptor
```

## Ordinary content slide

Needs:

- slide title
- optional subtitle
- body region
- sensible text measure
- slide number treatment

## Heading hierarchy inside a slide

Demonstrate:

```text
slide title
H2
H3
body
caption
```

## Lists

Ensure:

- bullet spacing is presentation-appropriate
- indentation is shallow
- marker does not dominate
- nested lists remain legible

## Equations

Academic and technical styles must make display equations feel native.

Use proper math typography.

Equations should have deliberate vertical whitespace.

## Tables

Tables should generally avoid full cell grids.

Prefer:

- whitespace
- selective rules
- emphasized header row
- strong number alignment

## Code

Code blocks should feel integrated with the style.

Do not make code a completely separate visual universe.

## Quotes / callouts

Keep them restrained.

A quote can use:

- a thin accent rule
- a subtle surface color
- modest typography changes

Avoid giant quotation marks or excessive decoration.

## Figures and captions

Captions should have their own typography token.

Figure layout should respect slide margins and avoid accidental overflow.

---

# 10. User-facing syntax

Front matter:

```yaml
---
type: slides
title: Peer Effects
subtitle: High School Application
author: Jane Researcher
date: October 2026

style: academic
palette: bottle
ratio: 16:9
text-size: medium
---
```

CLI equivalents should be supported where appropriate:

```bash
maddy talk.md \
  --style academic \
  --palette bottle \
  --ratio 16:9 \
  --text-size medium
```

If CLI surface area is intentionally kept smaller, front matter + config are sufficient initially, but keep the internal configuration model clean.

Configuration should follow existing precedence:

```text
built-in defaults
→ maddy.toml
→ front matter
→ CLI
```

Suggested defaults:

```text
style: modern
palette: midnight
ratio: 16:9
text-size: medium
```

---

# 11. Architecture

Prefer composition over duplicated `.typ` themes.

A useful architecture could look approximately like:

```text
src/templates/slides/
    base.typ
    styles/
        modern.typ
        academic.typ
        editorial.typ
        bold.typ
        technical.typ
    palettes/
        midnight.typ
        bottle.typ
        ochre.typ
        claret.typ
        aubergine.typ
        petrol.typ
```

The exact module structure can differ if another organization fits the current Rust/Typst embedding better.

Conceptually:

```text
presentation
    ├── ratio
    ├── text-size
    ├── style
    │     ├── typography
    │     ├── spacing
    │     ├── layout
    │     └── component treatments
    └── palette
          └── semantic colors
```

The Rust layer should choose/configure these pieces.

The Typst layer should consume semantic configuration and perform visual composition.

Avoid generating large amounts of style-specific Typst code in Rust if it can remain cleanly expressed in Typst templates.

---

# 12. Backwards compatibility

Current Maddy styles include:

```text
academic
minimal
dark
bold
mono
```

Plan migration deliberately.

Likely mapping:

```text
academic → academic
minimal  → modern, or retain as an alias temporarily
bold     → bold
mono     → technical, or retain as an alias temporarily
dark     → migrate toward a dark palette
```

Do not silently break users if these names are already public.

Potential transition:

```text
minimal → alias of modern
mono    → alias of technical
dark    → deprecated alias resolving to modern + midnight-dark
```

Only do this if it can be implemented without making configuration confusing.

---

# 13. Implementation sequence

## Phase 1 — specimen only

Create the Typst specimen deck.

Do not yet change user-facing behavior.

The specimen should make it easy to answer:

- Are the fonts right?
- Is the hierarchy right?
- Are titles large enough?
- Is body text large enough?
- Do palettes feel coherent?
- Does each style have a distinct personality?
- Are any combinations ugly?
- Do dense slides still work?
- Do equations and tables look good?
- Does 4:3 still work?
- Do `small`, `medium`, and `large` feel useful?

Compile it to PDF.

## Phase 2 — agree on tokens and defaults

Once the specimen looks right, freeze:

- style names
- palette names
- semantic palette tokens
- typography levels
- text-size presets
- ratio names
- default combination

## Phase 3 — refactor slide template architecture

Separate style from palette.

Introduce shared slide primitives.

Avoid copy-pasting whole templates for every combination.

## Phase 4 — front matter/config parsing

Add:

```text
palette
ratio
text-size
```

while preserving existing configuration behavior.

## Phase 5 — font bundling

Bundle the selected font files/weights needed for deterministic built-in rendering.

Ensure font loading stays fast.

## Phase 6 — tests

Add tests for:

- parsing valid style
- parsing valid palette
- parsing ratio
- parsing named text sizes
- invalid values
- precedence
- style/palette independence
- deterministic Typst generation
- all built-in combinations compile
- all supported ratios compile
- all text sizes compile

A useful compile matrix is:

```text
5 styles
× 6 palettes
× 3 ratios
× 3 text sizes
```

That is 270 combinations.

Not every combination needs a golden PDF, but every combination should at least compile.

Use a much smaller representative subset for visual regression/golden tests if desired.

---

# 14. Acceptance criteria

The implementation is successful when:

1. `style`, `palette`, `ratio`, and `text-size` are independent.
2. A style contains no hard-coded palette identity.
3. All palettes expose the same semantic tokens.
4. Built-in font rendering is deterministic across supported platforms.
5. The default deck looks intentionally designed without user configuration.
6. A title slide has significantly stronger hierarchy than the current themes.
7. Academic slides can still accommodate equations, tables, and moderately dense material.
8. Bold/editorial slides can be sparse without looking unfinished.
9. Technical slides handle code naturally without making everything monospaced.
10. All built-in style × palette combinations compile.
11. `16:9`, `4:3`, and `3:2` compile and remain usable.
12. `small`, `medium`, and `large` produce coherent hierarchy rather than simply scaling one font.
13. Existing public theme names are either preserved or migrated deliberately.
14. The first deliverable is a compiled specimen PDF that we can inspect before final implementation.

---

# 15. First task for the agent

**Start here.**

Create:

```text
examples/slide-design-system.typ
examples/slide-design-system.pdf
```

or an equivalent location that fits the repository.

The deck should contain, at minimum:

- one overview slide using the dark navy / ochre reference aesthetic
- one typography specimen slide per style
- one palette specimen slide showing all semantic colors
- one title-slide example per style
- one representative content slide per style
- an academic slide with equation + table
- a technical slide with code
- an editorial slide with a large statement
- a bold slide with very little text
- a slide comparing `small`, `medium`, and `large`
- a slide comparing `16:9`, `3:2`, and `4:3` conceptually
- a matrix or selection of style × palette combinations

Use real Typst wherever possible rather than mockups.

Compile the deck and present the PDF before proceeding to the implementation.

The point of the first pass is not to finish the theming system. The point is to make the design concrete enough that we can agree on it.
