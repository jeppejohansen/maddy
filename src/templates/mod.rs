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

#[cfg(test)]
mod tests;

/// The default document template.
pub const DOCUMENT_DEFAULT: &str = include_str!("document/default.typ");

/// The default slide style.
pub const DEFAULT_SLIDE_STYLE: &str = "academic";

/// The built-in slide themes, in the order the documentation lists them.
pub const SLIDE_STYLES: &[Theme] = &[
    Theme::bundled("academic", include_str!("slides/academic.typ")),
    // Typst bundles no sans-serif face, so these two name system families.
    Theme::system("minimal", include_str!("slides/minimal.typ")),
    Theme::bundled("dark", include_str!("slides/dark.typ")),
    Theme::system("bold", include_str!("slides/bold.typ")),
    Theme::bundled("mono", include_str!("slides/mono.typ")),
];

/// A built-in theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub source: &'static str,
    /// Whether the theme renders correctly with only Typst's bundled fonts.
    ///
    /// A theme that names a system family needs those fonts loaded, which is
    /// slower; knowing this up front saves scanning them for the themes that
    /// do not.
    pub bundled_fonts_only: bool,
}

impl Theme {
    const fn bundled(name: &'static str, source: &'static str) -> Self {
        Self {
            name,
            source,
            bundled_fonts_only: true,
        }
    }

    const fn system(name: &'static str, source: &'static str) -> Self {
        Self {
            name,
            source,
            bundled_fonts_only: false,
        }
    }
}

/// The built-in document template.
pub fn document() -> &'static str {
    DOCUMENT_DEFAULT
}

/// A built-in slide theme by name.
pub fn slides(style: &str) -> Option<&'static str> {
    slide_theme(style).map(|theme| theme.source)
}

/// A built-in slide theme, with its font requirements.
pub fn slide_theme(style: &str) -> Option<&'static Theme> {
    SLIDE_STYLES
        .iter()
        .find(|theme| theme.name.eq_ignore_ascii_case(style.trim()))
}

/// The names of the built-in slide themes.
pub fn slide_style_names() -> Vec<&'static str> {
    SLIDE_STYLES.iter().map(|theme| theme.name).collect()
}

/// Whether a built-in template renders correctly with only the bundled fonts.
///
/// The document template names only bundled families, so it always does.
pub fn bundled_fonts_only(document_type: DocumentType, style: Option<&str>) -> bool {
    match document_type {
        DocumentType::Document => true,
        DocumentType::Slides => slide_theme(style.unwrap_or(DEFAULT_SLIDE_STYLE))
            .is_some_and(|theme| theme.bundled_fonts_only),
    }
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
