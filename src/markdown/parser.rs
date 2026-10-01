//! Markdown to IR.
//!
//! The parser walks `pulldown-cmark`'s event stream and builds the semantic IR.
//! It never produces Typst, and it never parses mathematics: an equation is
//! captured as its original LaTeX source plus a span, and translated later.
//!
//! Two stacks drive the walk. Block containers (the document, a blockquote, a
//! list item) collect [`Block`]s; inline containers (a paragraph, a heading,
//! emphasis, a link, a table cell) collect [`Inline`]s. Every `Start` event
//! pushes a container and the matching `End` pops it and attaches the result to
//! its parent, so nesting needs no special cases.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser as Cmark, Tag, TagEnd};

use crate::diagnostics::{Diagnostic, SourceSpan};
use crate::ir::{
    Alignment, Block, CodeBlock, Image, Inline, ListItem, MathMode, MathSource, Table,
};
use crate::metadata::DocumentType;

/// The result of parsing a document body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParseOutcome {
    pub blocks: Vec<Block>,
    /// Non-fatal problems, such as raw HTML. Escalated to errors under
    /// `--strict` by the collector that receives them.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse a document body into blocks.
///
/// `base_offset` is the position of `body` within the original file, as reported
/// by the front-matter pre-pass. Every span is shifted by it so that diagnostics
/// point at the file the user wrote.
///
/// `document_type` decides only one thing: whether a thematic break is a
/// [`Block::Rule`] or a [`Block::SlideBreak`]. Nothing else about parsing
/// depends on the output mode.
pub fn parse(body: &str, base_offset: usize, document_type: DocumentType) -> ParseOutcome {
    Builder::new(base_offset, document_type).run(body)
}

/// The Markdown extensions this dialect enables.
fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_MATH);
    options
}

/// A partially built list.
struct ListFrame {
    /// `Some(start)` for an ordered list.
    start: Option<u64>,
    items: Vec<ListItem>,
}

/// A partially built table.
struct TableFrame {
    alignments: Vec<Alignment>,
    header: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    /// Cells accumulate here until the row ends.
    row: Vec<Vec<Inline>>,
    in_header: bool,
}

/// A partially built heading.
struct HeadingFrame {
    level: u8,
    span: SourceSpan,
}

/// A partially built image. The alt text arrives as inline events.
struct ImageFrame {
    destination: String,
    title: Option<String>,
    span: SourceSpan,
}

struct Builder {
    base: usize,
    document_type: DocumentType,
    /// Stack of block containers; the first is the document itself.
    blocks: Vec<Vec<Block>>,
    /// Stack of inline containers. Empty while between blocks.
    inlines: Vec<Vec<Inline>>,
    lists: Vec<ListFrame>,
    tables: Vec<TableFrame>,
    headings: Vec<HeadingFrame>,
    images: Vec<ImageFrame>,
    links: Vec<String>,
    /// Set while inside a code block; text events append to it.
    code: Option<CodeBlock>,
    diagnostics: Vec<Diagnostic>,
    /// Suppresses repeated raw-HTML warnings for one contiguous HTML block.
    in_html_block: bool,
    /// Whether the innermost inline container was opened implicitly.
    ///
    /// A *tight* list item emits inline events with no enclosing paragraph, so
    /// the builder opens one on demand and closes it when the item, or any
    /// block-level construct inside it, ends.
    implicit_paragraph: bool,
}

impl Builder {
    fn new(base: usize, document_type: DocumentType) -> Self {
        Self {
            base,
            document_type,
            blocks: vec![Vec::new()],
            inlines: Vec::new(),
            lists: Vec::new(),
            tables: Vec::new(),
            headings: Vec::new(),
            images: Vec::new(),
            links: Vec::new(),
            code: None,
            diagnostics: Vec::new(),
            in_html_block: false,
            implicit_paragraph: false,
        }
    }

    fn run(mut self, body: &str) -> ParseOutcome {
        for (event, range) in Cmark::new_ext(body, options()).into_offset_iter() {
            let span = self.span(range.start, range.end);
            self.event(event, span);
        }

        let blocks = self.blocks.pop().unwrap_or_default();
        debug_assert!(self.blocks.is_empty(), "unbalanced block containers");
        ParseOutcome {
            blocks,
            diagnostics: self.diagnostics,
        }
    }

