//! Compiler orchestration.
//!
//! This layer reads a file, runs each stage in turn and writes the results. It
//! holds no parsing, rendering or formatting logic of its own, and it is meant
//! to stay boring — that is a design goal, not an accident.
//!
//! ```text
//! source
//!   ↓  front-matter pre-pass
//! metadata + body
//!   ↓  option resolution
//! effective options
//!   ↓  Markdown parser
//! Document IR
//!   ↓  Typst emitter
//! Typst source
//!   ↓  PDF backend
//! PDF
//! ```

use std::path::{Path, PathBuf};

use crate::cli::{EmitFormat, Verbosity};
use crate::diagnostics::{io_error, CompileError, Diagnostic, Diagnostics, Result};
use crate::ir::{Block, Document};
use crate::markdown;
use crate::metadata::{DocumentType, Metadata};
use crate::templates;
use crate::typst::backend::{CompileContext, ExternalTypstBackend, PdfBackend};
use crate::typst::{emit_document, RenderOptions};

/// Everything the caller can decide, before the document has been read.
///
/// Fields that a document may also set are `Option`, so that "not given on the
/// command line" is distinguishable from "given, and equal to the default" —
/// which is what makes the precedence rule implementable.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub output: Option<PathBuf>,
    pub emit: EmitFormat,
    /// Set by `--slides`; overrides the document's own type.
    pub force_slides: bool,
    pub style: Option<String>,
    pub template: Option<PathBuf>,
    pub keep_typst: bool,
    pub strict: bool,
    pub verbosity: Verbosity,
}

/// The options actually in force, after resolving precedence.
#[derive(Debug, Clone)]
pub struct EffectiveOptions {
    pub document_type: DocumentType,
    pub style: Option<String>,
    pub emit: EmitFormat,
    pub keep_typst: bool,
    pub strict: bool,
    pub verbosity: Verbosity,
    pub output: Option<PathBuf>,
    pub template: Option<PathBuf>,
}

/// What a compilation produced.
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub document_type: DocumentType,
    /// Files written, in the order they were written.
    pub outputs: Vec<PathBuf>,
    /// Non-fatal diagnostics, for the caller to render.
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileResult {
    /// The PDF that was written, if any.
    pub fn pdf(&self) -> Option<&Path> {
        self.outputs
            .iter()
            .find(|path| path.extension().is_some_and(|ext| ext == "pdf"))
            .map(PathBuf::as_path)
    }

    /// The Typst source that was written, if any.
    pub fn typst(&self) -> Option<&Path> {
        self.outputs
            .iter()
            .find(|path| path.extension().is_some_and(|ext| ext == "typ"))
            .map(PathBuf::as_path)
    }
}

/// Read and compile a Markdown file.
pub fn compile(input: &Path, options: &CompileOptions) -> Result<CompileResult> {
    let source =
        std::fs::read_to_string(input).map_err(io_error(format!("reading {}", input.display())))?;
    compile_source(input, &source, options)
}

/// Compile a source string that has already been read.
///
/// The caller keeps the source so that it can render diagnostics with a snippet
/// even when compilation fails.
pub fn compile_source(
    input: &Path,
    source: &str,
    options: &CompileOptions,
) -> Result<CompileResult> {
    let log = Logger::new(options.verbosity);

    log.stage("Reading metadata...");
    let front = markdown::parse_frontmatter(source)?;
    let effective = resolve_options(&front.metadata, options);

    let mut diagnostics = if effective.strict {
        Diagnostics::strict()
    } else {
        Diagnostics::new()
    };
    diagnostics.extend(front.diagnostics);

    log.stage("Parsing Markdown...");
    let parsed = markdown::parse(front.body, front.body_offset, effective.document_type);
    diagnostics.extend(parsed.diagnostics);

    let document = Document::new(front.metadata, parsed.blocks);
    log.stage(&format!(
        "Parsing {} math expression(s)...",
        count_math(&document.blocks)
    ));

    log.stage("Generating Typst...");
    let render = render_options(&effective)?;
    let typst = match effective.document_type {
        DocumentType::Document => emit_document(&document, &render, &mut diagnostics)?,
        // Presentation rendering is added with the slides milestone.
        DocumentType::Slides => {
            return Err(CompileError::Internal(
                "presentation rendering is not implemented yet".into(),
            ))
        }
    };

    // Warnings escalated by `--strict` are fatal, and are reported before
    // anything is written.
    if diagnostics.has_errors() {
        return Err(CompileError::Diagnostics(diagnostics.into_entries()));
    }

    let outputs = write_outputs(input, &typst, &effective, &log)?;

    Ok(CompileResult {
        document_type: effective.document_type,
        outputs,
        diagnostics: diagnostics.into_entries(),
    })
}

