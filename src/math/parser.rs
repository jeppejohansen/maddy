//! The math parser abstraction.
//!
//! Hiding the parser behind a trait means it can be replaced without touching
//! Markdown parsing, the IR, the renderers or the CLI. Today the only
//! implementation is [`super::ratex::RatexParser`]; its AST type never leaves
//! the `math` module.

use crate::ir::MathMode;

use super::error::MathError;

/// A parser for LaTeX-style mathematics.
pub trait MathParser {
    /// The syntax tree this parser produces.
    ///
    /// Deliberately an associated type: callers outside this module must not be
    /// able to name it, which is what keeps parser internals from leaking.
    type Ast;

    fn parse(&self, source: &str, mode: MathMode) -> Result<Self::Ast, MathError>;
}
