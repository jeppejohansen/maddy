//! Typst emission and PDF compilation.
//!
//! This module is the only part of the compiler that knows Typst syntax
//! exists. Everything above it works in terms of the IR.
//!
//! ```text
//! Document IR  →  markup::Renderer  →  document.rs  →  Typst source
//! ```

pub mod backend;
pub mod document;
pub mod escape;
pub mod markup;
pub mod presentation;

#[cfg(test)]
pub(crate) mod tests;

pub use backend::{CompileContext, ExternalTypstBackend, PdfBackend};
pub use document::emit_document;
pub use presentation::emit_presentation;

/// What a renderer needs besides the document itself.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// The Typst template source, inlined into the generated document.
    pub template: String,
    /// Typst `set` rules from configuration.
    ///
    /// Emitted at the top of the body, after the template has applied its own
    /// rules, so that these override it. That is what lets configuration work
    /// without any change to the theme contract, and work equally well with a
    /// custom template.
    pub prelude: String,
}

impl RenderOptions {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            prelude: String::new(),
        }
    }

    /// Add configuration rules.
    pub fn with_prelude(mut self, prelude: impl Into<String>) -> Self {
        self.prelude = prelude.into();
        self
    }

    /// Render with the built-in document template.
    pub fn document_default() -> Self {
        Self::new(crate::templates::document())
    }
}
