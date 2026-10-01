//! Source spans, diagnostic messages and the compiler's error taxonomy.
//!
//! Diagnostics are a first-class feature: wherever a position is available the
//! message must point back at the *Markdown* source rather than at generated
//! Typst. Spans are byte ranges into the original UTF-8 input, and line and
//! column numbers are computed only when a diagnostic is rendered.

use std::fmt;

/// A byte range into the original UTF-8 source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// An empty span pointing at a single offset.
    pub const fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// The smallest span covering both `self` and `other`.
    pub fn merge(&self, other: &SourceSpan) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Shift the span forward by `offset` bytes.
    ///
    /// This is how an offset reported by the math parser — which only ever sees
    /// the extracted equation substring — is translated back into a position in
    /// the enclosing Markdown file.
    pub fn shift(&self, offset: usize) -> Self {
        Self {
            start: self.start + offset,
            end: self.end + offset,
        }
    }

    /// Clamp the span to lie within `len` bytes, so that a bad span can never
    /// panic the diagnostic renderer.
    pub fn clamp(&self, len: usize) -> Self {
        let start = self.start.min(len);
        Self {
            start,
            end: self.end.clamp(start, len),
        }
    }
}

/// One-based line and column of a byte offset, for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

/// Resolve a byte offset into a one-based line and column.
///
/// The column counts characters, not bytes, so that non-ASCII text produces a
/// caret in the right visual place.
pub fn line_column(source: &str, offset: usize) -> LineColumn {
    let offset = offset.min(source.len());
    let mut line = 1;
    let mut line_start = 0;

    for (index, byte) in source.as_bytes()[..offset].iter().enumerate() {
        if *byte == b'\n' {
            line += 1;
            line_start = index + 1;
        }
    }

    let column = source[line_start..offset].chars().count() + 1;
    LineColumn { line, column }
}

/// Extract the full text of the line containing `offset`, without its newline.
pub fn line_text(source: &str, offset: usize) -> &str {
    let offset = offset.min(source.len());
    let start = source[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = source[offset..]
        .find('\n')
        .map_or(source.len(), |i| offset + i);
    source[start..end].trim_end_matches('\r')
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Note,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Severity::Note => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        f.write_str(text)
    }
}

/// A single diagnostic message, optionally anchored to a source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    /// Extra lines printed below the snippet, such as `Parsed as: CD environment`.
    pub notes: Vec<String>,
}

impl fmt::Display for Diagnostic {
    /// The bare message, without a source snippet.
    ///
    /// Use [`Diagnostic::render`] when the source text is available.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.message)
    }
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            span: None,
            notes: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn note(message: impl Into<String>) -> Self {
        Self::new(Severity::Note, message)
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Re-label this diagnostic as an error, used by `--strict`.
    pub fn escalate(mut self) -> Self {
        if self.severity == Severity::Warning {
            self.severity = Severity::Error;
        }
        self
    }

    /// Render the diagnostic as a source-annotated snippet.
    ///
    /// ```text
    /// warning: raw HTML is not supported
    ///   --> paper.md:42:1
    ///
    /// 42 | <div>hello</div>
    ///    | ^^^^^^^^^^^^^^^^
    /// ```
    pub fn render(&self, path: &str, source: &str) -> String {
        let mut out = format!("{}: {}", self.severity, self.message);

        if let Some(span) = self.span.map(|s| s.clamp(source.len())) {
            let LineColumn { line, column } = line_column(source, span.start);
            let text = line_text(source, span.start);
            let gutter = line.to_string();
            let pad = " ".repeat(gutter.len());

            // The caret run covers the span, clipped to the first line.
            let visible = text.chars().count().saturating_sub(column - 1);
            let width = source
                .get(span.start..span.end)
                .map_or(1, |s| s.chars().count())
                .clamp(1, visible.max(1));

            out.push_str(&format!("\n{pad} --> {path}:{line}:{column}\n"));
            out.push_str(&format!("{pad} |\n"));
            out.push_str(&format!("{gutter} | {text}\n"));
            out.push_str(&format!(
                "{pad} | {}{}",
                " ".repeat(column - 1),
                "^".repeat(width)
            ));
        }

        for note in &self.notes {
            out.push_str(&format!("\n\n{note}"));
        }

        out
    }
}

