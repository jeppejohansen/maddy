//! IR to Typst markup.
//!
//! This renders the parts of a document that look the same whether the output is
//! a paper or a slide: paragraphs, headings, lists, equations, tables, code and
//! quotations. The document and presentation emitters wrap it in their own
//! structure and hand the result to a theme.
//!
//! Two invariants hold throughout:
//!
//! * all text goes through [`escape`], so Markdown can never become Typst
//!   syntax;
//! * the output is semantic. There is no theme-specific formatting here — a
//!   template decides how a heading or a table *looks*.

use crate::diagnostics::{CompileError, Diagnostic, Diagnostics, Result, SourceSpan};
use crate::ir::{Alignment, Block, CodeBlock, Image, Inline, ListItem, MathSource, Table};
use crate::math::{self, MathError};

use super::escape;

/// Renders IR into Typst markup.
pub struct Renderer<'a> {
    diagnostics: &'a mut Diagnostics,
    /// Indentation of the current block container, in spaces.
    indent: usize,
}

impl<'a> Renderer<'a> {
    pub fn new(diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            diagnostics,
            indent: 0,
        }
    }

    /// Render a sequence of blocks, separated by blank lines.
    pub fn blocks(&mut self, blocks: &[Block]) -> Result<String> {
        let mut parts = Vec::new();
        for block in blocks {
            let rendered = self.block(block)?;
            if !rendered.trim().is_empty() {
                parts.push(rendered);
            }
        }
        Ok(parts.join("\n\n"))
    }

    fn block(&mut self, block: &Block) -> Result<String> {
        match block {
            Block::Paragraph(content) => self.line(content),

            Block::Heading { level, content, .. } => {
                let marker = "=".repeat((*level).clamp(1, 6) as usize);
                Ok(format!(
                    "{}{marker} {}",
                    self.pad(),
                    self.inlines(content, true)?
                ))
            }

            Block::Math(math) => {
                // Spaces inside the delimiters are what make Typst set an
                // equation as a display block rather than inline.
                Ok(format!("{}$ {} $", self.pad(), self.equation(math)?))
            }

            Block::CodeBlock(code) => Ok(format!("{}{}", self.pad(), raw_block(code))),

            Block::BlockQuote(blocks) => {
                let inner = self.nested(blocks)?;
                Ok(format!(
                    "{}#quote(block: true)[\n{inner}\n{}]",
                    self.pad(),
                    self.pad()
                ))
            }

            Block::UnorderedList(items) => self.list(items, "-", None),

            Block::OrderedList { start, items } => self.list(items, "+", Some(*start)),

            Block::Table(table) => self.table(table),

            Block::Image(image) => Ok(format!("{}{}", self.pad(), self.image(image)?)),

            Block::Rule => Ok(format!("{}#line(length: 100%)", self.pad())),

            // Segmentation removes these before a document is rendered; a stray
            // one is a thematic break.
            Block::SlideBreak => Ok(format!("{}#line(length: 100%)", self.pad())),
        }
    }

    /// A paragraph: one line of markup, with line-start constructs guarded.
    fn line(&mut self, content: &[Inline]) -> Result<String> {
        Ok(format!("{}{}", self.pad(), self.inlines(content, true)?))
    }

    /// Render inline content.
    ///
    /// `at_line_start` guards the first text run against being read as a list,
    /// heading or enumeration marker.
    pub fn inlines(&mut self, content: &[Inline], at_line_start: bool) -> Result<String> {
        let mut out = String::new();
        let mut first = at_line_start;

        for inline in content {
            out.push_str(&self.inline(inline, first)?);
            // Only a leading run can begin a line; everything after is mid-line.
            if !matches!(inline, Inline::SoftBreak) {
                first = false;
            }
        }

        Ok(out)
    }

    fn inline(&mut self, inline: &Inline, at_line_start: bool) -> Result<String> {
        Ok(match inline {
            Inline::Text(text) => {
                if at_line_start {
                    escape::markup_at_line_start(text)
                } else {
                    escape::markup(text)
                }
            }

            Inline::Emphasis(content) => format!("_{}_", self.inlines(content, false)?),
            Inline::Strong(content) => format!("*{}*", self.inlines(content, false)?),
            Inline::Strike(content) => format!("#strike[{}]", self.inlines(content, false)?),

            // The function form takes any content, including backticks.
            Inline::Code(code) => format!("#raw({})", escape::string_literal(code)),

            Inline::Link {
                destination,
                content,
            } => format!(
                "#link({})[{}]",
                escape::string_literal(destination),
                self.inlines(content, false)?
            ),

            Inline::Image(image) => format!("#box({})", self.image(image)?),

            Inline::Math(math) => format!("${}$", self.equation(math)?),

            // A soft break is a space: a paragraph is emitted on one line, so
            // that only its first character can start a line.
            Inline::SoftBreak => " ".to_string(),

            // The function form keeps the paragraph on one line, where a
            // trailing backslash would not.
            Inline::HardBreak => "#linebreak()".to_string(),
        })
    }

    /// Translate an equation, turning a failure into a diagnostic that points at
    /// the Markdown source.
    fn equation(&mut self, math: &MathSource) -> Result<String> {
        math::source_to_typst(math).map_err(|error| math_error(math, error))
    }

    fn list(&mut self, items: &[ListItem], marker: &str, start: Option<u64>) -> Result<String> {
        let pad = self.pad();
        let mut lines = Vec::with_capacity(items.len());

        for item in items {
            // The first block shares the marker's line; the rest are indented
            // under it, which is how Typst continues a list item.
            let body = self.nested(&item.blocks)?;
            let body = body
                .strip_prefix(&" ".repeat(self.indent + 2))
                .unwrap_or(&body);
            lines.push(format!("{pad}{marker} {body}"));
        }

        let list = lines.join("\n");

        // A list that does not start at one needs a scoped set rule, since the
        // markup form carries no starting number.
        match start {
            Some(start) if start != 1 => Ok(format!(
                "{pad}#[\n{pad}  #set enum(start: {start})\n{}\n{pad}]",
                indent_by(&list, 2)
            )),
            _ => Ok(list),
        }
    }

    /// Render blocks one level deeper.
    fn nested(&mut self, blocks: &[Block]) -> Result<String> {
        self.indent += 2;
        let rendered = self.blocks(blocks);
        self.indent -= 2;
        rendered
    }

    fn table(&mut self, table: &Table) -> Result<String> {
        let pad = self.pad();
        let columns = table.columns();
        let alignments: Vec<&str> = (0..columns)
            .map(|index| alignment(table.alignment(index)))
            .collect();

        let mut out = format!("{pad}#table(\n{pad}  columns: {columns},\n");
        out.push_str(&format!("{pad}  align: ({}),\n", alignments.join(", ")));

        if table.has_header() {
            out.push_str(&format!(
                "{pad}  table.header({}),\n",
                self.cells(&table.header, columns)?
            ));
        }

        for row in &table.rows {
            out.push_str(&format!("{pad}  {},\n", self.cells(row, columns)?));
        }

        out.push_str(&format!("{pad})"));
        Ok(out)
    }

    /// Render one row's cells, padding a ragged row to the table's width.
    fn cells(&mut self, row: &[Vec<Inline>], columns: usize) -> Result<String> {
        let mut cells = Vec::with_capacity(columns);
        for index in 0..columns {
            let content = match row.get(index) {
                Some(cell) => self.inlines(cell, false)?,
                None => String::new(),
            };
            cells.push(format!("[{content}]"));
        }
        Ok(cells.join(", "))
    }

    fn image(&mut self, image: &Image) -> Result<String> {
        // Remote images are not fetched: v0.1 does not touch the network.
        if image.is_remote() {
            self.diagnostics.push(
                Diagnostic::warning("remote images are not supported")
                    .with_span(image.span)
                    .with_note("Download the image and reference it by a local path."),
            );
            return Ok("#box[]".to_string());
        }

        let mut arguments = vec![escape::string_literal(&image.destination)];

        // Alt text becomes PDF metadata rather than a visible caption.
        let alt = crate::ir::plain_text(&image.alt);
        if !alt.trim().is_empty() {
            arguments.push(format!("alt: {}", escape::string_literal(&alt)));
        }

        Ok(format!("image({})", arguments.join(", ")))
    }

    fn pad(&self) -> String {
        " ".repeat(self.indent)
    }
}

