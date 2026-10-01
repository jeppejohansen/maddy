//! Shared helpers for integration tests.

#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

/// Whether the Typst executable is available.
///
/// Tests that need the real compiler skip themselves when it is absent, with a
/// printed note, rather than failing on a machine without the toolchain.
pub fn typst_available() -> bool {
    Command::new("typst")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Announce that a test is being skipped, and why.
pub fn skip(test: &str) {
    eprintln!("skipping {test}: the `typst` executable was not found on PATH");
}

/// Compile a Typst file, returning the compiler's stderr on failure.
pub fn compile(source: &Path, output: &Path) -> Result<(), String> {
    let result = Command::new("typst")
        .arg("compile")
        .arg("--root")
        .arg(source.parent().unwrap_or(Path::new(".")))
        .arg(source)
        .arg(output)
        .output()
        .map_err(|error| error.to_string())?;

    if result.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&result.stderr).into_owned())
    }
}

/// Read every expression from a corpus file, skipping blanks and comments.
pub fn corpus(name: &str) -> Vec<(usize, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/math")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));

    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim().to_string()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// The names of every corpus file, so a new one is picked up automatically.
pub fn corpus_files() -> Vec<String> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/math");
    let mut names: Vec<String> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".txt"))
        .collect();
    names.sort();
    names
}

/// The number of pages in a PDF.
///
/// Counts `/Type /Page` objects, excluding the `/Type /Pages` tree node. The
/// obvious alternative — reading the page tree's `/Count` — is wrong: Typst also
/// writes a `/Count` for the document outline, which counts headings, so a
/// one-page paper with two headings reports two pages.
///
/// A deliberately small heuristic: these tests assert page counts, not PDF
/// structure, and a full PDF parser would not make them stronger.
pub fn pdf_page_count(pdf: &[u8]) -> Option<usize> {
    const KEY: &[u8] = b"/Type";
    let mut pages = 0;

    for start in 0..pdf.len().saturating_sub(KEY.len()) {
        if &pdf[start..start + KEY.len()] != KEY {
            continue;
        }

        let rest = &pdf[start + KEY.len()..];
        let value = rest.iter().position(|byte| !byte.is_ascii_whitespace())?;
        let rest = &rest[value..];

        if !rest.starts_with(b"/Page") {
            continue;
        }
        // `/Pages` is the tree node, not a page.
        if rest
            .get(b"/Page".len())
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        {
            continue;
        }
        pages += 1;
    }

    (pages > 0).then_some(pages)
}

/// Whether a byte slice looks like a PDF.
pub fn is_pdf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}

/// Write a Markdown file into a directory and return its path.
pub fn write_markdown(directory: &Path, name: &str, contents: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, contents).unwrap_or_else(|error| panic!("writing {name}: {error}"));
    path
}

/// The width-to-height ratio of a PDF's first page.
///
/// Read from `/MediaBox`, which Typst writes in plain text.
pub fn pdf_aspect_ratio(pdf: &[u8]) -> Option<f64> {
    const KEY: &[u8] = b"/MediaBox";
    let found = pdf.windows(KEY.len()).position(|window| window == KEY)? + KEY.len();
    // Typst writes `/MediaBox[...]`, but the space is optional in the format.
    let bracket = found + pdf[found..].iter().position(|byte| *byte == b'[')?;
    let start = bracket + 1;
    let end = start + pdf[start..].iter().position(|byte| *byte == b']')?;

    let numbers: Vec<f64> = String::from_utf8_lossy(&pdf[start..end])
        .split_whitespace()
        .filter_map(|value| value.parse().ok())
        .collect();

    let [left, bottom, right, top] = numbers[..] else {
        return None;
    };
    let height = top - bottom;
    (height > 0.0).then(|| (right - left) / height)
}
