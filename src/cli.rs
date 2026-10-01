//! Command-line interface.
//!
//! This module owns argument parsing and the top-level exit-code and
//! diagnostic-printing behaviour. It contains no compilation logic: everything
//! past argument parsing is delegated to [`crate::compiler`].

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

use crate::compiler::{self, CompileOptions};
use crate::diagnostics::{CompileError, Diagnostic};

/// What the compiler should write to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "lower")]
pub enum EmitFormat {
    /// Write only the PDF.
    #[default]
    Pdf,
    /// Write only the generated Typst source.
    Typst,
    /// Write both the Typst source and the PDF.
    Both,
}

impl EmitFormat {
    pub fn writes_pdf(self) -> bool {
        matches!(self, EmitFormat::Pdf | EmitFormat::Both)
    }

    pub fn writes_typst(self) -> bool {
        matches!(self, EmitFormat::Typst | EmitFormat::Both)
    }
}

/// How much the compiler should say on success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    /// Print nothing unless an error occurs.
    Quiet,
    /// Print one summary line.
    #[default]
    Normal,
    /// Print each compilation stage.
    Verbose,
}

/// Compile Markdown with LaTeX-style mathematics into a PDF document or
/// presentation.
#[derive(Debug, Parser)]
#[command(
    name = "mdpdf",
    version,
    about = "Compile Markdown with LaTeX-style mathematics into a PDF",
    long_about = None,
)]
pub struct Cli {
    /// Markdown source file
    pub input: PathBuf,

    /// Output path
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Output format
    #[arg(long, value_name = "FORMAT", default_value = "pdf")]
    pub emit: EmitFormat,

    /// Force presentation mode
    #[arg(long)]
    pub slides: bool,

    /// Rendering style
    #[arg(long, value_name = "STYLE")]
    pub style: Option<String>,

    /// Configuration file
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Custom Typst template
    #[arg(long, value_name = "PATH")]
    pub template: Option<PathBuf>,

    /// Keep generated Typst
    #[arg(long)]
    pub keep_typst: bool,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,

    /// Recompile after changes
    #[arg(long)]
    pub watch: bool,

    /// Suppress non-error output
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Show compilation details
    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    /// Translate the parsed arguments into compiler options.
    pub fn options(&self) -> CompileOptions {
        CompileOptions {
            output: self.output.clone(),
            config: self.config.clone(),
            emit: self.emit,
            force_slides: self.slides,
            style: self.style.clone(),
            template: self.template.clone(),
            keep_typst: self.keep_typst,
            strict: self.strict,
            verbosity: self.verbosity(),
        }
    }

    pub fn verbosity(&self) -> Verbosity {
        match (self.quiet, self.verbose) {
            (true, _) => Verbosity::Quiet,
            (_, true) => Verbosity::Verbose,
            _ => Verbosity::Normal,
        }
    }
}

