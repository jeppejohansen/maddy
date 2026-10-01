//! The RaTeX parser adapter.
//!
//! This file and [`super::emit`] are the only places that name a RaTeX type.

use ratex_parser::ParseNode;

use crate::ir::MathMode;

use super::error::MathError;
use super::parser::MathParser;

/// A [`MathParser`] backed by `ratex-parser`, a Rust port of KaTeX's parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct RatexParser;

impl MathParser for RatexParser {
    type Ast = Vec<ParseNode>;

    fn parse(&self, source: &str, _mode: MathMode) -> Result<Self::Ast, MathError> {
        // The parser is mode-agnostic: `$x$` and `$$x$$` differ in how Typst
        // *sets* the result, not in how the LaTeX is read.
        ratex_parser::parse(source).map_err(|error| MathError::Parse {
            message: error.message,
            offset: error.loc.map(|loc| loc.start),
        })
    }
}