    /// A span in the original file, shifted past any front matter.
    fn span(&self, start: usize, end: usize) -> SourceSpan {
        SourceSpan::new(self.base + start, self.base + end)
    }

    fn event(&mut self, event: Event<'_>, span: SourceSpan) {
        match event {
            Event::Start(tag) => self.start(tag, span),
            Event::End(tag) => self.end(tag, span),

            Event::Text(text) => match &mut self.code {
                Some(code) => code.source.push_str(&text),
                None => self.push_inline(Inline::Text(text.into_string())),
            },

            Event::Code(code) => self.push_inline(Inline::code(code.into_string())),

            Event::InlineMath(source) => {
                // The event range covers `$…$`, so the content starts one byte in.
                let span = SourceSpan::new(span.start + 1, span.end.saturating_sub(1));
                self.push_inline(Inline::Math(MathSource::new(
                    source,
                    MathMode::Inline,
                    span,
                )));
            }

            Event::DisplayMath(source) => {
                // The event range covers `$$…$$`.
                let inner = SourceSpan::new(span.start + 2, span.end.saturating_sub(2));
                let (source, span) = trim_math(&source, inner);
                self.push_inline(Inline::Math(MathSource::new(
                    source,
                    MathMode::Display,
                    span,
                )));
            }

            Event::SoftBreak => self.push_inline(Inline::SoftBreak),
            Event::HardBreak => self.push_inline(Inline::HardBreak),

            Event::Rule => {
                self.flush_implicit_paragraph();
                let rule = match self.document_type {
                    DocumentType::Document => Block::Rule,
                    DocumentType::Slides => Block::SlideBreak,
                };
                self.push_block(rule);
            }

            // Raw HTML is unsupported and never silently reinterpreted.
            Event::Html(_) => {
                if !self.in_html_block {
                    self.warn_html(span);
                    self.in_html_block = true;
                }
            }
            Event::InlineHtml(_) => self.warn_html(span),

            // Not enabled in this dialect, so these cannot occur.
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>, span: SourceSpan) {
        if is_block_tag(&tag) {
            self.flush_implicit_paragraph();
        }
        match tag {
            Tag::Paragraph => self.inlines.push(Vec::new()),

            Tag::Heading { level, .. } => {
                self.headings.push(HeadingFrame {
                    level: heading_level(level),
                    span,
                });
                self.inlines.push(Vec::new());
            }

            Tag::BlockQuote(_) => self.blocks.push(Vec::new()),

            Tag::List(start) => {
                self.lists.push(ListFrame {
                    start,
                    items: Vec::new(),
                });
            }
            Tag::Item => self.blocks.push(Vec::new()),

            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => language_of(&info),
                    CodeBlockKind::Indented => None,
                };
                self.code = Some(CodeBlock {
                    language,
                    source: String::new(),
                });
            }

            Tag::Table(alignments) => {
                self.tables.push(TableFrame {
                    alignments: alignments.iter().copied().map(alignment).collect(),
                    header: Vec::new(),
                    rows: Vec::new(),
                    row: Vec::new(),
                    in_header: false,
                });
            }
            Tag::TableHead => {
                if let Some(table) = self.tables.last_mut() {
                    table.in_header = true;
                }
            }
            Tag::TableRow => {}
            Tag::TableCell => self.inlines.push(Vec::new()),

            Tag::Emphasis | Tag::Strong | Tag::Strikethrough => self.inlines.push(Vec::new()),

            Tag::Link { dest_url, .. } => {
                self.links.push(dest_url.into_string());
                self.inlines.push(Vec::new());
            }

            Tag::Image {
                dest_url, title, ..
            } => {
                let title = (!title.is_empty()).then(|| title.into_string());
                self.images.push(ImageFrame {
                    destination: dest_url.into_string(),
                    title,
                    span,
                });
                self.inlines.push(Vec::new());
            }

            Tag::HtmlBlock => self.in_html_block = false,

            // Not enabled in this dialect.
            Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::MetadataBlock(_)
            | Tag::Superscript
            | Tag::Subscript => {}
        }
    }

    fn end(&mut self, tag: TagEnd, _span: SourceSpan) {
        match tag {
            TagEnd::Paragraph => {
                let content = self.pop_inlines();
                // A paragraph holding nothing but one display equation, or one
                // image, is really a block of its own.
                match hoist(content) {
                    Hoisted::Block(block) => self.push_block(block),
                    Hoisted::Paragraph(content) if content.is_empty() => {}
                    Hoisted::Paragraph(content) => self.push_block(Block::Paragraph(content)),
                }
            }

            TagEnd::Heading(_) => {
                let content = self.pop_inlines();
                if let Some(HeadingFrame { level, span }) = self.headings.pop() {
                    self.push_block(Block::Heading {
                        level,
                        content,
                        span,
                    });
                }
            }

            TagEnd::BlockQuote(_) => {
                self.flush_implicit_paragraph();
                let inner = self.blocks.pop().unwrap_or_default();
                self.push_block(Block::BlockQuote(inner));
            }

            TagEnd::Item => {
                self.flush_implicit_paragraph();
                let blocks = self.blocks.pop().unwrap_or_default();
                if let Some(list) = self.lists.last_mut() {
                    list.items.push(ListItem::new(blocks));
                }
            }

            TagEnd::List(_) => {
                if let Some(ListFrame { start, items }) = self.lists.pop() {
                    let list = match start {
                        Some(start) => Block::OrderedList { start, items },
                        None => Block::UnorderedList(items),
                    };
                    self.push_block(list);
                }
            }

            TagEnd::CodeBlock => {
                if let Some(code) = self.code.take() {
                    self.push_block(Block::CodeBlock(code));
                }
            }

            TagEnd::TableCell => {
                let cell = self.pop_inlines();
                if let Some(table) = self.tables.last_mut() {
                    table.row.push(cell);
                }
            }
            TagEnd::TableHead => {
                if let Some(table) = self.tables.last_mut() {
                    table.header = std::mem::take(&mut table.row);
                    table.in_header = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = self.tables.last_mut() {
                    let row = std::mem::take(&mut table.row);
                    if table.in_header {
                        table.header = row;
                    } else {
                        table.rows.push(row);
                    }
                }
            }
            TagEnd::Table => {
                if let Some(frame) = self.tables.pop() {
                    self.push_block(Block::Table(Table {
                        alignments: frame.alignments,
                        header: frame.header,
                        rows: frame.rows,
                    }));
                }
            }

            TagEnd::Emphasis => {
                let content = self.pop_inlines();
                self.push_inline(Inline::Emphasis(content));
            }
            TagEnd::Strong => {
                let content = self.pop_inlines();
                self.push_inline(Inline::Strong(content));
            }
            TagEnd::Strikethrough => {
                let content = self.pop_inlines();
                self.push_inline(Inline::Strike(content));
            }

            TagEnd::Link => {
                let content = self.pop_inlines();
                let destination = self.links.pop().unwrap_or_default();
                self.push_inline(Inline::Link {
                    destination,
                    content,
                });
            }

            TagEnd::Image => {
                let alt = self.pop_inlines();
                if let Some(ImageFrame {
                    destination,
                    title,
                    span,
                }) = self.images.pop()
                {
                    self.push_inline(Inline::Image(Image {
                        destination,
                        alt,
                        title,
                        span,
                    }));
                }
            }

            TagEnd::HtmlBlock => self.in_html_block = false,

            TagEnd::FootnoteDefinition
            | TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition
            | TagEnd::MetadataBlock(_)
            | TagEnd::Superscript
            | TagEnd::Subscript => {}
        }
    }

    /// Append a block to the innermost block container.
    fn push_block(&mut self, block: Block) {
        if let Some(container) = self.blocks.last_mut() {
            container.push(block);
        }
    }

    /// Append an inline to the innermost inline container, opening an implicit
    /// paragraph if the event stream gave us none — which is what a tight list
    /// item does.
    fn push_inline(&mut self, inline: Inline) {
        if self.inlines.is_empty() {
            self.inlines.push(Vec::new());
            self.implicit_paragraph = true;
        }
        if let Some(container) = self.inlines.last_mut() {
            container.push(inline);
        }
    }

    /// Close an implicitly opened paragraph, if one is open.
    ///
    /// Called before any block-level construct and when a container ends, so
    /// that `- text\n  - nested` yields a paragraph followed by a list rather
    /// than one swallowing the other.
    fn flush_implicit_paragraph(&mut self) {
        if !self.implicit_paragraph {
            return;
        }
        self.implicit_paragraph = false;
        let content = self.pop_inlines();
        match hoist(content) {
            Hoisted::Block(block) => self.push_block(block),
            Hoisted::Paragraph(content) if content.is_empty() => {}
            Hoisted::Paragraph(content) => self.push_block(Block::Paragraph(content)),
        }
    }

    fn pop_inlines(&mut self) -> Vec<Inline> {
        self.inlines.pop().unwrap_or_default()
    }

    fn warn_html(&mut self, span: SourceSpan) {
        self.diagnostics
            .push(Diagnostic::warning("raw HTML is not supported").with_span(span));
    }
}