/// Collects diagnostics during a compilation.
///
/// Under `--strict` every warning is escalated to an error as it is recorded,
/// so the rest of the compiler never needs to know about the flag.
#[derive(Debug, Default)]
pub struct Diagnostics {
    entries: Vec<Diagnostic>,
    strict: bool,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strict() -> Self {
        Self {
            entries: Vec::new(),
            strict: true,
        }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        let diagnostic = if self.strict {
            diagnostic.escalate()
        } else {
            diagnostic
        };
        self.entries.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        for diagnostic in diagnostics {
            self.push(diagnostic);
        }
    }

    pub fn entries(&self) -> &[Diagnostic] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether any recorded diagnostic is an error.
    pub fn has_errors(&self) -> bool {
        self.entries.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.entries
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    pub fn into_entries(self) -> Vec<Diagnostic> {
        self.entries
    }
}

/// The compiler's error taxonomy.
///
/// Every variant maps to an exit code via [`CompileError::exit_code`], and
/// carries a source span wherever the underlying failure had a position, so the
/// top-level handler can render a snippet instead of a bare message.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("{0}")]
    Markdown(Diagnostic),

    #[error("invalid front matter: {message}")]
    FrontMatter {
        message: String,
        span: Option<SourceSpan>,
    },

    #[error("{0}")]
    MathParse(Diagnostic),

    #[error("{0}")]
    MathEmit(Diagnostic),

    #[error("{0}")]
    UnsupportedFeature(Diagnostic),

    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("generated Typst failed to compile")]
    TypstCompile { details: String },

    #[error("the Typst executable was not found")]
    TypstMissing,

    #[error("internal compiler error: {0}")]
    Internal(String),

    /// One or more diagnostics were fatal (including escalated warnings).
    #[error("compilation produced {} error(s)", .0.iter().filter(|d| d.severity == Severity::Error).count())]
    Diagnostics(Vec<Diagnostic>),
}

impl CompileError {
    /// The underlying tool output, when there is more of it than a diagnostic
    /// shows. Printed by `--verbose`.
    pub fn full_details(&self) -> Option<&str> {
        match self {
            CompileError::TypstCompile { details } => Some(details),
            _ => None,
        }
    }

    /// Exit code for this error, per the specification:
    ///
    /// ```text
    /// 0    success
    /// 1    document compilation error
    /// 2    invalid CLI/configuration
    /// 3    I/O or environment error
    /// ```
    pub fn exit_code(&self) -> u8 {
        match self {
            CompileError::Markdown(_)
            | CompileError::FrontMatter { .. }
            | CompileError::MathParse(_)
            | CompileError::MathEmit(_)
            | CompileError::UnsupportedFeature(_)
            | CompileError::TypstCompile { .. }
            | CompileError::Diagnostics(_)
            | CompileError::Internal(_) => 1,
            CompileError::Config(_) => 2,
            CompileError::Io { .. } | CompileError::TypstMissing => 3,
        }
    }

    /// The diagnostics this error should be rendered as.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            CompileError::Markdown(d)
            | CompileError::MathParse(d)
            | CompileError::MathEmit(d)
            | CompileError::UnsupportedFeature(d) => vec![d.clone()],
            CompileError::FrontMatter { message, span } => {
                let mut d = Diagnostic::error(format!("invalid front matter: {message}"));
                d.span = *span;
                vec![d]
            }
            CompileError::Diagnostics(entries) => entries.clone(),
            CompileError::TypstCompile { details } => {
                vec![Diagnostic::error("generated Typst failed to compile")
                    .with_note(format!("Caused by:\n{}", indent(&truncate(details, 8), 4)))
                    .with_note("Run with --keep-typst to inspect generated source.")]
            }
            CompileError::TypstMissing => vec![Diagnostic::error("Typst executable was not found")
                .with_note("Install Typst or run:\n\n    maddy --emit typst document.md")],
            other => vec![Diagnostic::error(other.to_string())],
        }
    }
}

