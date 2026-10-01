//! Mathematics errors.
//!
//! Both variants carry an offset *within the equation source*, not within the
//! document. The caller shifts it onto the enclosing [`crate::ir::MathSource`]
//! span, which is how an error inside `$...$` ends up pointing at the right
//! column of the Markdown file.

/// A failure while translating mathematics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MathError {
    /// The mathematics could not be parsed.
    #[error("{message}")]
    Parse {
        message: String,
        offset: Option<usize>,
    },

    /// The mathematics parsed, but cannot be translated to Typst.
    ///
    /// Reported rather than approximated: incorrect mathematics is worse than a
    /// refusal to render.
    #[error("{construct} is not yet supported")]
    Unsupported {
        construct: String,
        offset: Option<usize>,
    },
}

impl MathError {
    pub fn unsupported(construct: impl Into<String>) -> Self {
        MathError::Unsupported {
            construct: construct.into(),
            offset: None,
        }
    }

    /// The offset within the equation source, when known.
    pub fn offset(&self) -> Option<usize> {
        match self {
            MathError::Parse { offset, .. } | MathError::Unsupported { offset, .. } => *offset,
        }
    }

    /// A note naming what was parsed, for the diagnostic's second line.
    pub fn note(&self) -> Option<String> {
        match self {
            MathError::Parse { .. } => None,
            MathError::Unsupported { construct, .. } => Some(format!("Parsed as: {construct}")),
        }
    }
}
