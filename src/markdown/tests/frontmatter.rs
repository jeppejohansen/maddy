//! Unit tests for the front-matter pre-pass.

use crate::diagnostics::{CompileError, Severity};
use crate::markdown::frontmatter::split;
use crate::metadata::DocumentType;

#[test]
fn a_document_without_front_matter_is_returned_unchanged() {
    let source = "# Heading\n\nSome text.\n";
    let front = split(source).unwrap();

    assert_eq!(front.metadata, Default::default());
    assert_eq!(front.body, source);
    assert_eq!(front.body_offset, 0);
    assert!(front.diagnostics.is_empty());
}

#[test]
fn front_matter_is_parsed_and_removed_from_the_body() {
    let source = "---\ntitle: Peer Effects\n---\n\n# Motivation\n";
    let front = split(source).unwrap();

    assert_eq!(front.metadata.title.as_deref(), Some("Peer Effects"));
    assert_eq!(front.body, "\n# Motivation\n");
    assert!(front.diagnostics.is_empty());
}

#[test]
fn the_body_offset_locates_the_body_in_the_original_source() {
    let source = "---\ntitle: T\n---\n# Heading\n";
    let front = split(source).unwrap();

    // Spans produced from the body must be shiftable back onto the file.
    assert_eq!(front.body_offset, source.find("# Heading").unwrap());
    assert_eq!(&source[front.body_offset..], front.body);
}

#[test]
fn all_six_fields_are_recognized() {
    let source = "---\n\
                  title: Peer Effects in High School Application\n\
                  subtitle: Strategic Complementarities\n\
                  author: Jane Researcher\n\
                  date: October 2026\n\
                  type: slides\n\
                  style: academic\n\
                  ---\nbody\n";
    let metadata = split(source).unwrap().metadata;

    assert_eq!(
        metadata.title.as_deref(),
        Some("Peer Effects in High School Application")
    );
    assert_eq!(
        metadata.subtitle.as_deref(),
        Some("Strategic Complementarities")
    );
    assert_eq!(metadata.author.as_deref(), Some("Jane Researcher"));
    assert_eq!(metadata.date.as_deref(), Some("October 2026"));
    assert_eq!(metadata.document_type, DocumentType::Slides);
    assert_eq!(metadata.style.as_deref(), Some("academic"));
}

#[test]
fn front_matter_is_only_recognized_at_the_very_beginning_of_the_file() {
    let source = "Some text.\n\n---\ntitle: Not Front Matter\n---\n";
    let front = split(source).unwrap();

    assert_eq!(front.metadata.title, None);
    assert_eq!(front.body, source);
    assert_eq!(front.body_offset, 0);
}

#[test]
fn a_leading_thematic_break_is_not_treated_as_front_matter() {
    // No closing delimiter, so this is ordinary Markdown.
    let source = "---\n\n# Heading\n\nText.\n";
    let front = split(source).unwrap();

    assert_eq!(front.body, source);
    assert!(front.metadata.is_empty());
}

#[test]
fn slide_separators_in_the_body_survive_the_pre_pass() {
    let source = "---\ntype: slides\n---\n\n# One\n\n---\n\n# Two\n";
    let front = split(source).unwrap();

    assert_eq!(front.metadata.document_type, DocumentType::Slides);
    assert_eq!(front.body, "\n# One\n\n---\n\n# Two\n");
    assert_eq!(front.body.matches("---").count(), 1);
}

#[test]
fn empty_front_matter_yields_default_metadata() {
    let front = split("---\n---\nbody\n").unwrap();

    assert!(front.metadata.is_empty());
    assert_eq!(front.body, "body\n");
    assert!(front.diagnostics.is_empty());
}

#[test]
fn front_matter_may_be_the_entire_file() {
    let front = split("---\ntitle: T\n---").unwrap();

    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.body, "");
}

#[test]
fn trailing_whitespace_on_the_delimiters_is_tolerated() {
    let front = split("---  \ntitle: T\n---\t\nbody\n").unwrap();
    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.body, "body\n");
}

