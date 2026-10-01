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
use crate::config::Config;
use crate::diagnostics::{io_error, CompileError, Diagnostic, Diagnostics, Result};
use crate::ir::{Block, Document, Presentation};
use crate::markdown;
use crate::metadata::{DocumentType, Metadata};
use crate::templates;
use crate::typst::backend::{CompileContext, PdfBackend};
use crate::typst::EmbeddedTypstBackend;
use crate::typst::{emit_document, emit_presentation, RenderOptions};

/// Everything the caller can decide, before the document has been read.
///
/// Fields that a document may also set are `Option`, so that "not given on the
/// command line" is distinguishable from "given, and equal to the default" —
/// which is what makes the precedence rule implementable.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    pub output: Option<PathBuf>,
    /// An explicit `--config` path; otherwise one is discovered.
    pub config: Option<PathBuf>,
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
    /// Project configuration, already merged into the precedence chain.
    pub config: Config,
}

/// What a compilation produced.
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub document_type: DocumentType,
    /// Files written, in the order they were written.
    pub outputs: Vec<PathBuf>,
    /// Non-fatal diagnostics, for the caller to render.
    pub diagnostics: Vec<Diagnostic>,
    /// Every file this compilation read, which is what watch mode watches.
    pub inputs: Vec<PathBuf>,
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

/// The files a compilation of `input` would depend on, before it is attempted.
///
/// Used by watch mode after a failure, when no result is available: without it a
/// document with a syntax error would stop rebuilding.
pub fn probable_dependencies(input: &Path, options: &CompileOptions) -> Vec<PathBuf> {
    let config = Config::discover(input, options.config.as_deref())
        .ok()
        .and_then(|(_, path)| path)
        .or_else(|| options.config.clone());
    dependencies(input, config, options.template.clone())
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

    let (config, config_path) = Config::discover(input, options.config.as_deref())?;
    if let Some(path) = &config_path {
        log.stage(&format!("Using configuration from {}...", path.display()));
    }

    let effective = resolve_options(&front.metadata, &config, options);

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
        DocumentType::Slides => {
            // Slides are a rendering of the same blocks: segmentation happens
            // here, after one shared parse.
            let (presentation, warnings) = Presentation::from_document(document);
            diagnostics.extend(warnings);
            log.stage(&format!(
                "Laying out {} slide(s)...",
                presentation.page_count()
            ));
            emit_presentation(&presentation, &render, &mut diagnostics)?
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
        inputs: dependencies(input, config_path, effective.template.clone()),
    })
}

/// Whether this compilation names a font family Typst does not bundle.
///
/// A custom template is assumed to, since its contents are unknown. The
/// backend independently detects characters the bundled faces cannot render, so
/// being wrong here costs speed, never correctness.
fn needs_system_fonts(effective: &EffectiveOptions) -> bool {
    effective.template.is_some()
        || effective.config.document.font.is_some()
        || effective.config.code.font.is_some()
        || !templates::bundled_fonts_only(effective.document_type, effective.style.as_deref())
}

/// Every file a compilation depended on.
///
/// Watch mode rebuilds when any of them changes, so a document that starts or
/// stops using a configuration file is handled by recomputing this each time.
fn dependencies(input: &Path, config: Option<PathBuf>, template: Option<PathBuf>) -> Vec<PathBuf> {
    let mut paths = vec![input.to_path_buf()];
    paths.extend(config);
    paths.extend(template);
    paths
}

/// Apply the precedence rule.
///
/// ```text
/// built-in defaults  →  configuration file  →  front matter  →  CLI arguments
/// ```
pub fn resolve_options(
    metadata: &Metadata,
    config: &Config,
    options: &CompileOptions,
) -> EffectiveOptions {
    let document_type = if options.force_slides {
        DocumentType::Slides
    } else {
        metadata.document_type
    };

    EffectiveOptions {
        document_type,
        // Each source overrides the one before it: configuration, then the
        // document's front matter, then the command line.
        style: options
            .style
            .clone()
            .or_else(|| metadata.style.clone())
            .or_else(|| config.slides.style.clone()),
        emit: options.emit,
        keep_typst: options.keep_typst,
        strict: options.strict,
        verbosity: options.verbosity,
        output: options.output.clone(),
        template: options.template.clone(),
        config: config.clone(),
    }
}

/// Choose the template to render with.
///
/// A custom template replaces the built-in one entirely and is expected to
/// implement the same contract.
fn render_options(effective: &EffectiveOptions) -> Result<RenderOptions> {
    match &effective.template {
        Some(path) => {
            // A custom template receives configuration too: the rules are
            // emitted after it applies its own, so they override it just as
            // they override a built-in theme.
            let template = std::fs::read_to_string(path)
                .map_err(io_error(format!("reading template {}", path.display())))?;
            Ok(RenderOptions::new(template).with_prelude(effective.config.prelude()))
        }
        None => {
            let template = templates::builtin(effective.document_type, effective.style.as_deref())
                .map_err(|error| CompileError::Config(error.message()))?;
            Ok(RenderOptions::new(template).with_prelude(effective.config.prelude()))
        }
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
        let backend = EmbeddedTypstBackend::new();
        let context = CompileContext::for_source(input)
            .verbose(effective.verbosity == Verbosity::Verbose)
            .system_fonts(needs_system_fonts(effective));
        let pdf = backend.compile(typst, &context)?;

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