/// Apply the precedence rule.
///
/// ```text
/// built-in defaults  →  configuration file  →  front matter  →  CLI arguments
/// ```
pub fn resolve_options(metadata: &Metadata, options: &CompileOptions) -> EffectiveOptions {
    let document_type = if options.force_slides {
        DocumentType::Slides
    } else {
        metadata.document_type
    };

    EffectiveOptions {
        document_type,
        // A style given on the command line wins over the document's own.
        style: options.style.clone().or_else(|| metadata.style.clone()),
        emit: options.emit,
        keep_typst: options.keep_typst,
        strict: options.strict,
        verbosity: options.verbosity,
        output: options.output.clone(),
        template: options.template.clone(),
    }
}

/// Choose the template to render with.
fn render_options(effective: &EffectiveOptions) -> Result<RenderOptions> {
    match &effective.template {
        Some(path) => {
            let template = std::fs::read_to_string(path)
                .map_err(io_error(format!("reading template {}", path.display())))?;
            Ok(RenderOptions::new(template))
        }
        None => Ok(RenderOptions::new(templates::document())),
    }
}

/// Write whichever artifacts were asked for.
fn write_outputs(
    input: &Path,
    typst: &str,
    effective: &EffectiveOptions,
    log: &Logger,
) -> Result<Vec<PathBuf>> {
    let mut outputs = Vec::new();

    // `--keep-typst` preserves the intermediate even when the PDF is what was
    // asked for.
    if effective.emit.writes_typst() || effective.keep_typst {
        let path = typst_path(input, effective);
        std::fs::write(&path, typst).map_err(io_error(format!("writing {}", path.display())))?;
        outputs.push(path);
    }

    if effective.emit.writes_pdf() {
        log.stage("Running Typst...");
        let backend = ExternalTypstBackend::new();
        let pdf = backend.compile(typst, &CompileContext::for_source(input))?;

        let path = pdf_path(input, effective);
        std::fs::write(&path, pdf).map_err(io_error(format!("writing {}", path.display())))?;
        outputs.push(path);
    }

    Ok(outputs)
}

/// Where the PDF goes.
fn pdf_path(input: &Path, effective: &EffectiveOptions) -> PathBuf {
    match &effective.output {
        Some(output) if effective.emit.writes_pdf() => output.clone(),
        _ => input.with_extension("pdf"),
    }
}

/// Where the Typst source goes.
///
/// An explicit `--output` names the Typst file only when Typst is the one thing
/// being emitted; with `--emit both` it names the PDF, and the Typst file takes
/// the same stem.
fn typst_path(input: &Path, effective: &EffectiveOptions) -> PathBuf {
    match &effective.output {
        Some(output) if effective.emit == EmitFormat::Typst => output.clone(),
        Some(output) => output.with_extension("typ"),
        None => input.with_extension("typ"),
    }
}

/// The number of equations in a block tree, for verbose logging.
fn count_math(blocks: &[Block]) -> usize {
    use crate::ir::Inline;

    fn in_inlines(content: &[Inline]) -> usize {
        content
            .iter()
            .map(|inline| match inline {
                Inline::Math(_) => 1,
                Inline::Emphasis(inner)
                | Inline::Strong(inner)
                | Inline::Strike(inner)
                | Inline::Link { content: inner, .. } => in_inlines(inner),
                _ => 0,
            })
            .sum()
    }

    blocks
        .iter()
        .map(|block| match block {
            Block::Math(_) => 1,
            Block::Paragraph(content) | Block::Heading { content, .. } => in_inlines(content),
            Block::BlockQuote(inner) => count_math(inner),
            Block::UnorderedList(items) => items.iter().map(|item| count_math(&item.blocks)).sum(),
            Block::OrderedList { items, .. } => {
                items.iter().map(|item| count_math(&item.blocks)).sum()
            }
            Block::Table(table) => {
                let header: usize = table.header.iter().map(|c| in_inlines(c)).sum();
                let rows: usize = table
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|c| in_inlines(c)).sum::<usize>())
                    .sum();
                header + rows
            }
            _ => 0,
        })
        .sum()
}

/// Progress reporting, which only says anything in verbose mode.
struct Logger {
    verbosity: Verbosity,
}

impl Logger {
    fn new(verbosity: Verbosity) -> Self {
        Self { verbosity }
    }

    fn stage(&self, message: &str) {
        if self.verbosity == Verbosity::Verbose {
            eprintln!("{message}");
        }
    }
}
