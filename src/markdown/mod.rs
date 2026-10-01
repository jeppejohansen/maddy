//! Markdown parsing.
//!
//! Front matter is removed by a pre-pass before the Markdown parser runs, which
//! keeps YAML handling out of the parser and resolves the ambiguity between a
//! YAML delimiter and a slide separator:
//!
//! ```text
//! source
//!   ↓
//! front-matter pre-pass
//!   ↓
//! metadata + body
//!   ↓
//! Markdown parser
//! ```

pub mod frontmatter;

#[cfg(test)]
mod tests;

pub use frontmatter::{split as parse_frontmatter, FrontMatter};
