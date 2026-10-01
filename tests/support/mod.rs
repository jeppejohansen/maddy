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
