//! PDF compilation.
//!
//! The backend is abstracted so that the current subprocess implementation can
//! later be replaced by Typst's Rust compilation API without any upstream code
//! caring which is in use.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::diagnostics::{CompileError, Result};

/// What a backend needs to know besides the source text.
#[derive(Debug, Clone)]
pub struct CompileContext {
    /// The directory that relative paths in the source resolve against.
    ///
    /// This is the directory holding the Markdown file, so that an image written
    /// as `figures/results.png` means what the author expects.
    pub root: PathBuf,
}

impl CompileContext {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The context for compiling a given source file.
    pub fn for_source(input: &Path) -> Self {
        let root = input.parent().filter(|path| !path.as_os_str().is_empty());
        Self::new(root.unwrap_or(Path::new(".")))
    }
}

/// Compiles Typst source into a PDF.
pub trait PdfBackend {
    fn compile(&self, typst_source: &str, context: &CompileContext) -> Result<Vec<u8>>;
}

/// Compiles by invoking the installed `typst` executable.
///
/// Source is piped in and the PDF is piped out, so nothing is written to the
/// filesystem and no stray intermediate file is left in the author's directory.
/// Relative paths still resolve correctly because Typst resolves them against
/// `--root`.
#[derive(Debug, Clone)]
pub struct ExternalTypstBackend {
    program: OsString,
}

impl Default for ExternalTypstBackend {
    fn default() -> Self {
        Self {
            program: OsString::from("typst"),
        }
    }
}

impl ExternalTypstBackend {
    pub fn new() -> Self {
        Self::default()
    }

    /// Use a different executable, for testing or an unusual installation.
    pub fn with_program(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: program.as_ref().to_os_string(),
        }
    }
}

impl PdfBackend for ExternalTypstBackend {
    fn compile(&self, typst_source: &str, context: &CompileContext) -> Result<Vec<u8>> {
        // The command is built directly, never as a shell string: nothing from
        // the document can become part of a command line.
        let mut child = Command::new(&self.program)
            .arg("compile")
            .arg("--root")
            .arg(&context.root)
            .arg("-")
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => CompileError::TypstMissing,
                _ => CompileError::Io {
                    context: "starting typst".into(),
                    source: error,
                },
            })?;

        // Written on another thread: the child begins emitting the PDF before it
        // has read all of its input, so writing and reading must overlap or a
        // large document deadlocks.
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let source = typst_source.to_owned();
        let writer = std::thread::spawn(move || stdin.write_all(source.as_bytes()));

        let output = child.wait_with_output().map_err(|error| CompileError::Io {
            context: "running typst".into(),
            source: error,
        })?;

        // A broken pipe here means Typst exited early, which its own stderr
        // explains better than the write error would.
        let _ = writer.join();

        if !output.status.success() {
            return Err(CompileError::TypstCompile {
                details: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        if output.stdout.is_empty() {
            return Err(CompileError::TypstCompile {
                details: "typst reported success but produced no PDF".into(),
            });
        }

        Ok(output.stdout)
    }
}