/// Parse arguments and run the compiler, returning the process exit code.
pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match run_once(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

/// Compile once, reporting any failure. Returns the exit code on failure.
fn run_once(cli: &Cli) -> std::result::Result<(), u8> {
    let path = cli.input.display().to_string();

    // The source is read here rather than inside the compiler so that a
    // diagnostic can still be rendered with its snippet when compilation fails.
    let source = match std::fs::read_to_string(&cli.input) {
        Ok(source) => source,
        Err(error) => {
            let error = CompileError::Io {
                context: format!("reading {path}"),
                source: error,
            };
            report(&error, &path, "");
            return Err(error.exit_code());
        }
    };

    match compiler::compile_source(&cli.input, &source, &cli.options()) {
        Ok(result) => {
            print_diagnostics(&result.diagnostics, &path, &source);
            if cli.verbosity() != Verbosity::Quiet {
                for output in &result.outputs {
                    println!("Compiled {path} → {}", output.display());
                }
            }
            Ok(())
        }
        Err(error) => {
            report(&error, &path, &source);
            // The diagnostic shows the first few lines; `-v` shows everything.
            if cli.verbosity() == Verbosity::Verbose {
                if let Some(details) = error.full_details() {
                    eprintln!("Full output:\n{}\n", details.trim_end());
                }
            }
            Err(error.exit_code())
        }
    }
}

/// Print an error as a source-annotated diagnostic.
fn report(error: &CompileError, path: &str, source: &str) {
    print_diagnostics(&error.diagnostics(), path, source);
}

fn print_diagnostics(diagnostics: &[Diagnostic], path: &str, source: &str) {
    for diagnostic in diagnostics {
        eprintln!("{}\n", diagnostic.render(path, source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("mdpdf").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn the_command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn a_bare_input_file_is_the_only_required_argument() {
        let cli = parse(&["paper.md"]);

        assert_eq!(cli.input, PathBuf::from("paper.md"));
        assert_eq!(cli.output, None);
        assert_eq!(cli.emit, EmitFormat::Pdf);
        assert!(!cli.slides);
        assert_eq!(cli.style, None);
        assert_eq!(cli.config, None);
        assert_eq!(cli.template, None);
        assert!(!cli.keep_typst);
        assert!(!cli.strict);
        assert!(!cli.watch);
        assert_eq!(cli.verbosity(), Verbosity::Normal);
    }

    #[test]
    fn an_input_file_is_required() {
        assert!(Cli::try_parse_from(["mdpdf"]).is_err());
    }

    #[test]
    fn output_accepts_both_spellings() {
        assert_eq!(
            parse(&["paper.md", "-o", "result.pdf"]).output,
            Some("result.pdf".into())
        );
        assert_eq!(
            parse(&["paper.md", "--output", "result.pdf"]).output,
            Some("result.pdf".into())
        );
    }

    #[test]
    fn emit_defaults_to_pdf_and_accepts_the_three_formats() {
        assert_eq!(parse(&["paper.md"]).emit, EmitFormat::Pdf);
        assert_eq!(
            parse(&["paper.md", "--emit", "typst"]).emit,
            EmitFormat::Typst
        );
        assert_eq!(
            parse(&["paper.md", "--emit", "both"]).emit,
            EmitFormat::Both
        );
        assert!(Cli::try_parse_from(["mdpdf", "paper.md", "--emit", "docx"]).is_err());
    }

    #[test]
    fn emit_format_reports_which_artifacts_it_writes() {
        assert!(EmitFormat::Pdf.writes_pdf() && !EmitFormat::Pdf.writes_typst());
        assert!(EmitFormat::Typst.writes_typst() && !EmitFormat::Typst.writes_pdf());
        assert!(EmitFormat::Both.writes_pdf() && EmitFormat::Both.writes_typst());
    }

    #[test]
    fn slides_and_style_are_accepted_together() {
        let cli = parse(&["talk.md", "--slides", "--style", "academic"]);
        assert!(cli.slides);
        assert_eq!(cli.style.as_deref(), Some("academic"));
    }

    #[test]
    fn config_and_template_paths_are_parsed() {
        let cli = parse(&[
            "paper.md",
            "--config",
            "mdpdf.toml",
            "--template",
            "custom.typ",
        ]);
        assert_eq!(cli.config, Some("mdpdf.toml".into()));
        assert_eq!(cli.template, Some("custom.typ".into()));
    }

    #[test]
    fn boolean_flags_are_parsed() {
        let cli = parse(&["paper.md", "--keep-typst", "--strict", "--watch"]);
        assert!(cli.keep_typst);
        assert!(cli.strict);
        assert!(cli.watch);
    }

    #[test]
    fn quiet_and_verbose_select_a_verbosity() {
        assert_eq!(parse(&["paper.md", "-q"]).verbosity(), Verbosity::Quiet);
        assert_eq!(
            parse(&["paper.md", "--quiet"]).verbosity(),
            Verbosity::Quiet
        );
        assert_eq!(parse(&["paper.md", "-v"]).verbosity(), Verbosity::Verbose);
        assert_eq!(
            parse(&["paper.md", "--verbose"]).verbosity(),
            Verbosity::Verbose
        );
    }

    #[test]
    fn quiet_and_verbose_are_mutually_exclusive() {
        assert!(Cli::try_parse_from(["mdpdf", "paper.md", "-q", "-v"]).is_err());
    }
}