/// Keep the first `lines` lines, noting how many were dropped.
///
/// A failure in generated Typst usually means a bug in the emitter or an invalid
/// user template, and the first few lines say which. Pages of subprocess output
/// bury that, so the rest is available through `-v` instead.
fn truncate(text: &str, lines: usize) -> String {
    let text = text.trim_end();
    let total = text.lines().count();
    if total <= lines {
        return text.to_string();
    }

    let kept: Vec<&str> = text.lines().take(lines).collect();
    format!(
        "{}\n... {} more line(s); run with --verbose for the full output",
        kept.join("\n"),
        total - lines
    )
}

/// Indent every line of `text` by `spaces`.
fn indent(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convenience for attaching context to an I/O failure.
pub fn io_error(context: impl Into<String>) -> impl FnOnce(std::io::Error) -> CompileError {
    move |source| CompileError::Io {
        context: context.into(),
        source,
    }
}

pub type Result<T> = std::result::Result<T, CompileError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_reports_length_and_emptiness() {
        assert_eq!(SourceSpan::new(3, 9).len(), 6);
        assert!(!SourceSpan::new(3, 9).is_empty());
        assert!(SourceSpan::at(4).is_empty());
        assert_eq!(SourceSpan::at(4).len(), 0);
        // A reversed span is degenerate rather than a panic.
        assert_eq!(SourceSpan::new(9, 3).len(), 0);
    }

    #[test]
    fn span_merge_covers_both_operands() {
        let merged = SourceSpan::new(10, 12).merge(&SourceSpan::new(2, 4));
        assert_eq!(merged, SourceSpan::new(2, 12));
    }

    #[test]
    fn span_shift_translates_math_offsets_into_the_document() {
        // The math parser reports 0..4 inside "\foo"; the equation began at
        // byte 17 of the Markdown file.
        assert_eq!(SourceSpan::new(0, 4).shift(17), SourceSpan::new(17, 21));
    }

    #[test]
    fn span_clamp_keeps_spans_inside_the_source() {
        assert_eq!(SourceSpan::new(2, 99).clamp(10), SourceSpan::new(2, 10));
        assert_eq!(SourceSpan::new(50, 99).clamp(10), SourceSpan::new(10, 10));
    }

    #[test]
    fn line_column_is_one_based() {
        let source = "alpha\nbeta\ngamma";
        assert_eq!(line_column(source, 0), LineColumn { line: 1, column: 1 });
        assert_eq!(line_column(source, 4), LineColumn { line: 1, column: 5 });
        // The newline itself still belongs to the line it terminates.
        assert_eq!(line_column(source, 5), LineColumn { line: 1, column: 6 });
        assert_eq!(line_column(source, 6), LineColumn { line: 2, column: 1 });
        assert_eq!(line_column(source, 11), LineColumn { line: 3, column: 1 });
    }

    #[test]
    fn line_column_counts_characters_not_bytes() {
        let source = "π≈3 here";
        // "π≈3 " is 4 characters but 7 bytes.
        assert_eq!(line_column(source, 7), LineColumn { line: 1, column: 5 });
    }

    #[test]
    fn line_column_clamps_an_out_of_range_offset() {
        assert_eq!(line_column("ab", 99), LineColumn { line: 1, column: 3 });
    }

    #[test]
    fn line_text_extracts_the_containing_line() {
        let source = "first\nsecond\nthird";
        assert_eq!(line_text(source, 0), "first");
        assert_eq!(line_text(source, 8), "second");
        assert_eq!(line_text(source, 14), "third");
        assert_eq!(line_text("solo", 2), "solo");
    }

    #[test]
    fn line_text_strips_carriage_returns() {
        assert_eq!(line_text("win\r\nnext", 1), "win");
    }

    #[test]
    fn severity_orders_note_below_error() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Note);
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Note.to_string(), "note");
    }

    #[test]
    fn diagnostic_renders_a_source_snippet() {
        let source = "line one\nWe have $\\foo{x}$ here.\n";
        let span = SourceSpan::new(17, 24);
        let rendered = Diagnostic::error("unsupported math command `\\foo`")
            .with_span(span)
            .render("paper.md", source);

        assert_eq!(
            rendered,
            "error: unsupported math command `\\foo`\n\
             \x20 --> paper.md:2:9\n\
             \x20 |\n\
             2 | We have $\\foo{x}$ here.\n\
             \x20 |         ^^^^^^^"
        );
    }

    #[test]
    fn diagnostic_without_a_span_renders_only_the_message() {
        let rendered = Diagnostic::warning("raw HTML is not supported").render("x.md", "hi");
        assert_eq!(rendered, "warning: raw HTML is not supported");
    }

    #[test]
    fn diagnostic_renders_trailing_notes() {
        let rendered = Diagnostic::error("unsupported construct")
            .with_note("Parsed as: CD environment")
            .render("x.md", "hi");
        assert!(
            rendered.ends_with("\n\nParsed as: CD environment"),
            "{rendered}"
        );
    }

    #[test]
    fn diagnostic_caret_never_runs_past_the_line() {
        // A span that overruns the line (and the source) must still render.
        let rendered = Diagnostic::error("bad")
            .with_span(SourceSpan::new(2, 400))
            .render("x.md", "ab\ncd");
        assert!(rendered.contains("1 | ab\n"), "{rendered}");
        assert!(rendered.trim_end().ends_with('^'), "{rendered}");
    }

    #[test]
    fn escalate_promotes_warnings_only() {
        assert_eq!(
            Diagnostic::warning("w").escalate().severity,
            Severity::Error
        );
        assert_eq!(Diagnostic::note("n").escalate().severity, Severity::Note);
        assert_eq!(Diagnostic::error("e").escalate().severity, Severity::Error);
    }

    #[test]
    fn diagnostic_displays_as_severity_and_message() {
        assert_eq!(
            Diagnostic::warning("raw HTML is not supported").to_string(),
            "warning: raw HTML is not supported"
        );
    }

    #[test]
    fn collector_records_in_order() {
        let mut diagnostics = Diagnostics::new();
        assert!(diagnostics.is_empty());

        diagnostics.push(Diagnostic::warning("first"));
        diagnostics.extend([Diagnostic::note("second"), Diagnostic::error("third")]);

        assert_eq!(diagnostics.len(), 3);
        assert_eq!(diagnostics.entries()[0].message, "first");
        assert_eq!(diagnostics.entries()[2].message, "third");
        assert!(diagnostics.has_errors());
        assert_eq!(diagnostics.errors().count(), 1);
    }

    #[test]
    fn collector_without_strict_keeps_warnings_non_fatal() {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(Diagnostic::warning("raw HTML is not supported"));

        assert!(!diagnostics.has_errors());
        assert_eq!(diagnostics.entries()[0].severity, Severity::Warning);
    }

    #[test]
    fn strict_collector_escalates_warnings_to_errors() {
        let mut diagnostics = Diagnostics::strict();
        diagnostics.push(Diagnostic::warning("raw HTML is not supported"));
        diagnostics.push(Diagnostic::note("informational"));

        assert!(diagnostics.has_errors());
        assert_eq!(diagnostics.entries()[0].severity, Severity::Error);
        // Notes are informational and stay that way even under --strict.
        assert_eq!(diagnostics.entries()[1].severity, Severity::Note);
        assert_eq!(diagnostics.into_entries().len(), 2);
    }

    #[test]
    fn exit_codes_follow_the_specification() {
        let io = CompileError::Io {
            context: "reading paper.md".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "nope"),
        };

        assert_eq!(
            CompileError::Markdown(Diagnostic::error("x")).exit_code(),
            1
        );
        assert_eq!(
            CompileError::MathParse(Diagnostic::error("x")).exit_code(),
            1
        );
        assert_eq!(
            CompileError::MathEmit(Diagnostic::error("x")).exit_code(),
            1
        );
        assert_eq!(
            CompileError::UnsupportedFeature(Diagnostic::error("x")).exit_code(),
            1
        );
        assert_eq!(
            CompileError::FrontMatter {
                message: "x".into(),
                span: None
            }
            .exit_code(),
            1
        );
        assert_eq!(
            CompileError::TypstCompile {
                details: "x".into()
            }
            .exit_code(),
            1
        );
        assert_eq!(CompileError::Internal("x".into()).exit_code(), 1);
        assert_eq!(CompileError::Diagnostics(vec![]).exit_code(), 1);
        assert_eq!(CompileError::Config("x".into()).exit_code(), 2);
        assert_eq!(io.exit_code(), 3);
        assert_eq!(CompileError::TypstMissing.exit_code(), 3);
    }

    #[test]
    fn missing_typst_error_tells_the_user_what_to_do() {
        let diagnostics = CompileError::TypstMissing.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].notes[0].contains("--emit typst"));
    }

    #[test]
    fn typst_compile_error_points_at_keep_typst_instead_of_dumping_output() {
        let diagnostics = CompileError::TypstCompile {
            details: "  error: unknown variable\n".into(),
        }
        .diagnostics();

        assert_eq!(diagnostics[0].message, "generated Typst failed to compile");
        assert!(diagnostics[0].notes[0].contains("unknown variable"));
        assert!(diagnostics[0].notes[1].contains("--keep-typst"));
    }

    #[test]
    fn front_matter_error_keeps_its_span() {
        let error = CompileError::FrontMatter {
            message: "mapping expected".into(),
            span: Some(SourceSpan::new(4, 8)),
        };
        assert_eq!(error.diagnostics()[0].span, Some(SourceSpan::new(4, 8)));
    }

    #[test]
    fn diagnostics_error_counts_only_errors_in_its_message() {
        let error = CompileError::Diagnostics(vec![
            Diagnostic::warning("w"),
            Diagnostic::error("e1"),
            Diagnostic::error("e2"),
        ]);
        assert_eq!(error.to_string(), "compilation produced 2 error(s)");
        assert_eq!(error.diagnostics().len(), 3);
    }

    #[test]
    fn io_error_helper_attaches_context() {
        let error = io_error("reading paper.md")(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(error.to_string(), "reading paper.md: denied");
        assert_eq!(error.exit_code(), 3);
    }
}

