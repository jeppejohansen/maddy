//! The presentation representation, and the segmentation pass that builds it.
//!
//! Slides are a *rendering* of the same blocks a document is made of, not a
//! separate parse. The Markdown parser emits [`Block::SlideBreak`] in place of a
//! thematic rule when the output is a presentation, and this pass cuts the block
//! list at those points:
//!
//! ```text
//! Markdown
//!    ↓
//! Block IR containing SlideBreak
//!    ↓
//! presentation segmentation
//!    ↓
//! Vec<Slide>
//! ```
//!
//! Keeping segmentation separate is what lets both output modes share exactly
//! one Markdown parser.

use crate::diagnostics::{Diagnostic, SourceSpan};
use crate::metadata::Metadata;

use super::block::Block;
use super::document::Document;
use super::inline::Inline;

/// A presentation: metadata plus a sequence of slides.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Presentation {
    pub metadata: Metadata,
    pub slides: Vec<Slide>,
}

/// One slide.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Slide {
    /// The slide's title, taken from a level-one heading at its start.
    ///
    /// Extracted here rather than in the renderer so that a theme receives a
    /// title and a body, and needs no knowledge of how either was identified.
    pub title: Option<Vec<Inline>>,
    pub blocks: Vec<Block>,
    pub span: Option<SourceSpan>,
}

impl Slide {
    /// Whether the slide would render as blank.
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.blocks.is_empty()
    }
}

impl Presentation {
    /// Segment a parsed document into slides.
    ///
    /// Returns any warnings raised while segmenting, such as a slide carrying
    /// more than one title.
    pub fn from_document(document: Document) -> (Self, Vec<Diagnostic>) {
        let Document { metadata, blocks } = document;
        let mut diagnostics = Vec::new();
        let mut slides = Vec::new();

        for group in split_on_breaks(blocks) {
            let slide = build_slide(group, &mut diagnostics);
            // A separator at the very start, or two in a row, yields nothing to
            // show; a blank slide is never what the author meant.
            if !slide.is_empty() {
                slides.push(slide);
            }
        }

        (Presentation { metadata, slides }, diagnostics)
    }

    /// Whether a title slide should be generated before the first slide.
    pub fn has_title_slide(&self) -> bool {
        self.metadata.has_title_page()
    }

    /// The number of rendered pages, including any generated title slide.
    pub fn page_count(&self) -> usize {
        self.slides.len() + usize::from(self.has_title_slide())
    }
}

/// Cut a block list at every slide break.
fn split_on_breaks(blocks: Vec<Block>) -> Vec<Vec<Block>> {
    let mut groups = vec![Vec::new()];
    for block in blocks {
        match block {
            Block::SlideBreak => groups.push(Vec::new()),
            other => groups
                .last_mut()
                .expect("there is always a current group")
                .push(other),
        }
    }
    groups
}

/// Turn one group of blocks into a slide, lifting a leading title out of it.
fn build_slide(blocks: Vec<Block>, diagnostics: &mut Vec<Diagnostic>) -> Slide {
    let span = slide_span(&blocks);
    let mut blocks = blocks;

    // Only a level-one heading *at the start* is the title. One further in is
    // body content, which is what makes a slide of sections work.
    let title = match blocks.first() {
        Some(block) if block.is_slide_title() => match blocks.remove(0) {
            Block::Heading { content, .. } => Some(content),
            _ => unreachable!("checked by is_slide_title"),
        },
        _ => None,
    };

    // A second level-one heading cannot become a second title, so say so rather
    // than silently rendering it as body text.
    for block in &blocks {
        if let Block::Heading { level: 1, span, .. } = block {
            diagnostics.push(
                Diagnostic::warning("a slide has more than one level-one heading")
                    .with_span(*span)
                    .with_note(
                        "Only a level-one heading at the start of a slide becomes its title; \
                         separate slides with `---`.",
                    ),
            );
        }
    }

    Slide {
        title,
        blocks,
        span,
    }
}

/// The span covering a slide's source, where its blocks carry one.
fn slide_span(blocks: &[Block]) -> Option<SourceSpan> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading { span, .. } => Some(*span),
            Block::Math(math) => Some(math.span),
            Block::Image(image) => Some(image.span),
            _ => None,
        })
        .reduce(|accumulated, span| accumulated.merge(&span))
}