#[test]
fn carriage_returns_are_tolerated() {
    let front = split("---\r\ntitle: T\r\n---\r\nbody\r\n").unwrap();
    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.body, "body\r\n");
}

#[test]
fn a_byte_order_mark_does_not_hide_front_matter() {
    let front = split("\u{feff}---\ntitle: T\n---\nbody\n").unwrap();
    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.body, "body\n");
}

#[test]
fn unknown_fields_warn_rather_than_fail() {
    let source = "---\ntitle: T\nkeywords: economics\nfontsize: 11pt\n---\nbody\n";
    let front = split(source).unwrap();

    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.diagnostics.len(), 2);
    assert!(front
        .diagnostics
        .iter()
        .all(|d| d.severity == Severity::Warning));
    assert!(front.diagnostics[0].message.contains("keywords"));
    assert!(front.diagnostics[1].message.contains("fontsize"));
}

#[test]
fn an_unknown_field_warning_points_at_the_offending_key() {
    let source = "---\ntitle: T\nkeywords: economics\n---\nbody\n";
    let front = split(source).unwrap();

    let span = front.diagnostics[0]
        .span
        .expect("the warning should carry a span");
    assert_eq!(&source[span.start..span.end], "keywords");
}

#[test]
fn field_names_are_matched_case_insensitively() {
    let front = split("---\nTitle: T\nAUTHOR: A\n---\nbody\n").unwrap();

    assert_eq!(front.metadata.title.as_deref(), Some("T"));
    assert_eq!(front.metadata.author.as_deref(), Some("A"));
    assert!(front.diagnostics.is_empty());
}

#[test]
fn non_string_scalars_are_coerced_to_text() {
    let front = split("---\ntitle: 42\ndate: 2026-10-01\n---\nbody\n").unwrap();

    assert_eq!(front.metadata.title.as_deref(), Some("42"));
    assert_eq!(front.metadata.date.as_deref(), Some("2026-10-01"));
}

#[test]
fn an_explicitly_null_field_is_treated_as_absent() {
    let front = split("---\ntitle:\nauthor: A\n---\nbody\n").unwrap();

    assert_eq!(front.metadata.title, None);
    assert_eq!(front.metadata.author.as_deref(), Some("A"));
}

#[test]
fn an_invalid_document_type_is_an_error() {
    let error = split("---\ntype: poster\n---\nbody\n").unwrap_err();

    assert!(matches!(error, CompileError::FrontMatter { .. }));
    assert_eq!(error.exit_code(), 1);
    assert!(error.to_string().contains("poster"));
}

#[test]
fn a_non_scalar_field_value_is_an_error_naming_the_field() {
    let error = split("---\nauthor:\n  - Jane\n  - John\n---\nbody\n").unwrap_err();

    assert!(matches!(error, CompileError::FrontMatter { .. }));
    assert!(error.to_string().contains("author"), "{error}");
}

#[test]
fn malformed_yaml_is_an_error_with_a_span_inside_the_front_matter() {
    let source = "---\ntitle: [unclosed\n---\nbody\n";
    let error = split(source).unwrap_err();

    let CompileError::FrontMatter { span, .. } = &error else {
        panic!("expected a front-matter error, got {error:?}");
    };
    let span = span.expect("a YAML error should carry a span");
    assert!(span.start >= 4 && span.end <= source.len(), "{span:?}");
}

#[test]
fn front_matter_that_is_not_a_mapping_is_an_error() {
    let error = split("---\n- just\n- a list\n---\nbody\n").unwrap_err();
    assert!(matches!(error, CompileError::FrontMatter { .. }));
}

#[test]
fn a_comment_only_front_matter_block_is_empty_rather_than_an_error() {
    let front = split("---\n# just a comment\n---\nbody\n").unwrap();
    assert!(front.metadata.is_empty());
    assert_eq!(front.body, "body\n");
}
