# Bundled fonts

The faces the built-in slide styles are drawn in, compiled into the binary so a
deck renders identically on every machine.

| File | Family | Licence |
| --- | --- | --- |
| `InstrumentSans.ttf` | Instrument Sans (variable) | SIL OFL 1.1 |
| `InstrumentSerif.ttf`, `InstrumentSerif-Italic.ttf` | Instrument Serif | SIL OFL 1.1 |
| `Newsreader.ttf`, `Newsreader-Italic.ttf` | Newsreader (variable) | SIL OFL 1.1 |
| `Manrope.ttf` | Manrope (variable) | SIL OFL 1.1 |
| `Geist.ttf`, `GeistMono.ttf` | Geist (variable) | SIL OFL 1.1 |
| `IBMPlexMono.ttf`, `IBMPlexMono-Bold.ttf` | IBM Plex Mono | SIL OFL 1.1 |

All are variable fonts where one exists, which is why the set costs about two
megabytes rather than ten: the styles ask for weights such as 650, and a
variable axis interpolates them rather than needing a file per weight.

Typst registers Newsreader as `Newsreader 16pt`, because the family carries an
optical-size axis. That name appears only inside the slide template.
