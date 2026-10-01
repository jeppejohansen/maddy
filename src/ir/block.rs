//! Block-level content.

use crate::diagnostics::SourceSpan;

use super::inline::Inline;

/// A block-level element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),

    Heading {
        level: u8,
        content: Vec<Inline>,
        span: SourceSpan,
    },

    BlockQuote(Vec<Block>),

    UnorderedList(Vec<ListItem>),

    OrderedList {
        start: u64,
        items: Vec<ListItem>,
    },

    CodeBlock(CodeBlock),

    Table(Table),

    Image(Image),

    /// Display mathematics.
    Math(MathSource),

    /// A thematic break, in document mode.
    Rule,

    /// A thematic break, in slides mode.
    ///
    /// The Markdown parser emits this instead of [`Block::Rule`] when compiling
    /// a presentation; a later segmentation pass turns the resulting block list
    /// into slides. Keeping it in the block list means both modes share one
    /// parser.
    SlideBreak,
}

impl Block {
    pub fn paragraph(content: Vec<Inline>) -> Self {
        Block::Paragraph(content)
    }

    pub fn heading(level: u8, content: Vec<Inline>) -> Self {
        Block::Heading {
            level,
            content,
            span: SourceSpan::default(),
        }
    }

    /// Whether this is a level-one heading, which a slide treats as its title.
    pub fn is_slide_title(&self) -> bool {
        matches!(self, Block::Heading { level: 1, .. })
    }
}

/// One item of an ordered or unordered list.
///
/// An item holds blocks rather than inlines so that nested lists, paragraphs and
/// display mathematics inside a list item need no special representation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListItem {
    pub blocks: Vec<Block>,
}

impl ListItem {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }

    /// A single-paragraph item, the overwhelmingly common case.
    pub fn text(content: Vec<Inline>) -> Self {
        Self {
            blocks: vec![Block::Paragraph(content)],
        }
    }
}

/// A fenced or indented code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// The info string of a fenced block, when present.
    ///
    /// Passed through to Typst, which owns syntax highlighting. An unknown
    /// language renders as plain code rather than failing.
    pub language: Option<String>,
    pub source: String,
}

/// A Markdown table.
///
/// Column alignments come from the delimiter row and apply to both the header
/// and the body, so they are stored once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    pub alignments: Vec<Alignment>,
    pub header: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

impl Table {
    /// The number of columns, taken as the widest row.
    ///
    /// Markdown permits ragged rows; the renderer pads them.
    pub fn columns(&self) -> usize {
        self.rows
            .iter()
            .map(Vec::len)
            .chain(std::iter::once(self.header.len()))
            .max()
            .unwrap_or(0)
    }

    /// The alignment of column `index`, defaulting to [`Alignment::None`] for
    /// columns the delimiter row did not describe.
    pub fn alignment(&self, index: usize) -> Alignment {
        self.alignments.get(index).copied().unwrap_or_default()
    }

    pub fn has_header(&self) -> bool {
        !self.header.is_empty()
    }
}

/// Column alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    /// Unspecified; the renderer chooses.
    #[default]
    None,
    Left,
    Center,
    Right,
}

/// An image reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// The destination exactly as written, before path resolution.
    pub destination: String,
    pub alt: Vec<Inline>,
    pub title: Option<String>,
    pub span: SourceSpan,
}

impl Image {
    pub fn new(destination: impl Into<String>, alt: Vec<Inline>) -> Self {
        Self {
            destination: destination.into(),
            alt,
            title: None,
            span: SourceSpan::default(),
        }
    }

    /// Whether the destination is a remote URL.
    ///
    /// Remote images are not fetched; the compiler reports an unsupported
    /// feature instead of silently reaching out to the network.
    pub fn is_remote(&self) -> bool {
        let destination = self.destination.trim_start().to_ascii_lowercase();
        ["http://", "https://", "//"]
            .iter()
            .any(|p| destination.starts_with(p))
    }
}

/// Mathematics, stored as its original LaTeX source.
///
/// Mathematics is *not* parsed during Markdown parsing. Keeping the source here
/// means the Markdown layer never depends on the math parser, and the span lets
/// an error inside an equation point back into the Markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathSource {
    pub source: String,
    pub mode: MathMode,
    /// The span of the mathematical content, excluding the `$` delimiters.
    pub span: SourceSpan,
}

impl MathSource {
    pub fn new(source: impl Into<String>, mode: MathMode, span: SourceSpan) -> Self {
        Self {
            source: source.into(),
            mode,
            span,
        }
    }

    pub fn inline(source: impl Into<String>) -> Self {
        Self::new(source, MathMode::Inline, SourceSpan::default())
    }

    pub fn display(source: impl Into<String>) -> Self {
        Self::new(source, MathMode::Display, SourceSpan::default())
    }
}

/// Whether mathematics is set inline or as a displayed equation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathMode {
    Inline,
    Display,
}

impl MathMode {
    pub fn is_display(self) -> bool {
        matches!(self, MathMode::Display)
    }
}
