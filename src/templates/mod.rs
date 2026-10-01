//! Built-in Typst templates.
//!
//! Templates are plain Typst source, compiled into the binary and inlined into
//! the generated document. Inlining keeps the output a single self-contained
//! file, which is what makes `--emit typst` and `--keep-typst` genuinely useful
//! for debugging.
//!
//! The Rust renderer never contains theme-specific formatting. Its only
//! knowledge of a theme is which template source to inline.

/// The default document template.
pub const DOCUMENT_DEFAULT: &str = include_str!("document/default.typ");

/// The built-in document template.
pub fn document() -> &'static str {
    DOCUMENT_DEFAULT
}
