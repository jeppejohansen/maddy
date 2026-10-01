//! Every slide theme, compiled.
//!
//! A theme is Typst source that the Rust renderer never inspects, so the only
//! way to know a theme works is to compile a deck with it. These tests drive
//! each theme through the real compiler with a deck that exercises the whole
//! theme contract — title slide, titled and untitled slides, headings, lists,
//! equations, a table, code, a quotation and a link.

mod support;

use std::process::Command;

use assert_cmd::prelude::*;

/// A deck exercising everything a theme must render.
const DECK: &str = r#"---
type: slides
title: Theme Check
subtitle: Every Construct
author: Jane Researcher
date: October 2026
---

# Motivation

Students make decisions in social environments.

- Friends share information
- Choices may be strategic complements

## A sub-heading

### A deeper heading

---

# Model

Student $i$ chooses $a_i \in \{0,1\}$ with

$$
p_i = \operatorname{logit}^{-1}\left(X_i^\top\beta + \lambda \bar p_{-i}\right).
$$

---

# Results

| Model | Effect |
|---|---:|
| No peers | 3.2% |
| Peer equilibrium | 4.0% |

---

# Code

```rust
fn estimate(x: Matrix, y: Vector) -> Vector {
    solve(x.t() * x, x.t() * y)
}
```

> Economic models are abstractions.

See [the paper](https://example.com/paper.pdf).

---

$$
\hat\beta = (X^\top X)^{-1} X^\top Y
$$
"#;

/// The themes the documentation promises.
const THEMES: &[&str] = &["academic", "minimal", "dark", "bold", "mono"];

fn mdpdf() -> Command {
    Command::cargo_bin("mdpdf").expect("the mdpdf binary should be built")
}

#[test]
fn every_theme_compiles_the_whole_contract() {
    if !support::typst_available() {
        support::skip("every_theme_compiles_the_whole_contract");
        return;
    }

    for theme in THEMES {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let input = support::write_markdown(directory.path(), "talk.md", DECK);

        mdpdf()
            .arg(&input)
            .args(["--style", theme])
            .assert()
            .try_success()
            .unwrap_or_else(|error| panic!("the `{theme}` theme failed to compile:\n{error}"));

        let bytes = std::fs::read(directory.path().join("talk.pdf"))
            .unwrap_or_else(|error| panic!("`{theme}` produced no PDF: {error}"));

        assert!(
            support::is_pdf(&bytes),
            "`{theme}` produced something that is not a PDF"
        );
        // A title slide plus five content slides.
        assert_eq!(
            support::pdf_page_count(&bytes),
            Some(6),
            "`{theme}` produced the wrong number of slides"
        );
    }
}

#[test]
fn every_theme_is_sixteen_by_nine() {
    if !support::typst_available() {
        support::skip("every_theme_is_sixteen_by_nine");
        return;
    }

    for theme in THEMES {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let input = support::write_markdown(directory.path(), "talk.md", DECK);
        mdpdf()
            .arg(&input)
            .args(["--style", theme])
            .assert()
            .success();

        let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("a PDF");
        let ratio = support::pdf_aspect_ratio(&bytes).expect("a page size");
        assert!(
            (ratio - 16.0 / 9.0).abs() < 0.01,
            "`{theme}` is {ratio}, not 16:9"
        );
    }
}

#[test]
fn a_theme_changes_appearance_without_changing_semantics() {
    if !support::typst_available() {
        support::skip("a_theme_changes_appearance_without_changing_semantics");
        return;
    }

    // The specification's requirement: switching styles must alter appearance
    // without altering parsing or slide segmentation.
    let mut sizes = Vec::new();

    for theme in THEMES {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let input = support::write_markdown(directory.path(), "talk.md", DECK);
        mdpdf()
            .arg(&input)
            .args(["--style", theme])
            .assert()
            .success();

        let bytes = std::fs::read(directory.path().join("talk.pdf")).expect("a PDF");
        assert_eq!(
            support::pdf_page_count(&bytes),
            Some(6),
            "`{theme}` changed segmentation"
        );
        sizes.push((theme, bytes.len()));
    }

    // Every theme renders the same deck differently, so no two produce an
    // identical PDF.
    for (index, (theme, size)) in sizes.iter().enumerate() {
        for (other, other_size) in sizes.iter().skip(index + 1) {
            assert_ne!(
                size, other_size,
                "`{theme}` and `{other}` rendered identically"
            );
        }
    }
}

#[test]
fn the_style_can_be_chosen_from_front_matter() {
    if !support::typst_available() {
        support::skip("the_style_can_be_chosen_from_front_matter");
        return;
    }

    let deck = DECK.replace("date: October 2026", "date: October 2026\nstyle: dark");
    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "talk.md", &deck);

    mdpdf().arg(&input).arg("--keep-typst").assert().success();

    let typst = std::fs::read_to_string(directory.path().join("talk.typ")).expect("talk.typ");
    assert!(
        typst.contains("The dark slide theme"),
        "front matter should select the theme"
    );
}

#[test]
fn a_command_line_style_overrides_the_front_matter() {
    if !support::typst_available() {
        support::skip("a_command_line_style_overrides_the_front_matter");
        return;
    }

    // The specification's example: `style: academic` overridden by `--style dark`.
    let deck = DECK.replace("date: October 2026", "date: October 2026\nstyle: academic");
    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "talk.md", &deck);

    mdpdf()
        .arg(&input)
        .args(["--style", "dark"])
        .arg("--keep-typst")
        .assert()
        .success();

    let typst = std::fs::read_to_string(directory.path().join("talk.typ")).expect("talk.typ");
    assert!(
        typst.contains("The dark slide theme"),
        "the command line should win"
    );
    assert!(!typst.contains("The academic slide theme"));
}
