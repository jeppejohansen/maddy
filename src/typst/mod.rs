//! Typst emission and PDF compilation.
//!
//! This module is the only part of the compiler that knows Typst syntax
//! exists. Everything above it works in terms of the IR.

pub mod escape;

#[cfg(test)]
pub(crate) mod tests;
