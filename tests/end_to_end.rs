//! End-to-end tests.
//!
//! These drive the real binary, with the real Typst compiler, and assert on the
//! files it produces:
//!
//! ```text
//! Markdown → compiler → Typst → real Typst compiler → PDF
//! ```
//!
//! Per the specification they check that the process succeeds, that the PDF
//! exists, that it is non-empty and that it has the expected page count. They do
//! not compare raw PDF bytes.

mod support;

use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

/// The `maddy` binary.
fn maddy() -> Command {
    Command::cargo_bin("maddy").expect("the maddy binary should be built")
}

/// A temporary directory holding `name` with the given contents.
fn workspace(name: &str, contents: &str) -> (TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = support::write_markdown(directory.path(), name, contents);
    (directory, path)
}

/// Copy a committed fixture into a fresh directory.
fn fixture(name: &str) -> (TempDir, std::path::PathBuf) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let contents = std::fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("reading {}: {error}", source.display()));
    workspace(name, &contents)
}

const SIMPLE: &str =
    "# Heading\n\nSome **bold** text with $x_i^2$ inline.\n\n$$\n\\frac{x}{y}\n$$\n";

// -------------------------------------------------------------- the happy path

#[test]
fn compiling_a_markdown_file_produces_a_pdf_beside_it() {
    if !support::typst_available() {
        support::skip("compiling_a_markdown_file_produces_a_pdf_beside_it");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy().arg(&input).assert().success();

    let pdf = directory.path().join("paper.pdf");
    let bytes = std::fs::read(&pdf).expect("paper.pdf should exist");

    assert!(!bytes.is_empty(), "the PDF should not be empty");
    assert!(support::is_pdf(&bytes), "the output should be a PDF");
    assert_eq!(support::pdf_page_count(&bytes), Some(1));
}

#[test]
fn success_reports_what_was_compiled() {
    if !support::typst_available() {
        support::skip("success_reports_what_was_compiled");
        return;
    }

    let (_directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Compiled").and(predicate::str::contains("paper.pdf")));
}

#[test]
fn the_acceptance_document_compiles_without_intervention() {
    if !support::typst_available() {
        support::skip("the_acceptance_document_compiles_without_intervention");
        return;
    }

    // The document the specification requires to work.
    let (directory, input) = fixture("regression.md");
    maddy().arg(&input).assert().success();

    let bytes = std::fs::read(directory.path().join("regression.pdf")).expect("a PDF");
    assert!(support::is_pdf(&bytes));
    assert_eq!(support::pdf_page_count(&bytes), Some(1));
}

#[test]
fn a_document_using_every_supported_construct_compiles() {
    if !support::typst_available() {
        support::skip("a_document_using_every_supported_construct_compiles");
        return;
    }

    let (directory, input) = fixture("rich.md");
    maddy().arg(&input).assert().success();
    assert!(support::is_pdf(
        &std::fs::read(directory.path().join("rich.pdf")).expect("a PDF")
    ));
}

// ------------------------------------------------------------- output handling

#[test]
fn an_explicit_output_path_is_honoured() {
    if !support::typst_available() {
        support::skip("an_explicit_output_path_is_honoured");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    let output = directory.path().join("result.pdf");

    maddy()
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    assert!(output.exists(), "result.pdf should exist");
    assert!(
        !directory.path().join("paper.pdf").exists(),
        "the default name should not be used"
    );
}

#[test]
fn emitting_typst_writes_only_typst() {
    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .args(["--emit", "typst"])
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    assert!(
        typst.contains("#let article("),
        "the template should be inlined"
    );
    assert!(typst.contains("= Heading"));
    assert!(
        !directory.path().join("paper.pdf").exists(),
        "no PDF should be written"
    );
}

#[test]
fn emitting_typst_needs_no_typst_installation() {
    // The whole point of the suggestion in the missing-Typst error.
    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .args(["--emit", "typst"])
        .env("PATH", directory.path())
        .assert()
        .success();
    assert!(directory.path().join("paper.typ").exists());
}

#[test]
fn emitting_both_writes_typst_and_pdf() {
    if !support::typst_available() {
        support::skip("emitting_both_writes_typst_and_pdf");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .args(["--emit", "both"])
        .assert()
        .success();

    assert!(
        directory.path().join("paper.typ").exists(),
        "paper.typ should exist"
    );
    assert!(
        directory.path().join("paper.pdf").exists(),
        "paper.pdf should exist"
    );
}

#[test]
fn keep_typst_preserves_the_intermediate_alongside_the_pdf() {
    if !support::typst_available() {
        support::skip("keep_typst_preserves_the_intermediate_alongside_the_pdf");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy().arg(&input).arg("--keep-typst").assert().success();

    assert!(
        directory.path().join("paper.typ").exists(),
        "paper.typ should be kept"
    );
    assert!(
        directory.path().join("paper.pdf").exists(),
        "paper.pdf should exist"
    );
}

#[test]
fn compiling_leaves_no_stray_files_behind() {
    if !support::typst_available() {
        support::skip("compiling_leaves_no_stray_files_behind");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy().arg(&input).assert().success();

    let mut names: Vec<String> = std::fs::read_dir(directory.path())
        .expect("reading the directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["paper.md", "paper.pdf"]);
}

// -------------------------------------------------------------------- images

#[test]
fn a_relative_image_path_resolves_against_the_markdown_file() {
    if !support::typst_available() {
        support::skip("a_relative_image_path_resolves_against_the_markdown_file");
        return;
    }

    let (directory, input) = workspace("paper.md", "# Figure\n\n![A dot](figures/dot.png)\n");
    let figures = directory.path().join("figures");
    std::fs::create_dir(&figures).expect("creating figures/");
    std::fs::write(figures.join("dot.png"), one_pixel_png()).expect("writing the image");

    maddy().arg(&input).assert().success();
    assert!(directory.path().join("paper.pdf").exists());
}

#[test]
fn a_remote_image_warns_but_still_compiles() {
    if !support::typst_available() {
        support::skip("a_remote_image_warns_but_still_compiles");
        return;
    }

    let (_directory, input) = workspace(
        "paper.md",
        "# Figure\n\n![Plot](https://example.com/plot.png)\n",
    );

    maddy()
        .arg(&input)
        .assert()
        .success()
        .stderr(predicate::str::contains("remote images are not supported"));
}

// ------------------------------------------------------------------ diagnostics

#[test]
fn an_unknown_math_command_fails_and_points_at_the_markdown() {
    let (_directory, input) = workspace("paper.md", "# T\n\nWe have $\\foo{x}$ here.\n");

    maddy().arg(&input).assert().code(1).stderr(
        predicate::str::contains("paper.md:3:10")
            .and(predicate::str::contains("\\foo"))
            .and(predicate::str::contains("^^^^^^^")),
    );
}

#[test]
fn an_unsupported_math_construct_says_what_it_parsed() {
    let (_directory, input) = workspace("paper.md", "$$\n\\color{red} x\n$$\n");

    maddy()
        .arg(&input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Parsed as: coloured mathematics"));
}

#[test]
fn raw_html_warns_but_succeeds() {
    if !support::typst_available() {
        support::skip("raw_html_warns_but_succeeds");
        return;
    }

    let (_directory, input) = workspace("paper.md", "# T\n\n<div>hello</div>\n");

    maddy()
        .arg(&input)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: raw HTML is not supported",
        ));
}

#[test]
fn strict_turns_a_warning_into_an_error() {
    let (directory, input) = workspace("paper.md", "# T\n\n<div>hello</div>\n");

    maddy()
        .arg(&input)
        .arg("--strict")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("error: raw HTML is not supported"));

    assert!(
        !directory.path().join("paper.pdf").exists(),
        "nothing should be written"
    );
}

#[test]
fn an_unknown_front_matter_field_warns_but_succeeds() {
    if !support::typst_available() {
        support::skip("an_unknown_front_matter_field_warns_but_succeeds");
        return;
    }

    let (_directory, input) = workspace(
        "paper.md",
        "---\ntitle: T\nkeywords: economics\n---\n\nBody.\n",
    );

    maddy()
        .arg(&input)
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "unknown front-matter field `keywords`",
        ));
}

#[test]
fn an_invalid_document_type_is_rejected() {
    let (_directory, input) = workspace("paper.md", "---\ntype: poster\n---\n\nBody.\n");

    maddy()
        .arg(&input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("unknown document type `poster`"));
}

#[test]
fn a_missing_input_file_is_an_environment_error() {
    maddy()
        .arg("does-not-exist.md")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("reading does-not-exist.md"));
}

#[test]
fn compiling_needs_no_typst_installation() {
    // The compiler is embedded, so an empty PATH changes nothing. This is the
    // whole point of the embedded backend, and the inverse of the test that
    // stood here when Typst was a subprocess.
    let (directory, input) = workspace("paper.md", SIMPLE);

    maddy()
        .arg(&input)
        .env("PATH", directory.path())
        .assert()
        .success();

    let bytes = std::fs::read(directory.path().join("paper.pdf")).expect("paper.pdf");
    assert!(support::is_pdf(&bytes));
    assert_eq!(support::pdf_page_count(&bytes), Some(1));
}

// -------------------------------------------------------------------- logging

#[test]
fn quiet_prints_nothing_on_success() {
    if !support::typst_available() {
        support::skip("quiet_prints_nothing_on_success");
        return;
    }

    let (_directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .arg("-q")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn verbose_reports_each_stage() {
    if !support::typst_available() {
        support::skip("verbose_reports_each_stage");
        return;
    }

    let (_directory, input) = workspace("paper.md", SIMPLE);
    maddy().arg(&input).arg("-v").assert().success().stderr(
        predicate::str::contains("Reading metadata...")
            .and(predicate::str::contains("Parsing Markdown..."))
            .and(predicate::str::contains("Parsing 2 math expression(s)..."))
            .and(predicate::str::contains("Generating Typst..."))
            .and(predicate::str::contains("Running Typst...")),
    );
}

#[test]
fn the_help_and_version_flags_work() {
    maddy()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--slides"));
    maddy()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("maddy"));
}

#[test]
fn no_arguments_is_a_usage_error() {
    maddy()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

/// The smallest valid PNG: one opaque pixel.
fn one_pixel_png() -> Vec<u8> {
    const DATA: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xdd, 0x8d, 0xb0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    DATA.to_vec()
}

// -------------------------------------------------------------- presentations

/// The three-slide deck from the specification's first slides stopping point.
const TALK: &str =
    "---\ntype: slides\ntitle: Example Presentation\nauthor: Jane Researcher\n---\n\n\
                    # Question\n\nWhy does this matter?\n\n---\n\n\
                    # Model\n\n$$\nY_i = X_i^\\top \\beta + \\varepsilon_i\n$$\n\n---\n\n\
                    # Result\n\nThe coefficient is **positive**.\n";

#[test]
fn front_matter_selects_presentation_mode() {
    if !support::typst_available() {
        support::skip("front_matter_selects_presentation_mode");
        return;
    }

    let (directory, input) = workspace("talk.md", TALK);
    maddy().arg(&input).assert().success();

    let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("talk.pdf");
    // A title slide plus three content slides.
    assert_eq!(support::pdf_page_count(&bytes), Some(4));
}

#[test]
fn a_presentation_is_sixteen_by_nine() {
    if !support::typst_available() {
        support::skip("a_presentation_is_sixteen_by_nine");
        return;
    }

    let (directory, input) = workspace("talk.md", TALK);
    maddy().arg(&input).assert().success();

    let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("talk.pdf");
    let ratio = support::pdf_aspect_ratio(&bytes).expect("a page size");
    assert!(
        (ratio - 16.0 / 9.0).abs() < 0.01,
        "expected 16:9, got {ratio}"
    );
}

#[test]
fn the_slides_flag_overrides_the_document_type() {
    if !support::typst_available() {
        support::skip("the_slides_flag_overrides_the_document_type");
        return;
    }

    // The document says `document`; the command line says slides, and wins.
    let source = "---\ntype: document\n---\n\n# One\n\ntext\n\n---\n\n# Two\n\ntext\n";
    let (directory, input) = workspace("talk.md", source);

    maddy().arg(&input).arg("--slides").assert().success();

    let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("talk.pdf");
    // Two slides, and no title slide since there is no title.
    assert_eq!(support::pdf_page_count(&bytes), Some(2));
}

#[test]
fn a_separator_is_a_rule_in_document_mode_and_a_break_in_slides_mode() {
    if !support::typst_available() {
        support::skip("a_separator_is_a_rule_in_document_mode_and_a_break_in_slides_mode");
        return;
    }

    let source = "# One\n\ntext\n\n---\n\n# Two\n\ntext\n";

    let (document_directory, document_input) = workspace("paper.md", source);
    maddy().arg(&document_input).assert().success();
    let document = std::fs::read(document_directory.path().join("paper.pdf")).expect("a PDF");
    assert_eq!(
        support::pdf_page_count(&document),
        Some(1),
        "a document is one flowing page"
    );

    let (slides_directory, slides_input) = workspace("talk.md", source);
    maddy()
        .arg(&slides_input)
        .arg("--slides")
        .assert()
        .success();
    let slides = std::fs::read(slides_directory.path().join("talk.pdf")).expect("a PDF");
    assert_eq!(
        support::pdf_page_count(&slides),
        Some(2),
        "slides are cut at the separator"
    );
}

#[test]
fn the_presentation_acceptance_deck_compiles() {
    if !support::typst_available() {
        support::skip("the_presentation_acceptance_deck_compiles");
        return;
    }

    // The deck the specification requires to work.
    let (directory, input) = fixture("talk.md");
    maddy().arg(&input).assert().success();

    let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("talk.pdf");
    assert!(support::is_pdf(&bytes));
    // A generated title slide plus four content slides.
    assert_eq!(support::pdf_page_count(&bytes), Some(5));

    let ratio = support::pdf_aspect_ratio(&bytes).expect("a page size");
    assert!(
        (ratio - 16.0 / 9.0).abs() < 0.01,
        "expected 16:9, got {ratio}"
    );
}

#[test]
fn a_deck_without_a_title_has_no_title_slide() {
    if !support::typst_available() {
        support::skip("a_deck_without_a_title_has_no_title_slide");
        return;
    }

    let (directory, input) = workspace(
        "talk.md",
        "---\ntype: slides\n---\n\n# One\n\ntext\n\n---\n\n# Two\n\ntext\n",
    );
    maddy().arg(&input).assert().success();

    let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("talk.pdf");
    assert_eq!(support::pdf_page_count(&bytes), Some(2));
}

#[test]
fn two_titles_on_one_slide_warns() {
    if !support::typst_available() {
        support::skip("two_titles_on_one_slide_warns");
        return;
    }

    let (_directory, input) = workspace(
        "talk.md",
        "---\ntype: slides\n---\n\n# One\n\n# Two\n\ntext\n",
    );

    maddy()
        .arg(&input)
        .assert()
        .success()
        .stderr(predicate::str::contains("more than one level-one heading"));
}

#[test]
fn an_unknown_style_is_a_configuration_error_naming_the_choices() {
    let (_directory, input) = workspace("talk.md", TALK);

    maddy()
        .arg(&input)
        .args(["--style", "chartreuse"])
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("unknown style `chartreuse`")
                .and(predicate::str::contains("academic")),
        );
}

// ------------------------------------------------- generated-Typst failures

/// A template that is valid Rust-side but fails to compile, standing in for the
/// two things that can go wrong here: an emitter bug or an invalid user
/// template.
const BROKEN_TEMPLATE: &str = "#let article(title: none, subtitle: none, author: none, \
                               date: none, body) = {\n  #one\n  #two\n  #three\n  #four\n  \
                               #five\n  body\n}\n";

#[test]
fn a_failing_template_reports_concisely_and_points_at_keep_typst() {
    if !support::typst_available() {
        support::skip("a_failing_template_reports_concisely_and_points_at_keep_typst");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    let template = directory.path().join("broken.typ");
    std::fs::write(&template, BROKEN_TEMPLATE).expect("writing the template");

    let output = maddy()
        .arg(&input)
        .arg("--template")
        .arg(&template)
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("generated Typst failed to compile")
                .and(predicate::str::contains(
                    "run with --verbose for the full output",
                ))
                .and(predicate::str::contains("--keep-typst")),
        )
        .get_output()
        .stderr
        .clone();

    // Concise by default: pages of subprocess output would bury the first line.
    let lines = String::from_utf8_lossy(&output).lines().count();
    assert!(
        lines < 20,
        "the default report should be short, got {lines} lines"
    );
}

#[test]
fn verbose_shows_the_whole_compiler_output() {
    let (directory, input) = workspace("paper.md", SIMPLE);
    let template = directory.path().join("broken.typ");
    std::fs::write(&template, BROKEN_TEMPLATE).expect("writing the template");

    let output = maddy()
        .arg(&input)
        .arg("-v")
        .arg("--template")
        .arg(&template)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Full output:"))
        .get_output()
        .stderr
        .clone();

    // The default report stops after a few lines; the full output carries the
    // later errors too.
    let text = String::from_utf8_lossy(&output);
    let errors = text
        .matches("the character `#` is not valid in code")
        .count();
    assert!(
        errors > 2,
        "expected the withheld errors as well, saw {errors}"
    );
}

#[test]
fn a_missing_template_is_an_environment_error() {
    let (_directory, input) = workspace("paper.md", SIMPLE);

    maddy()
        .arg(&input)
        .args(["--template", "no-such-template.typ"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "reading template no-such-template.typ",
        ));
}

// ------------------------------------------------------------ configuration

/// The configuration example from the specification.
const CONFIG: &str = "[document]\npaper = \"a5\"\nfont = \"Libertinus Serif\"\n\
                      font-size = \"13pt\"\n\n[page]\nmargin = \"15mm\"\n\n\
                      [headings]\nnumbered = true\n";

#[test]
fn configuration_beside_the_document_is_applied() {
    if !support::typst_available() {
        support::skip("configuration_beside_the_document_is_applied");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    std::fs::write(directory.path().join("maddy.toml"), CONFIG).expect("writing config");

    maddy().arg(&input).arg("--keep-typst").assert().success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    assert!(typst.contains("// From maddy.toml"), "{typst}");
    assert!(
        typst.contains(r#"#set page(paper: "a5", margin: 15mm)"#),
        "{typst}"
    );
    assert!(
        typst.contains(r#"#set text(font: "Libertinus Serif", size: 13pt)"#),
        "{typst}"
    );
    assert!(
        typst.contains(r#"#set heading(numbering: "1.1")"#),
        "{typst}"
    );
}

#[test]
fn configuration_rules_follow_the_template_so_they_override_it() {
    // The template applies its own rules first; configuration has to come after
    // or it would be the thing being overridden.
    let (directory, input) = workspace("paper.md", SIMPLE);
    std::fs::write(directory.path().join("maddy.toml"), CONFIG).expect("writing config");

    maddy()
        .arg(&input)
        .args(["--emit", "typst"])
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    let template = typst.find("#let article(").expect("the template");
    let rules = typst
        .find("// From maddy.toml")
        .expect("the configuration rules");
    let body = typst.find("= Heading").expect("the body");

    assert!(
        template < rules && rules < body,
        "configuration sits between the two"
    );
}

#[test]
fn an_explicit_config_path_is_used() {
    let (directory, input) = workspace("paper.md", SIMPLE);
    // One beside the document, which must be ignored in favour of the explicit
    // one.
    std::fs::write(
        directory.path().join("maddy.toml"),
        "[page]\nmargin = \"99mm\"\n",
    )
    .expect("writing the nearby config");
    let explicit = directory.path().join("other.toml");
    std::fs::write(&explicit, "[page]\nmargin = \"7mm\"\n").expect("writing the explicit config");

    maddy()
        .arg(&input)
        .arg("--config")
        .arg(&explicit)
        .args(["--emit", "typst"])
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    assert!(typst.contains("margin: 7mm"), "{typst}");
    assert!(!typst.contains("margin: 99mm"), "{typst}");
}

#[test]
fn a_broken_configuration_file_is_a_configuration_error() {
    let (directory, input) = workspace("paper.md", SIMPLE);
    std::fs::write(
        directory.path().join("maddy.toml"),
        "[document]\npapers = \"a4\"\n",
    )
    .expect("writing config");

    maddy()
        .arg(&input)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("maddy.toml").and(predicate::str::contains("papers")));
}

#[test]
fn a_missing_explicit_config_is_an_environment_error() {
    let (_directory, input) = workspace("paper.md", SIMPLE);

    maddy()
        .arg(&input)
        .args(["--config", "no-such-config.toml"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "reading configuration no-such-config.toml",
        ));
}

#[test]
fn the_full_precedence_chain_resolves_in_order() {
    // Configuration, then front matter, then the command line.
    let with_style = |front: &str, args: &[&str], expected: &str| {
        let source = format!("---\ntype: slides\n{front}---\n\n# One\n\ntext\n");
        let (directory, input) = workspace("talk.md", &source);
        std::fs::write(
            directory.path().join("maddy.toml"),
            "[slides]\nstyle = \"mono\"\n",
        )
        .expect("writing config");

        let mut command = maddy();
        command
            .arg(&input)
            .args(["--emit", "typst"])
            .args(args)
            .assert()
            .success();

        let typst = std::fs::read_to_string(directory.path().join("talk.typ")).expect("talk.typ");
        assert!(
            typst.contains(expected),
            "expected {expected:?} in the generated source"
        );
    };

    // Configuration alone.
    with_style("", &[], "The mono slide theme");
    // Front matter beats configuration.
    with_style("style: dark\n", &[], "The dark slide theme");
    // The command line beats both.
    with_style(
        "style: dark\n",
        &["--style", "bold"],
        "The bold slide theme",
    );
}

#[test]
fn a_custom_template_still_receives_configuration() {
    if !support::typst_available() {
        support::skip("a_custom_template_still_receives_configuration");
        return;
    }

    let (directory, input) = workspace("paper.md", SIMPLE);
    let template = directory.path().join("custom.typ");
    std::fs::write(
        &template,
        "#let article(title: none, subtitle: none, author: none, date: none, body) = {\n  \
         set page(paper: \"a4\")\n  body\n}\n",
    )
    .expect("writing the template");
    std::fs::write(
        directory.path().join("maddy.toml"),
        "[page]\nmargin = \"7mm\"\n",
    )
    .expect("writing config");

    maddy()
        .arg(&input)
        .arg("--template")
        .arg(&template)
        .args(["--emit", "typst"])
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    assert!(typst.contains("margin: 7mm"), "{typst}");
}

#[test]
fn no_configuration_file_is_not_an_error() {
    let (directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .args(["--emit", "typst"])
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("paper.typ")).expect("paper.typ");
    assert!(!typst.contains("// From maddy.toml"), "{typst}");
}

#[test]
fn emitting_both_reports_one_line_listing_each_output() {
    if !support::typst_available() {
        support::skip("emitting_both_reports_one_line_listing_each_output");
        return;
    }

    let (_directory, input) = workspace("paper.md", SIMPLE);
    let output = maddy()
        .arg(&input)
        .args(["--emit", "both"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8_lossy(&output);
    assert_eq!(
        text.lines().count(),
        1,
        "one compilation should report one line: {text}"
    );
    assert!(text.contains("paper.typ"), "{text}");
    assert!(text.contains("paper.pdf"), "{text}");
}

#[test]
fn emitting_typst_only_reports_the_typst_file() {
    let (_directory, input) = workspace("paper.md", SIMPLE);
    maddy()
        .arg(&input)
        .args(["--emit", "typst"])
        .assert()
        .success()
        .stdout(predicate::str::contains("paper.typ").and(predicate::str::contains(".pdf").not()));
}
