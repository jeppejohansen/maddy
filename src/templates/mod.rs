//! Built-in Typst templates.
//!
//! Templates are plain Typst source, compiled into the binary and inlined into
//! the generated document. Inlining keeps the output a single self-contained
//! file, which is what makes `--emit typst` and `--keep-typst` genuinely useful
//! for debugging.
//!
//! The Rust renderer never contains theme-specific formatting. Its only
//! knowledge of a theme is which template source to inline.

use crate::metadata::DocumentType;

/// The default document template.
pub const DOCUMENT_DEFAULT: &str = include_str!("document/default.typ");

/// The default slide style.
pub const DEFAULT_SLIDE_STYLE: &str = "academic";

/// The built-in slide themes, in the order the documentation lists them.
pub const SLIDE_STYLES: &[(&str, &str)] = &[("academic", include_str!("slides/academic.typ"))];

/// The built-in document template.
pub fn document() -> &'static str {
    DOCUMENT_DEFAULT
}

/// A built-in slide theme by name.
pub fn slides(style: &str) -> Option<&'static str> {
    SLIDE_STYLES
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(style.trim()))
        .map(|(_, source)| *source)
}

/// The names of the built-in slide themes.
pub fn slide_style_names() -> Vec<&'static str> {
    SLIDE_STYLES.iter().map(|(name, _)| *name).collect()
}

/// The built-in template for a document type and optional style.
///
/// An unknown style is an error rather than a silent fallback: a misspelled
/// `--style` should say so, not quietly produce the default theme.
pub fn builtin(
    document_type: DocumentType,
    style: Option<&str>,
) -> std::result::Result<&'static str, UnknownStyle> {
    match document_type {
        // v0.1 ships one excellent document style rather than five.
        DocumentType::Document => Ok(document()),
        DocumentType::Slides => {
            let style = style.unwrap_or(DEFAULT_SLIDE_STYLE);
            slides(style).ok_or_else(|| UnknownStyle(style.to_string()))
        }
    }
}

/// A style name that matches no built-in theme.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown style `{0}`")]
pub struct UnknownStyle(pub String);

impl UnknownStyle {
    /// The message shown to the user, naming the styles that do exist.
    pub fn message(&self) -> String {
        format!(
            "unknown style `{}`, expected one of: {}",
            self.0,
            slide_style_names().join(", ")
        )
    }
}