/// Whether a tag opens a block-level construct, which must close any implicitly
/// opened paragraph first.
fn is_block_tag(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::CodeBlock(_)
            | Tag::Table(_)
            | Tag::HtmlBlock
    )
}

/// What a finished paragraph turned out to be.
enum Hoisted {
    Block(Block),
    Paragraph(Vec<Inline>),
}

/// Promote a paragraph that holds a single display equation or a single image
/// into a block of its own.
///
/// `pulldown-cmark` reports both as inline events inside a paragraph, but
/// semantically a standalone equation or figure is a block, and the renderers
/// need to treat it as one.
fn hoist(mut content: Vec<Inline>) -> Hoisted {
    // Blank inlines are ignored when deciding, but never removed from a
    // paragraph that stays a paragraph: a soft break is meaningful content.
    let mut visible = content
        .iter()
        .enumerate()
        .filter(|(_, inline)| !is_blank(inline));
    let single = match (visible.next(), visible.next()) {
        (None, _) => return Hoisted::Paragraph(Vec::new()),
        (Some((index, inline)), None) => Some((index, inline)),
        _ => None,
    };

    if let Some((index, inline)) = single {
        let hoistable = match inline {
            Inline::Math(math) => math.mode.is_display(),
            Inline::Image(_) => true,
            _ => false,
        };
        if hoistable {
            return match content.swap_remove(index) {
                Inline::Math(math) => Hoisted::Block(Block::Math(math)),
                Inline::Image(image) => Hoisted::Block(Block::Image(image)),
                _ => unreachable!("only math and images are hoisted"),
            };
        }
    }

    Hoisted::Paragraph(content)
}

/// Whether an inline carries no visible content, so that surrounding whitespace
/// does not prevent hoisting.
fn is_blank(inline: &Inline) -> bool {
    match inline {
        Inline::Text(text) => text.trim().is_empty(),
        Inline::SoftBreak => true,
        _ => false,
    }
}

/// Trim surrounding whitespace from display mathematics, keeping the span in
/// step so it still points at the mathematics itself.
fn trim_math(source: &str, span: SourceSpan) -> (String, SourceSpan) {
    let leading = source.len() - source.trim_start().len();
    let trimmed = source.trim();
    let start = span.start + leading;
    (
        trimmed.to_string(),
        SourceSpan::new(start, start + trimmed.len()),
    )
}

/// The language of a fenced code block, taken as the first word of the info
/// string. An empty info string means no language.
fn language_of(info: &str) -> Option<String> {
    info.split_whitespace()
        .next()
        .filter(|word| !word.is_empty())
        .map(str::to_string)
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn alignment(alignment: pulldown_cmark::Alignment) -> Alignment {
    match alignment {
        pulldown_cmark::Alignment::None => Alignment::None,
        pulldown_cmark::Alignment::Left => Alignment::Left,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        pulldown_cmark::Alignment::Right => Alignment::Right,
    }
}
