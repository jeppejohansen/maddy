//! Mathematics: LaTeX-style source to Typst equations.
//!
//! ```text
//! LaTeX-style math source
//!           ↓
//!       ratex-parser          parser.rs + ratex.rs
//!           ↓
//!       ParseNode AST
//!           ↓
//!     Typst math emitter      emit.rs + symbols.rs
//!           ↓
//!        Typst math
//! ```
//!
//! This module is a boundary: no type from the underlying parser appears in its
//! public interface, so the parser can be replaced without touching Markdown
//! parsing, the IR, the renderers or the CLI.

mod emit;
pub mod error;
pub mod parser;
mod ratex;
mod symbols;

#[cfg(test)]
mod tests;

pub use error::MathError;
pub use parser::MathParser;
pub use ratex::RatexParser;

use crate::ir::{MathMode, MathSource};

/// Translate LaTeX-style mathematics into Typst mathematics.
pub fn to_typst(source: &str, mode: MathMode) -> Result<String, MathError> {
    let ast = RatexParser.parse(source, mode)?;
    emit::emit(&ast, mode)
}

/// Translate an equation from the IR.
pub fn source_to_typst(math: &MathSource) -> Result<String, MathError> {
    to_typst(&math.source, math.mode)
}
