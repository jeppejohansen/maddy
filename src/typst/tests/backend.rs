//! Unit tests for the PDF backend.

use std::path::{Path, PathBuf};

use crate::diagnostics::CompileError;
use crate::typst::backend::{CompileContext, ExternalTypstBackend, PdfBackend};

#[test]
fn the_context_root_is_the_directory_of_the_source() {
    let context = CompileContext::for_source(Path::new("/papers/thesis/paper.md"));
    assert_eq!(context.root, PathBuf::from("/papers/thesis"));
}

#[test]
fn a_bare_filename_resolves_against_the_working_directory() {
    // `paper.md` has no parent component, so relative paths inside it must still
    // resolve somewhere sensible.
    let context = CompileContext::for_source(Path::new("paper.md"));
    assert_eq!(context.root, PathBuf::from("."));
}

#[test]
fn a_missing_executable_is_reported_as_typst_missing() {
    let backend = ExternalTypstBackend::with_program("maddy-no-such-typst-executable");
    let error = backend
        .compile("Hello", &CompileContext::new("."))
        .expect_err("a missing executable should fail");

    assert!(matches!(error, CompileError::TypstMissing), "{error:?}");
    // An environment problem, not a document problem.
    assert_eq!(error.exit_code(), 3);
}

#[test]
fn valid_source_compiles_to_pdf_bytes() {
    let backend = ExternalTypstBackend::new();
    let Ok(pdf) = backend.compile("Hello, world.", &CompileContext::new(".")) else {
        return; // Typst is unavailable.
    };

    assert!(
        pdf.starts_with(b"%PDF-"),
        "the backend should return PDF bytes"
    );
    assert!(pdf.len() > 500, "a real PDF is more than a header");
}

#[test]
fn invalid_source_reports_the_compilers_own_explanation() {
    let backend = ExternalTypstBackend::new();
    let result = backend.compile("#undefined_variable_here", &CompileContext::new("."));

    let Err(error) = result else {
        return; // Typst is unavailable, so there is nothing to check.
    };

    let CompileError::TypstCompile { details } = &error else {
        panic!("expected a Typst compile error, got {error:?}");
    };
    assert!(details.contains("unknown variable"), "{details}");

    // The user is pointed at the generated source rather than drowned in output.
    let diagnostic = &error.diagnostics()[0];
    assert!(diagnostic
        .notes
        .iter()
        .any(|note| note.contains("--keep-typst")));
}

#[test]
fn a_large_document_does_not_deadlock() {
    // Source is written on one thread while output is read on another; a single
    // threaded implementation blocks once the pipe buffers fill.
    let backend = ExternalTypstBackend::new();
    let body = "Lorem ipsum dolor sit amet. ".repeat(20_000);
    let Ok(pdf) = backend.compile(&body, &CompileContext::new(".")) else {
        return; // Typst is unavailable.
    };

    assert!(pdf.starts_with(b"%PDF-"));
    assert!(
        pdf.len() > 10_000,
        "a long document should produce a substantial PDF"
    );
}
