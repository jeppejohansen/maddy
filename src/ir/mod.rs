//! The semantic intermediate representation.
//!
//! The IR sits between Markdown syntax and Typst syntax:
//!
//! ```text
//! Markdown syntax  →  document semantics  →  Typst syntax
//! ```
//!
//! Keeping a representation here — rather than emitting Typst strings straight
//! from the Markdown event stream — is what makes the renderer replaceable, the
//! parser testable, source mapping possible and slide support a segmentation
//! pass rather than a second parser.
//!
//! The IR is deliberately small. It models what this compiler renders, not a
//! general publishing model.

mod block;
mod document;
mod inline;
mod presentation;

#[cfg(test)]
mod tests;

pub use block::{Alignment, Block, CodeBlock, Image, ListItem, MathMode, MathSource, Table};
pub use document::Document;
pub use inline::{plain_text, Inline};
pub use presentation::{Presentation, Slide};
