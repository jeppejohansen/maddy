//! Unit tests for the template registry.

use crate::metadata::DocumentType;

use super::*;

#[test]
fn the_documented_options_are_what_the_registry_offers() {
    assert_eq!(
        STYLES,
        &["modern", "academic", "editorial", "bold", "technical"]
    );
    assert_eq!(
        PALETTES,
        &[
            "midnight",
            "bottle",
            "ochre",
            "claret",
            "aubergine",
            "petrol",
            "midnight-dark",
            "bottle-dark"
        ]
    );
    assert_eq!(RATIOS, &["16:9", "3:2", "4:3"]);
    assert_eq!(TEXT_SIZES, &["small", "medium", "large"]);
}

#[test]
fn the_defaults_are_valid_options() {
    assert_eq!(style(None).unwrap(), DEFAULT_STYLE);
    assert_eq!(palette(None).unwrap(), DEFAULT_PALETTE);
    assert_eq!(ratio(None).unwrap(), DEFAULT_RATIO);
    assert_eq!(text_size(None).unwrap(), DEFAULT_TEXT_SIZE);
}

#[test]
fn values_are_matched_case_insensitively_and_canonicalized() {
    // The template is indexed by the canonical spelling, so what comes back
    // must be that rather than what the user typed.
    assert_eq!(style(Some("  ACADEMIC ")).unwrap(), "academic");
    assert_eq!(palette(Some("Midnight-Dark")).unwrap(), "midnight-dark");
    assert_eq!(ratio(Some(" 4:3 ")).unwrap(), "4:3");
    assert_eq!(text_size(Some("LARGE")).unwrap(), "large");
}

#[test]
fn an_unknown_value_is_refused_and_names_the_alternatives() {
    for (message, bad) in [
        (style(Some("chartreuse")).unwrap_err(), "style"),
        (palette(Some("chartreuse")).unwrap_err(), "palette"),
        (ratio(Some("21:9")).unwrap_err(), "ratio"),
        (text_size(Some("enormous")).unwrap_err(), "text size"),
    ] {
        assert_eq!(message.axis, bad);
        assert!(
            message.message().contains("unknown"),
            "{}",
            message.message()
        );
        // The message must say what the user could have meant instead.
        assert!(
            message.message().contains("expected one of"),
            "{}",
            message.message()
        );
    }
}

#[test]
fn an_explicit_text_size_is_passed_through() {
    // A length keeps the style's own multipliers shaping everything around it,
    // which is the point of overriding only the base size.
    assert_eq!(text_size(Some("22pt")).unwrap(), "22pt");
    assert_eq!(text_size(Some(" 1.5em ")).unwrap(), "1.5em");
    assert_eq!(text_size(Some("12mm")).unwrap(), "12mm");
}

#[test]
fn a_malformed_text_size_is_refused() {
    for bad in ["22", "pt", "-3pt", "0pt", "22px", "big enough"] {
        assert!(text_size(Some(bad)).is_err(), "{bad} should be refused");
    }
}

#[test]
fn every_style_and_palette_appears_in_the_slide_template() {
    // The registry and the template must agree, or a valid name would reach
    // Typst and fail there instead of here.
    let source = slides();
    for name in STYLES {
        assert!(
            source.contains(&format!("#let {name} = (")),
            "no `{name}` style in the template"
        );
    }
    for name in PALETTES {
        let declaration = format!("#let {name} = (");
        assert!(
            source.contains(&declaration),
            "no `{name}` palette in the template"
        );
    }
}

#[test]
fn the_slide_template_implements_the_renderer_contract() {
    let source = slides();
    for function in ["#let presentation(", "#let title-slide(", "#let slide("] {
        assert!(
            source.contains(function),
            "the slide template is missing `{function}`"
        );
    }
}

#[test]
fn styles_carry_no_colour_of_their_own() {
    // A style that hard-coded a colour would break the promise that any style
    // composes with any palette. Colour literals belong to palettes only.
    let source = slides();
    let styles_section = source
        .split("// ============================================================ styles")
        .nth(1)
        .expect("a styles section")
        .split("// ============================================================ composition")
        .next()
        .expect("the section ends");

    assert!(
        !styles_section.contains("rgb(") && !styles_section.contains("luma("),
        "a style declares a colour literal"
    );
}

#[test]
fn every_palette_exposes_the_same_tokens() {
    let source = slides();
    const TOKENS: &[&str] = &[
        "primary:",
        "primary-dark:",
        "secondary:",
        "accent:",
        "background:",
        "surface:",
        "text:",
        "text-muted:",
        "border:",
    ];
    for name in PALETTES {
        let body = source
            .split(&format!("#let {name} = ("))
            .nth(1)
            .unwrap_or_else(|| panic!("no `{name}` palette"))
            .split(")\n")
            .next()
            .expect("the palette ends");
        for token in TOKENS {
            assert!(
                body.contains(token),
                "the `{name}` palette is missing `{token}`"
            );
        }
    }
}

#[test]
fn a_document_uses_the_single_document_template() {
    assert_eq!(builtin(DocumentType::Document), document());
    assert!(document().contains("#let article("));
}

#[test]
fn slides_use_the_slide_system() {
    assert_eq!(builtin(DocumentType::Slides), slides());
}
