//! The mathematics compatibility corpus.
//!
//! Every expression in `tests/math/` must clear four hurdles, per the
//! specification:
//!
//! 1. parse
//! 2. translate
//! 3. produce valid Typst
//! 4. compile successfully
//!
//! Steps 1 and 2 are checked per expression so a failure names the file, the
//! line and the expression. Steps 3 and 4 are checked by compiling every
//! expression in one document, which is both faster and a stronger test: a
//! construct that only breaks in the presence of others still fails here.

mod support;

use maddy::ir::MathMode;
use maddy::math::to_typst;

/// Translate every expression in every corpus file.
fn translate_all() -> Vec<(String, usize, String, String)> {
    let mut translated = Vec::new();
    let mut failures = Vec::new();

    for file in support::corpus_files() {
        for (line, expression) in support::corpus(&file) {
            match to_typst(&expression, MathMode::Display) {
                Ok(typst) => translated.push((file.clone(), line, expression, typst)),
                Err(error) => failures.push(format!("{file}:{line}: {expression}\n    {error}")),
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} corpus expression(s) failed to translate:\n{}",
        failures.len(),
        failures.join("\n")
    );

    translated
}

#[test]
fn the_corpus_is_not_empty() {
    let files = support::corpus_files();
    assert!(
        files.len() >= 14,
        "expected the full corpus, found {files:?}"
    );

    let total: usize = files.iter().map(|file| support::corpus(file).len()).sum();
    assert!(
        total >= 200,
        "expected a substantial corpus, found {total} expressions"
    );
}

#[test]
fn every_corpus_expression_parses_and_translates() {
    let translated = translate_all();
    assert!(!translated.is_empty());

    // A translation that silently produced nothing would pass the compile step
    // while rendering no mathematics at all.
    for (file, line, expression, typst) in &translated {
        assert!(
            !typst.trim().is_empty(),
            "{file}:{line}: {expression} translated to nothing"
        );
    }
}

#[test]
fn every_corpus_expression_compiles_under_typst() {
    if !support::typst_available() {
        support::skip("every_corpus_expression_compiles_under_typst");
        return;
    }

    let translated = translate_all();
    let directory = tempfile::tempdir().expect("creating a temporary directory");
    let source = directory.path().join("corpus.typ");

    // Each expression is labelled so a Typst error can be traced to its line.
    let mut document = String::from("#set page(width: 300mm, height: auto, margin: 10mm)\n");
    for (file, line, _, typst) in &translated {
        document.push_str(&format!("// {file}:{line}\n$ {typst} $\n\n"));
    }
    std::fs::write(&source, &document).expect("writing the corpus document");

    if let Err(stderr) = support::compile(&source, &directory.path().join("corpus.pdf")) {
        // Map Typst's reported line numbers back to the corpus.
        let lines: Vec<&str> = document.lines().collect();
        let mut context = String::new();
        for capture in stderr.lines() {
            if let Some(number) = capture
                .split("corpus.typ:")
                .nth(1)
                .and_then(|rest| rest.split(':').next())
                .and_then(|number| number.parse::<usize>().ok())
            {
                let origin = lines[..number.min(lines.len())]
                    .iter()
                    .rev()
                    .find(|line| line.starts_with("// "))
                    .unwrap_or(&"unknown origin");
                context.push_str(&format!("  {origin}\n"));
            }
        }
        panic!("the corpus produced invalid Typst:\n{stderr}\norigins:\n{context}");
    }
}
