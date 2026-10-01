//! Inline content.

use super::block::{Image, MathSource};

/// A span of inline content inside a paragraph, heading, list item or table
/// cell.
#[derive(Debug, Clone, PartialEq, Eq)]
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

impl Inline {
    pub fn text(value: impl Into<String>) -> Self {
        Inline::Text(value.into())
    }

    pub fn code(value: impl Into<String>) -> Self {
        Inline::Code(value.into())
    }

    pub fn link(destination: impl Into<String>, content: Vec<Inline>) -> Self {
        Inline::Link {
            destination: destination.into(),
            content,
        }
    }

    /// The plain-text content of this inline, with all markup removed.
    ///
    /// Used where Typst needs a bare string rather than markup, such as PDF
    /// metadata, and for the alt text of an image.
    pub fn plain_text(&self) -> String {
        let mut out = String::new();
        self.write_plain_text(&mut out);
        out
    }

    fn write_plain_text(&self, out: &mut String) {
        match self {
            Inline::Text(text) | Inline::Code(text) => out.push_str(text),
            Inline::Emphasis(content)
            | Inline::Strong(content)
            | Inline::Strike(content)
            | Inline::Link { content, .. } => {
                for inline in content {
                    inline.write_plain_text(out);
                }
            }
            Inline::Image(image) => out.push_str(&plain_text(&image.alt)),
            Inline::Math(math) => out.push_str(&math.source),
            Inline::SoftBreak | Inline::HardBreak => out.push(' '),
        }
    }
}

/// The concatenated plain text of a sequence of inlines.
pub fn plain_text(content: &[Inline]) -> String {
    let mut out = String::new();
    for inline in content {
        inline.write_plain_text(&mut out);
    }
    out
}
