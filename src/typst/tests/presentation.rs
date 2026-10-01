//! Unit tests for presentation emission.

use crate::diagnostics::Diagnostics;
use crate::ir::{Block, Document, Inline, MathSource, Presentation};
use crate::metadata::{DocumentType, Metadata};
use crate::templates;
use crate::typst::{emit_presentation, RenderOptions};

fn text(value: &str) -> Inline {
    Inline::text(value)
}

fn options() -> RenderOptions {
    RenderOptions::new(templates::slides("academic").expect("the academic theme"))
}

/// Segment and render, requiring success.
fn emit(metadata: Metadata, blocks: Vec<Block>) -> String {
    let (presentation, _) = Presentation::from_document(Document::new(metadata, blocks));
    let mut diagnostics = Diagnostics::new();
    emit_presentation(&presentation, &options(), &mut diagnostics)
        .unwrap_or_else(|error| panic!("emission failed: {error}"))
}

fn slides_metadata() -> Metadata {
    Metadata {
        title: Some("Peer Effects".into()),
        subtitle: Some("High School Application".into()),
        author: Some("Jane Researcher".into()),
        document_type: DocumentType::Slides,
        style: Some("academic".into()),
        ..Metadata::default()
    }
}

#[test]
fn the_theme_is_inlined_and_applied() {
    let output = emit(slides_metadata(), vec![Block::paragraph(vec![text("hi")])]);

    assert!(output.contains("#let presentation("), "{output}");
    assert!(output.contains("#let slide("), "{output}");
    assert!(output.contains("#let title-slide("), "{output}");
    assert!(output.contains("#show: presentation.with("), "{output}");
}

#[test]
fn a_title_slide_is_emitted_when_metadata_carries_a_title() {
    let output = emit(slides_metadata(), vec![Block::paragraph(vec![text("hi")])]);

    assert!(output.contains("#title-slide(\n"), "{output}");
    assert!(output.contains(r#"title: "Peer Effects""#), "{output}");
    assert!(
        output.contains(r#"subtitle: "High School Application""#),
        "{output}"
    );
}

#[test]
fn no_title_means_no_title_slide_call() {
    let metadata = Metadata {
        document_type: DocumentType::Slides,
        ..Metadata::default()
    };
    let output = emit(metadata, vec![Block::paragraph(vec![text("hi")])]);
    assert!(!output.contains("#title-slide("), "{output}");
}

#[test]
fn the_title_slide_precedes_the_first_slide() {
    let output = emit(
        slides_metadata(),
        vec![Block::paragraph(vec![text("body")])],
    );
    let title = output.find("#title-slide(").expect("a title slide");
    let first = output.find("#slide[").expect("a slide");
    assert!(title < first, "{output}");
}

#[test]
fn each_slide_becomes_a_call_to_the_theme() {
    let output = emit(
        Metadata::default(),
        vec![
            Block::paragraph(vec![text("one")]),
            Block::SlideBreak,
            Block::paragraph(vec![text("two")]),
        ],
    );
    assert_eq!(output.matches("#slide").count(), 2, "{output}");
}

#[test]
fn a_slide_title_is_passed_as_content_not_a_string() {
    // Content, so that markup and mathematics work in a title.
    let output = emit(
        Metadata::default(),
        vec![Block::heading(1, vec![text("Motivation")])],
    );
    assert!(output.contains("#slide(title: [Motivation])["), "{output}");
}

#[test]
fn mathematics_works_in_a_slide_title() {
    let output = emit(
        Metadata::default(),
        vec![Block::heading(
            1,
            vec![
                text("Estimating "),
                Inline::Math(MathSource::inline(r"\beta")),
            ],
        )],
    );
    assert!(
        output.contains("#slide(title: [Estimating $β$])["),
        "{output}"
    );
}

#[test]
fn an_untitled_slide_omits_the_argument() {
    let output = emit(
        Metadata::default(),
        vec![Block::Math(MathSource::display("Y = X"))],
    );
    // The inlined theme declares `title:` as a parameter, so the check has to
    // look at the generated call rather than the whole file.
    let call = output
        .rfind("#slide")
        .map(|at| &output[at..])
        .expect("a slide call");
    assert!(call.starts_with("#slide[\n"), "{call}");
}

#[test]
fn slide_bodies_are_indented_for_readability() {
    let output = emit(
        Metadata::default(),
        vec![Block::paragraph(vec![text("body")])],
    );
    assert!(output.contains("#slide[\n  body\n]"), "{output}");
}

#[test]
fn a_slide_body_renders_the_same_constructs_a_document_does() {
    let output = emit(
        Metadata::default(),
        vec![
            Block::heading(1, vec![text("Results")]),
            Block::UnorderedList(vec![crate::ir::ListItem::text(vec![text("positive")])]),
            Block::Math(MathSource::display(r"\hat\beta > 0")),
        ],
    );

    assert!(output.contains("- positive"), "{output}");
    assert!(output.contains("$ hat(β) > 0 $"), "{output}");
}

#[test]
fn an_empty_presentation_still_emits_a_valid_file() {
    let output = emit(Metadata::default(), vec![]);
    assert!(output.contains("#let presentation("));
    assert!(!output.contains("#slide"), "{output}");
}

// ----------------------------------------------- verified against real Typst

#[test]
fn the_generated_presentation_compiles_under_typst() {
    let output = emit(
        slides_metadata(),
        vec![
            Block::heading(1, vec![text("Motivation")]),
            Block::UnorderedList(vec![
                crate::ir::ListItem::text(vec![text("Friends share information")]),
                crate::ir::ListItem::text(vec![text("Choices may be complements")]),
            ]),
            Block::SlideBreak,
            Block::heading(1, vec![text("Model")]),
            Block::Math(MathSource::display(
                r"p_i = \operatorname{logit}^{-1}\left(X_i^\top\beta + \lambda \bar p_{-i}\right)",
            )),
            Block::SlideBreak,
            Block::heading(1, vec![text("Results")]),
            Block::Table(crate::ir::Table {
                alignments: vec![crate::ir::Alignment::None, crate::ir::Alignment::Right],
                header: vec![vec![text("Model")], vec![text("Effect")]],
                rows: vec![vec![vec![text("No peers")], vec![text("3.2%")]]],
            }),
        ],
    );

    let Some(rendered) = crate::typst::tests::render_text(&output) else {
        return; // Typst is unavailable; the assertions above still hold.
    };

    // Compilation succeeded and the slide content survived. Tables are checked
    // end to end instead: Typst's HTML export, which this harness uses, does not
    // render them yet.
    assert!(rendered.contains("Motivation"), "{rendered}");
    assert!(rendered.contains("Friends share information"), "{rendered}");
    assert!(rendered.contains("Results"), "{rendered}");
}
