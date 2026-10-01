//! Unit tests for the template registry.

use crate::metadata::DocumentType;

use super::*;

#[test]
fn the_five_specified_slide_themes_are_present() {
    assert_eq!(
        slide_style_names(),
        vec!["academic", "minimal", "dark", "bold", "mono"]
    );
}

#[test]
fn academic_is_the_default_slide_style() {
    assert_eq!(DEFAULT_SLIDE_STYLE, "academic");
    assert_eq!(
        builtin(DocumentType::Slides, None).unwrap(),
        slides("academic").unwrap()
    );
}

#[test]
fn every_slide_theme_implements_the_contract() {
    // The renderer calls exactly these three functions, so a theme missing one
    // would fail only at Typst compile time.
    for name in slide_style_names() {
        let source = slides(name).expect(name);
        for function in ["#let presentation(", "#let title-slide(", "#let slide("] {
            assert!(
                source.contains(function),
                "the `{name}` theme is missing `{function}`"
            );
        }
    }
}

#[test]
fn every_slide_theme_sets_a_sixteen_by_nine_page() {
    for name in slide_style_names() {
        let source = slides(name).expect(name);
        assert!(
            source.contains("presentation-16-9"),
            "the `{name}` theme does not set a widescreen page"
        );
    }
}

#[test]
fn every_slide_theme_is_distinct() {
    let sources: Vec<&str> = slide_style_names()
        .iter()
        .map(|n| slides(n).unwrap())
        .collect();
    for (index, source) in sources.iter().enumerate() {
        for (other_index, other) in sources.iter().enumerate().skip(index + 1) {
            assert_ne!(
                source,
                other,
                "themes {} and {} are identical",
                slide_style_names()[index],
                slide_style_names()[other_index]
            );
        }
    }
}

#[test]
fn style_lookup_ignores_case_and_surrounding_space() {
    assert!(slides("DARK").is_some());
    assert!(slides("  mono  ").is_some());
}

#[test]
fn an_unknown_style_is_reported_rather_than_silently_defaulted() {
    assert!(slides("chartreuse").is_none());

    let error = builtin(DocumentType::Slides, Some("chartreuse")).unwrap_err();
    assert_eq!(error, UnknownStyle("chartreuse".into()));
    assert!(error.message().contains("unknown style `chartreuse`"));
    // The message names what the user could have meant.
    for name in slide_style_names() {
        assert!(error.message().contains(name), "{}", error.message());
    }
}

#[test]
fn a_document_uses_the_single_document_template() {
    // v0.1 ships one excellent document style rather than five.
    assert_eq!(builtin(DocumentType::Document, None).unwrap(), document());
    // A style name is irrelevant in document mode rather than an error.
    assert_eq!(
        builtin(DocumentType::Document, Some("dark")).unwrap(),
        document()
    );
}

#[test]
fn the_document_template_implements_its_contract() {
    assert!(document().contains("#let article("));
}