#[cfg(test)]
mod reporting_tests {
    use super::*;

    #[test]
    fn short_tool_output_is_shown_in_full() {
        let error = CompileError::TypstCompile {
            details: "error: unknown variable\n  at line 4".into(),
        };
        let note = &error.diagnostics()[0].notes[0];

        assert!(note.contains("unknown variable"), "{note}");
        assert!(!note.contains("more line"), "{note}");
    }

    #[test]
    fn long_tool_output_is_truncated_with_a_pointer_to_verbose() {
        let details: String = (1..=30).map(|line| format!("line {line}\n")).collect();
        let error = CompileError::TypstCompile {
            details: details.clone(),
        };
        let note = &error.diagnostics()[0].notes[0];

        // The first lines say what went wrong; the rest would bury it.
        assert!(note.contains("line 1\n"), "{note}");
        assert!(note.contains("line 8"), "{note}");
        assert!(!note.contains("line 9"), "{note}");
        assert!(note.contains("22 more line(s)"), "{note}");
        assert!(note.contains("--verbose"), "{note}");

        // Nothing is lost: the full text stays available.
        assert_eq!(error.full_details(), Some(details.as_str()));
    }

    #[test]
    fn tool_output_is_indented_under_its_heading() {
        let error = CompileError::TypstCompile {
            details: "oops".into(),
        };
        assert_eq!(error.diagnostics()[0].notes[0], "Caused by:\n    oops");
    }

    #[test]
    fn errors_without_tool_output_have_no_full_details() {
        assert_eq!(CompileError::TypstMissing.full_details(), None);
        assert_eq!(CompileError::Config("x".into()).full_details(), None);
    }

    #[test]
    fn truncation_keeps_exactly_the_requested_lines() {
        assert_eq!(truncate("a\nb\nc", 8), "a\nb\nc");
        assert_eq!(truncate("a\nb\nc", 3), "a\nb\nc");
        assert!(truncate("a\nb\nc", 2).starts_with("a\nb\n... 1 more line(s)"));
    }

    #[test]
    fn truncation_ignores_trailing_blank_lines() {
        assert_eq!(truncate("a\nb\n\n\n", 8), "a\nb");
    }

    #[test]
    fn indentation_applies_to_every_line() {
        assert_eq!(indent("a\nb", 2), "  a\n  b");
    }
}
