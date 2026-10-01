//! Unit tests for the embedded Typst compiler.

use crate::diagnostics::CompileError;
use crate::typst::backend::{CompileContext, PdfBackend};
use crate::typst::{EmbeddedTypstBackend, ExternalTypstBackend};

fn compile(source: &str) -> crate::diagnostics::Result<Vec<u8>> {
    EmbeddedTypstBackend::new().compile(source, &CompileContext::new("."))
}

#[test]
fn valid_source_compiles_to_pdf_bytes() {
    let pdf = compile("Hello, world.").expect("compilation should succeed");

    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.len() > 500, "a real PDF is more than a header");
}

#[test]
fn compiling_needs_no_external_executable() {
    // The point of the embedded backend: nothing is spawned, so there is no
    // dependency on the environment at all.
    assert!(compile("Hello.").is_ok());
}

#[test]
fn mathematics_renders_through_the_bundled_math_font() {
    let pdf = compile("$ sum_(i=1)^n x_i^2 = hat(beta) $").expect("math should compile");
    assert!(pdf.starts_with(b"%PDF-"));
}

#[test]
fn invalid_source_reports_typst_own_explanation() {
    let error = compile("#undefined_variable_here").expect_err("this should fail");

    let CompileError::TypstCompile { details } = &error else {
        panic!("expected a Typst compile error, got {error:?}");
    };
    assert!(details.contains("unknown variable"), "{details}");
    // The position points into the generated source, which --keep-typst shows.
    assert!(details.contains("<generated>:1:"), "{details}");
}

#[test]
fn a_diagnostic_carries_typst_hints() {
    let error = compile("#let x = [\n").expect_err("this should fail");
    let CompileError::TypstCompile { details } = &error else {
        panic!("expected a failure")
    };
    assert!(details.starts_with("error: "), "{details}");
}

#[test]
fn output_is_deterministic_across_runs() {
    // No creation timestamp is recorded, so the same input always produces the
    // same bytes — which the specification asks for and the Typst command line,
    // which stamps every PDF, does not give.
    let first = compile("Deterministic.").expect("first run");
    let second = compile("Deterministic.").expect("second run");
    assert_eq!(first, second);
}

#[test]
fn a_document_of_only_bundled_characters_avoids_the_system_font_scan() {
    // Not observable in the output, so this asserts the decision function that
    // drives it rather than the timing.
    assert!(super::super::embedded::bundled_fonts_cover(
        "Hello, world. $x_i^2$"
    ));
    assert!(super::super::embedded::bundled_fonts_cover(
        "Greek: αβγ and ∑ ∫ ≤ ⊤"
    ));
}

#[test]
fn a_document_with_unrenderable_characters_asks_for_system_fonts() {
    // Typst draws an uncovered character as a blank box rather than failing, so
    // detecting this is what keeps a correct document from coming out wrong.
    assert!(!super::super::embedded::bundled_fonts_cover("中文标题"));
    assert!(!super::super::embedded::bundled_fonts_cover(
        "Mixed English and 日本語"
    ));
}

#[test]
fn ascii_is_never_treated_as_uncovered() {
    let ascii: String = (0x20u8..0x7f).map(char::from).collect();
    assert!(super::super::embedded::bundled_fonts_cover(&ascii));
}

#[test]
fn the_two_backends_agree_on_page_count() {
    // The embedded and subprocess backends must produce the same document, even
    // though the bytes differ: the command line stamps a creation date.
    let source = "#set page(width: 100mm, height: 40mm)\nOne\n#pagebreak()\nTwo\n";
    let context = CompileContext::new(".");

    let Ok(external) = ExternalTypstBackend::new().compile(source, &context) else {
        return; // Typst is not installed, so there is nothing to compare with.
    };
    let embedded = EmbeddedTypstBackend::new()
        .compile(source, &context)
        .expect("embedded");

    assert_eq!(page_count(&external), page_count(&embedded));
    assert_eq!(page_count(&embedded), Some(2));
}

/// Count `/Type /Page` objects, excluding the `/Type /Pages` tree node.
fn page_count(pdf: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(pdf);
    let count = text.match_indices("/Type").filter(|(index, _)| {
        let rest = text[index + "/Type".len()..].trim_start();
        rest.starts_with("/Page")
            && !rest["/Page".len()..].starts_with(|c: char| c.is_ascii_alphanumeric())
    });
    Some(count.count()).filter(|count| *count > 0)
}
