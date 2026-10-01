//! The top-level document.

use crate::metadata::{DocumentType, Metadata};

use super::block::Block;

/// A parsed document: metadata plus a flat sequence of blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    pub metadata: Metadata,
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new(metadata: Metadata, blocks: Vec<Block>) -> Self {
        Self { metadata, blocks }
    }

    pub fn document_type(&self) -> DocumentType {
        self.metadata.document_type
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}
