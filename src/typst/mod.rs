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
pub mod embedded;
pub mod escape;
pub mod markup;
pub mod presentation;

#[cfg(test)]
pub(crate) mod tests;

pub use backend::{CompileContext, ExternalTypstBackend, PdfBackend};
pub use document::emit_document;
pub use embedded::EmbeddedTypstBackend;
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
    /// The slide axes, already validated. Passed to the template's show rule;
    /// the renderer never acts on them itself.
    pub slides: Option<SlideOptions>,
}

/// The four independent slide choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideOptions {
    pub style: String,
    pub palette: String,
    pub ratio: String,
    pub text_size: String,
}

impl RenderOptions {
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            prelude: String::new(),
            slides: None,
        }
    }

    /// Render slides with the given style, palette, ratio and text size.
    pub fn with_slides(mut self, slides: SlideOptions) -> Self {
        self.slides = Some(slides);
        self
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
