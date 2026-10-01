//! Typst emission and PDF compilation.
//!
//! This module is the only part of the compiler that knows Typst syntax
//! exists. Everything above it works in terms of the IR.
//!
//! ```text
//! Document IR  →  markup::Renderer  →  document.rs  →  Typst source
//! ```

pub mod document;
pub mod escape;
pub mod markup;

#[cfg(test)]
pub(crate) mod tests;

pub use document::emit_document;

/// What a renderer needs besides the document itself.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// The Typst template source, inlined into the generated document.
    pub template: String,
}

impl RenderOptions {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
        }
    }

    /// Render with the built-in document template.
    pub fn document_default() -> Self {
        Self::new(crate::templates::document())
    }
}
