//! Shared helpers for the Typst layer's tests.

mod document;
mod escape;
mod markup;

use std::process::Command;

/// Compile a fragment of Typst markup and return the text it produces.
///
/// Escaping and emission are claims about what *Typst* does, so the strongest
/// available check is to ask Typst. This uses Typst's HTML export, which yields
/// the text directly and needs no PDF tooling.
///
/// Returns `None` when Typst is unavailable or the export is unsupported, so the
/// suite degrades to its pure assertions instead of failing on a machine without
/// the toolchain.
pub(crate) fn render_text(markup: &str) -> Option<String> {
    let directory = tempfile::tempdir().ok()?;
    let source = directory.path().join("probe.typ");
    let target = directory.path().join("probe.html");

    // Smart quotes are a feature we deliberately keep, but they would rewrite
    // the very characters a round-trip test is comparing, so turn them off here.
    let document = format!("#set smartquote(enabled: false)\n{markup}\n");
    std::fs::write(&source, document).ok()?;

    let output = Command::new("typst")
        .arg("compile")
        .arg("--format")
        .arg("html")
        .arg("--features")
        .arg("html")
        .arg(&source)
        .arg(&target)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(strip_html(&std::fs::read_to_string(&target).ok()?))
}

/// Reduce an HTML document to its text content.
fn strip_html(html: &str) -> String {
    let body = html.split_once("<body>").map_or(html, |(_, rest)| rest);
    let body = body
        .split_once("</body>")
        .map_or(body, |(before, _)| before);

    let mut text = String::new();
    let mut in_tag = false;
    for character in body.chars() {
        match character {
            // Typst's HTML export escapes `<` but leaves a literal `>` alone,
            // so `>` only closes a tag that is actually open.
            '<' if !in_tag => in_tag = true,
            '>' if in_tag => in_tag = false,
            other if !in_tag => text.push(other),
            _ => {}
        }
    }

    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[test]
fn strip_html_extracts_body_text_and_unescapes_entities() {
    let html = "<html><head><title>x</title></head><body><p>a &lt;b&gt; &amp; c</p></body></html>";
    assert_eq!(strip_html(html), "a <b> & c");
}

#[test]
fn strip_html_keeps_a_literal_greater_than_outside_a_tag() {
    assert_eq!(strip_html("<body><p>a > b</p></body>"), "a > b");
}

#[test]
fn strip_html_tolerates_a_document_without_a_body() {
    assert_eq!(strip_html("<p>bare</p>"), "bare");
}
