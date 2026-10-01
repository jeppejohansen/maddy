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
/// Read from the page tree's `/Count`, which Typst writes outside its compressed
/// object streams. The largest value is the root of the tree, and so the total.
/// This is a deliberately small heuristic: the tests assert page counts, not PDF
/// structure, and pulling in a PDF parser to do it would not make them stronger.
pub fn pdf_page_count(pdf: &[u8]) -> Option<usize> {
    const KEY: &[u8] = b"/Count ";
    let mut best = None;

    for start in 0..pdf.len().saturating_sub(KEY.len()) {
        if &pdf[start..start + KEY.len()] != KEY {
            continue;
        }
        let digits: Vec<u8> = pdf[start + KEY.len()..]
            .iter()
            .copied()
            .take_while(u8::is_ascii_digit)
            .collect();
        if digits.is_empty() {
            continue;
        }
        let count: usize = String::from_utf8_lossy(&digits).parse().ok()?;
        best = Some(best.map_or(count, |previous: usize| previous.max(count)));
    }

    best
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
