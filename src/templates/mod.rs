//! Built-in Typst templates.
//!
//! Templates are plain Typst source, compiled into the binary and inlined into
//! the generated document. Inlining keeps the output a single self-contained
//! file, which is what makes `--emit typst` and `--keep-typst` useful — and it
//! is also forced: a generated document is compiled with the author's Markdown
//! directory as its root, so a template cannot import siblings.
//!
//! The Rust renderer never contains theme-specific formatting. Its only
//! knowledge of appearance is which names to pass to the slide template.

use crate::metadata::DocumentType;

#[cfg(test)]
mod tests;

/// The default document template.
pub const DOCUMENT_DEFAULT: &str = include_str!("document/default.typ");

/// The slide system: palettes, styles and composition in one file.
pub const SLIDES: &str = include_str!("slides/base.typ");

pub const DEFAULT_STYLE: &str = "modern";
pub const DEFAULT_PALETTE: &str = "midnight";
pub const DEFAULT_RATIO: &str = "16:9";
pub const DEFAULT_TEXT_SIZE: &str = "medium";

/// Typography and composition. Carries no colour of its own.
pub const STYLES: &[&str] = &["modern", "academic", "editorial", "bold", "technical"];

/// Colour only. Every palette exposes the same semantic tokens.
pub const PALETTES: &[&str] = &[
    "midnight",
    "bottle",
    "ochre",
    "claret",
    "aubergine",
    "petrol",
    "midnight-dark",
    "bottle-dark",
];

/// Page geometry.
pub const RATIOS: &[&str] = &["16:9", "3:2", "4:3"];

/// Named base body sizes. A length such as `22pt` is also accepted.
pub const TEXT_SIZES: &[&str] = &["small", "medium", "large"];

pub fn document() -> &'static str {
    DOCUMENT_DEFAULT
}

pub fn slides() -> &'static str {
    SLIDES
}

/// The built-in template for a document type.
pub fn builtin(document_type: DocumentType) -> &'static str {
    match document_type {
        // v0.1 ships one excellent document style rather than five.
        DocumentType::Document => document(),
        DocumentType::Slides => slides(),
    }
}

/// A value that names no built-in option.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {axis} `{value}`")]
pub struct Unknown {
    pub axis: &'static str,
    pub value: String,
    allowed: &'static [&'static str],
}

impl Unknown {
    /// The message shown to the user, naming what they could have meant.
    pub fn message(&self) -> String {
        format!(
            "unknown {} `{}`, expected one of: {}",
            self.axis,
            self.value,
            self.allowed.join(", ")
        )
    }
}

/// Check a value against the options for its axis, case-insensitively.
///
/// Returns the canonical spelling, so `DARK` reaches the template as `dark`.
fn check(
    axis: &'static str,
    allowed: &'static [&'static str],
    value: &str,
) -> Result<String, Unknown> {
    let trimmed = value.trim();
    allowed
        .iter()
        .find(|option| option.eq_ignore_ascii_case(trimmed))
        .map(|option| (*option).to_string())
        .ok_or_else(|| Unknown {
            axis,
            value: trimmed.to_string(),
            allowed,
        })
}

pub fn style(value: Option<&str>) -> Result<String, Unknown> {
    check("style", STYLES, value.unwrap_or(DEFAULT_STYLE))
}

pub fn palette(value: Option<&str>) -> Result<String, Unknown> {
    check("palette", PALETTES, value.unwrap_or(DEFAULT_PALETTE))
}

pub fn ratio(value: Option<&str>) -> Result<String, Unknown> {
    check("ratio", RATIOS, value.unwrap_or(DEFAULT_RATIO))
}

/// A text size: a preset name, or an explicit Typst length.
///
/// An explicit length is passed through so that the style's own multipliers
/// still shape the rest of the hierarchy around it.
pub fn text_size(value: Option<&str>) -> Result<String, Unknown> {
    let value = value.unwrap_or(DEFAULT_TEXT_SIZE);
    if let Ok(name) = check("text size", TEXT_SIZES, value) {
        return Ok(name);
    }
    if is_length(value.trim()) {
        return Ok(value.trim().to_string());
    }
    Err(Unknown {
        axis: "text size",
        value: value.trim().to_string(),
        allowed: TEXT_SIZES,
    })
}

/// Whether a string is a Typst length, such as `22pt`.
fn is_length(value: &str) -> bool {
    let Some(split) = value.find(|c: char| c.is_ascii_alphabetic()) else {
        return false;
    };
    let (number, unit) = value.split_at(split);
    number.parse::<f64>().is_ok_and(|n| n > 0.0) && matches!(unit, "pt" | "mm" | "cm" | "in" | "em")
}
