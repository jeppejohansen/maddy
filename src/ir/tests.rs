//! Unit tests for the intermediate representation.

use crate::diagnostics::SourceSpan;
use crate::metadata::{DocumentType, Metadata};

use super::*;

fn text(value: &str) -> Inline {
    Inline::text(value)
}

#[test]
fn plain_text_strips_all_inline_markup() {
    let content = vec![
        text("This is "),
        Inline::Strong(vec![text("bold")]),
        text(" and "),
        Inline::Emphasis(vec![text("italic")]),
        text(" and "),
        Inline::Strike(vec![text("gone")]),
        text("."),
    ];
    assert_eq!(plain_text(&content), "This is bold and italic and gone.");
}

#[test]
fn plain_text_uses_link_content_not_the_destination() {
    let link = Inline::link("https://example.com", vec![text("the paper")]);
    assert_eq!(link.plain_text(), "the paper");
}

#[test]
fn plain_text_keeps_code_and_math_source() {
    assert_eq!(Inline::code("x + 1").plain_text(), "x + 1");
    assert_eq!(
        Inline::Math(MathSource::inline(r"\beta_1")).plain_text(),
        r"\beta_1"
    );
}

#[test]
fn plain_text_uses_the_alt_text_of_an_image() {
    let image = Inline::Image(Image::new("figures/results.png", vec![text("Results")]));
    assert_eq!(image.plain_text(), "Results");
}

#[test]
fn plain_text_renders_breaks_as_spaces() {
    let content = vec![
        text("one"),
        Inline::SoftBreak,
        text("two"),
        Inline::HardBreak,
        text("three"),
    ];
    assert_eq!(plain_text(&content), "one two three");
}

#[test]
fn plain_text_of_nothing_is_empty() {
    assert_eq!(plain_text(&[]), "");
}

#[test]
fn plain_text_recurses_through_nested_markup() {
    let content = vec![Inline::Strong(vec![Inline::Emphasis(vec![text("deep")])])];
    assert_eq!(plain_text(&content), "deep");
}

#[test]
fn a_level_one_heading_is_a_slide_title() {
    assert!(Block::heading(1, vec![text("Motivation")]).is_slide_title());
    assert!(!Block::heading(2, vec![text("Robustness")]).is_slide_title());
    assert!(!Block::paragraph(vec![text("body")]).is_slide_title());
}

#[test]
fn a_list_item_holds_blocks_so_nesting_needs_no_special_case() {
    let nested = ListItem::new(vec![
        Block::paragraph(vec![text("outer")]),
        Block::UnorderedList(vec![ListItem::text(vec![text("inner")])]),
    ]);
    assert_eq!(nested.blocks.len(), 2);
    assert_eq!(ListItem::text(vec![text("solo")]).blocks.len(), 1);
    assert!(ListItem::default().blocks.is_empty());
}

#[test]
fn table_column_count_is_the_widest_row() {
    let table = Table {
        alignments: vec![Alignment::Left, Alignment::Right],
        header: vec![vec![text("Model")], vec![text("Estimate")]],
        rows: vec![vec![
            vec![text("OLS")],
            vec![text("0.42")],
            vec![text("extra")],
        ]],
    };
    assert_eq!(table.columns(), 3);
    assert!(table.has_header());
}

#[test]
fn an_empty_table_has_no_columns_and_no_header() {
    let table = Table::default();
    assert_eq!(table.columns(), 0);
    assert!(!table.has_header());
}

#[test]
fn table_alignment_defaults_for_undescribed_columns() {
    let table = Table {
        alignments: vec![Alignment::Right],
        ..Table::default()
    };
    assert_eq!(table.alignment(0), Alignment::Right);
    assert_eq!(table.alignment(1), Alignment::None);
    assert_eq!(Alignment::default(), Alignment::None);
}

#[test]
fn remote_image_destinations_are_recognized() {
    for destination in [
        "https://example.com/plot.png",
        "http://example.com/plot.png",
        "HTTPS://EXAMPLE.COM/plot.png",
        "//example.com/plot.png",
        "  https://example.com/plot.png",
    ] {
        assert!(Image::new(destination, vec![]).is_remote(), "{destination}");
    }
}

#[test]
fn local_image_destinations_are_not_remote() {
    for destination in [
        "figures/results.png",
        "./a.png",
        "/abs/a.svg",
        "a-http-thing.png",
    ] {
        assert!(
            !Image::new(destination, vec![]).is_remote(),
            "{destination}"
        );
    }
}

#[test]
fn math_source_records_its_mode() {
    assert_eq!(MathSource::inline("x").mode, MathMode::Inline);
    assert_eq!(MathSource::display("x").mode, MathMode::Display);
    assert!(MathMode::Display.is_display());
    assert!(!MathMode::Inline.is_display());
}

#[test]
fn math_source_keeps_its_latex_verbatim_and_its_span() {
    let math = MathSource::new(r"\frac{x}{y}", MathMode::Display, SourceSpan::new(12, 24));
    assert_eq!(math.source, r"\frac{x}{y}");
    assert_eq!(math.span, SourceSpan::new(12, 24));
}

#[test]
fn a_default_document_is_empty_and_of_document_type() {
    let document = Document::default();
    assert!(document.is_empty());
    assert_eq!(document.document_type(), DocumentType::Document);
}

#[test]
fn a_document_exposes_the_type_from_its_metadata() {
    let document = Document::new(
        Metadata {
            document_type: DocumentType::Slides,
            ..Metadata::default()
        },
        vec![Block::paragraph(vec![text("hi")])],
    );
    assert_eq!(document.document_type(), DocumentType::Slides);
    assert!(!document.is_empty());
}
