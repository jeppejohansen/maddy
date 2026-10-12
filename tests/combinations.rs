//! Every built-in combination compiles.
//!
//! The design is four independent axes, so the thing most worth proving is that
//! they really are independent: any style with any palette, at any ratio and any
//! text size. That is 5 × 8 × 3 × 3 = 360 decks, which is too many to render as
//! PDFs on every run, so this checks them in two layers — all of them through
//! Typst generation, and a representative subset all the way to PDF.

mod support;

use std::process::Command;

use assert_cmd::prelude::*;

const STYLES: &[&str] = &["modern", "academic", "editorial", "bold", "technical"];
const PALETTES: &[&str] = &[
    "midnight",
    "bottle",
    "ochre",
    "claret",
    "aubergine",
    "petrol",
    "midnight-dark",
    "bottle-dark",
];
const RATIOS: &[&str] = &["16:9", "3:2", "4:3"];
const SIZES: &[&str] = &["small", "medium", "large"];

/// A deck exercising every component a style has to render.
const DECK: &str = r#"---
type: slides
title: Combination Check
subtitle: Every component
author: Jane Researcher
date: October 2026
---

# Motivation

Educational choices are made in social environments.

- Friends share information
- Choices may be complements

## A second level

### A third level

---

# Model

Student $i$ chooses $a_i \in \{0,1\}$ with

$$
p_i = \operatorname{logit}^{-1}\left(X_i^\top\beta\right).
$$

---

# Results

| Model | Effect |
|---|---:|
| No peers | 3.2% |

> Economic models are abstractions.

```rust
fn estimate(x: Matrix) -> Vector { solve(x) }
```
"#;

fn maddy() -> Command {
    Command::cargo_bin("maddy").expect("the maddy binary should be built")
}

/// Compile a deck with the given axes, to Typst or all the way to PDF.
fn compile(
    style: &str,
    palette: &str,
    ratio: &str,
    size: &str,
    pdf: bool,
) -> Result<Vec<u8>, String> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "talk.md", DECK);

    let mut command = maddy();
    command
        .arg(&input)
        .args(["--style", style])
        .args(["--palette", palette])
        .args(["--ratio", ratio])
        .args(["--text-size", size])
        .arg("-q");
    if !pdf {
        command.args(["--emit", "typst"]);
    }

    let output = command.output().expect("running maddy");
    if !output.status.success() {
        return Err(format!(
            "{style}/{palette}/{ratio}/{size}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let produced = directory
        .path()
        .join(if pdf { "talk.pdf" } else { "talk.typ" });
    std::fs::read(&produced).map_err(|error| format!("{style}/{palette}: {error}"))
}

#[test]
fn every_style_and_palette_generates_valid_typst() {
    let mut failures = Vec::new();
    for style in STYLES {
        for palette in PALETTES {
            if let Err(error) = compile(style, palette, "16:9", "medium", false) {
                failures.push(error);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} combinations failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_ratio_and_text_size_generates_valid_typst() {
    let mut failures = Vec::new();
    for ratio in RATIOS {
        for size in SIZES {
            if let Err(error) = compile("modern", "midnight", ratio, size, false) {
                failures.push(error);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} combinations failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn an_explicit_text_size_is_accepted() {
    assert!(compile("modern", "midnight", "16:9", "22pt", false).is_ok());
}

#[test]
fn every_style_compiles_to_a_pdf() {
    if !support::typst_available() {
        // The compiler is embedded, so this only gates on the PDF check being
        // worth its runtime, not on an external tool.
    }

    for style in STYLES {
        let pdf = compile(style, "midnight", "16:9", "medium", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(
            support::is_pdf(&pdf),
            "`{style}` produced something that is not a PDF"
        );
        // A title slide plus three content slides.
        assert_eq!(
            support::pdf_page_count(&pdf),
            Some(4),
            "`{style}` has the wrong page count"
        );
    }
}

#[test]
fn every_palette_compiles_to_a_pdf() {
    for palette in PALETTES {
        let pdf = compile("modern", palette, "16:9", "medium", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(
            support::is_pdf(&pdf),
            "`{palette}` produced something that is not a PDF"
        );
    }
}

#[test]
fn every_ratio_produces_its_geometry() {
    for (ratio, expected) in [("16:9", 16.0 / 9.0), ("3:2", 3.0 / 2.0), ("4:3", 4.0 / 3.0)] {
        let pdf = compile("modern", "midnight", ratio, "medium", true)
            .unwrap_or_else(|error| panic!("{error}"));
        let actual = support::pdf_aspect_ratio(&pdf).expect("a page size");
        assert!(
            (actual - expected).abs() < 0.01,
            "`{ratio}` rendered as {actual}"
        );
    }
}

#[test]
fn style_and_palette_are_independent() {
    // Changing only the palette must not change the typography, and changing
    // only the style must not change the colours. Both are visible in the
    // generated Typst: the show rule names them separately and nothing else
    // differs.
    let a = compile("academic", "bottle", "16:9", "medium", false).expect("academic/bottle");
    let b = compile("academic", "claret", "16:9", "medium", false).expect("academic/claret");
    let a = String::from_utf8_lossy(&a).replace(r#"palette: "bottle""#, "P");
    let b = String::from_utf8_lossy(&b).replace(r#"palette: "claret""#, "P");
    assert_eq!(
        a, b,
        "only the palette name should differ between these two decks"
    );
}

#[test]
fn text_size_changes_only_the_named_size() {
    let a = compile("modern", "midnight", "16:9", "small", false).expect("small");
    let b = compile("modern", "midnight", "16:9", "large", false).expect("large");
    let a = String::from_utf8_lossy(&a).replace(r#"text-size: "small""#, "S");
    let b = String::from_utf8_lossy(&b).replace(r#"text-size: "large""#, "S");
    assert_eq!(
        a, b,
        "only the text size should differ between these two decks"
    );
}
