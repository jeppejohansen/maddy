//! `maddy` — a compiler from Markdown with LaTeX-style mathematics to PDF.
//!
//! The crate is organized as a pipeline of independent layers:
//!
//! ```text
//! Markdown + LaTeX-style math
//!              │
//!              ▼
//!         markdown::parse            (syntax)
//!              │
//!              ▼
//!          ir::Document              (semantics)
//!              │
//!        ┌─────┴─────┐
//!        │           │
//!    document     presentation
//!        │           │
//!        └─────┬─────┘
//!              ▼
//!      typst::emit_*                 (rendering)
//!              │
//!              ▼
//!    typst::backend::PdfBackend      (PDF)
//! ```
//!
//! Each layer depends only on the one above it. In particular no
//! parser-specific type is visible outside the layer that owns the parser,
//! and no Typst syntax appears outside the [`typst`] module.

pub mod cli;
pub mod diagnostics;
pub mod ir;
pub mod metadata;

pub use diagnostics::{Diagnostic, Severity, SourceSpan};