/// A code block, as a Typst `raw` call.
///
/// The function form is used rather than a fenced block because an info string
/// such as `rust,ignore` silently corrupts a fence, and because the source may
/// contain any run of backticks.
fn raw_block(code: &CodeBlock) -> String {
    let mut arguments = vec!["block: true".to_string()];
    if let Some(language) = &code.language {
        arguments.push(format!("lang: {}", escape::string_literal(language)));
    }
    arguments.push(escape::string_literal(code.source.trim_end_matches('\n')));
    format!("#raw({})", arguments.join(", "))
}

/// The Typst alignment for a Markdown column alignment.
fn alignment(alignment: Alignment) -> &'static str {
    match alignment {
        // Unspecified: the template decides, so this defers to Typst's default.
        Alignment::None => "auto",
        Alignment::Left => "left",
        Alignment::Center => "center",
        Alignment::Right => "right",
    }
}

/// Indent every non-empty line of `text` by `spaces`.
fn indent_by(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Turn a mathematics failure into a compiler error pointing at the Markdown.
///
/// A parse error reports an offset within the equation; shifting it onto the
/// equation's own span is what makes the caret land on the right column of the
/// original file.
fn math_error(math: &MathSource, error: MathError) -> CompileError {
    let span = match error.offset() {
        Some(offset) => {
            SourceSpan::new((math.span.start + offset).min(math.span.end), math.span.end)
        }
        None => math.span,
    };

    let mut diagnostic = Diagnostic::error(error.to_string()).with_span(span);
    if let Some(note) = error.note() {
        diagnostic = diagnostic.with_note(note);
    }

    match error {
        MathError::Parse { .. } => CompileError::MathParse(diagnostic),
        MathError::Unsupported { .. } => CompileError::MathEmit(diagnostic),
    }
}
