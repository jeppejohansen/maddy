//! Typst snapshot tests.
//!
//! Each fixture under `tests/fixtures/` is rendered and compared with the
//! committed Typst under `tests/snapshots/`. The point is to catch unintended
//! renderer changes: a diff here means the generated Typst moved, and the author
//! has to decide whether that was intended.
//!
//! Regenerate with:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test --test snapshot
//! ```

mod support;

use std::path::{Path, PathBuf};

use maddy::compiler::CompileOptions;
use maddy::diagnostics::Diagnostics;
use maddy::ir::{Document, Presentation};
use maddy::markdown;
use maddy::metadata::DocumentType;
use maddy::templates;
use maddy::typst::{emit_document, emit_presentation, RenderOptions};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn snapshots() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots")
}

/// Render a fixture to Typst source, in whichever mode its front matter asks
/// for.
fn render(name: &str) -> String {
    let path = fixtures().join(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));

    let front = markdown::parse_frontmatter(&source).expect("front matter");
    let options = CompileOptions::default();
    let effective = maddy::compiler::resolve_options(&front.metadata, &options);

    let parsed = markdown::parse(front.body, front.body_offset, effective.document_type);
    let document = Document::new(front.metadata, parsed.blocks);

    let template = templates::builtin(effective.document_type, effective.style.as_deref())
        .unwrap_or_else(|error| panic!("{name}: {}", error.message()));
    let render = RenderOptions::new(template);
    let mut diagnostics = Diagnostics::new();

    match effective.document_type {
        DocumentType::Document => emit_document(&document, &render, &mut diagnostics),
        DocumentType::Slides => {
            let (presentation, _) = Presentation::from_document(document);
            emit_presentation(&presentation, &render, &mut diagnostics)
        }
    }
    .unwrap_or_else(|error| panic!("rendering {name}: {error}"))
}

/// Compare rendered output with its snapshot, or write it when updating.
fn check(fixture: &str) {
    let rendered = render(fixture);
    let snapshot = snapshots().join(fixture.replace(".md", ".typ"));

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(snapshots()).expect("creating the snapshot directory");
        std::fs::write(&snapshot, &rendered).expect("writing the snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&snapshot).unwrap_or_else(|error| {
        panic!(
            "reading {}: {error}\n\nRun with UPDATE_SNAPSHOTS=1 to create it.",
            snapshot.display()
        )
    });

    if rendered != expected {
        let line = rendered
            .lines()
            .zip(expected.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(expected.lines().count());
        panic!(
            "{fixture} no longer matches its snapshot, first difference at line {}:\n\
             \x20 rendered: {:?}\n\
             \x20 snapshot: {:?}\n\n\
             Run with UPDATE_SNAPSHOTS=1 to accept the new output.",
            line + 1,
            rendered.lines().nth(line),
            expected.lines().nth(line),
        );
    }
}

#[test]
fn basic_matches_its_snapshot() {
    check("basic.md");
}

#[test]
fn regression_matches_its_snapshot() {
    check("regression.md");
}

#[test]
fn rich_matches_its_snapshot() {
    check("rich.md");
}

#[test]
fn talk_matches_its_snapshot() {
    check("talk.md");
}

#[test]
fn every_fixture_has_a_snapshot() {
    // While updating, snapshots are being written by the other tests in this
    // same run, so the check would race them.
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        return;
    }

    let mut missing = Vec::new();
    for entry in std::fs::read_dir(fixtures()).expect("reading the fixture directory") {
        let name = entry
            .expect("a fixture entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        if !name.ends_with(".md") {
            continue;
        }
        if !snapshots().join(name.replace(".md", ".typ")).exists() {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "fixtures without a snapshot: {missing:?}"
    );
}

#[test]
fn every_snapshot_compiles_under_typst() {
    if !support::typst_available() {
        support::skip("every_snapshot_compiles_under_typst");
        return;
    }

    for entry in std::fs::read_dir(snapshots()).expect("reading the snapshot directory") {
        let path = entry.expect("a snapshot entry").path();
        if path.extension().is_none_or(|extension| extension != "typ") {
            continue;
        }

        let directory = tempfile::tempdir().expect("a temporary directory");
        let source = directory.path().join("snapshot.typ");
        std::fs::copy(&path, &source).expect("copying the snapshot");

        if let Err(stderr) = support::compile(&source, &directory.path().join("snapshot.pdf")) {
            panic!("{} does not compile:\n{stderr}", path.display());
        }
    }
}
