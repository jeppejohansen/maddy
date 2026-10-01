//! Unit tests for Typst escaping.

use crate::typst::escape::{markup, markup_at_line_start, string, string_literal};

/// Compile `source` with Typst and return the text the PDF contains.
///
/// Escaping is the one place where a unit test alone is not convincing: the
/// claim is about what Typst does, so these tests check it against the real
/// compiler when it is available.
fn rendered_text(markup: &str) -> Option<String> {
    crate::typst::tests::render_text(markup)
}

// ------------------------------------------------------------------- markup

#[test]
fn ordinary_prose_is_left_alone() {
    assert_eq!(
        markup("Consider the linear model."),
        "Consider the linear model."
    );
    assert_eq!(markup(""), "");
}

#[test]
fn typst_markup_characters_are_escaped() {
    assert_eq!(markup(r"a\b"), r"a\\b");
    assert_eq!(markup("a#b"), r"a\#b");
    assert_eq!(markup("a$b"), r"a\$b");
    assert_eq!(markup("a*b"), r"a\*b");
    assert_eq!(markup("a_b"), r"a\_b");
    assert_eq!(markup("a`b"), r"a\`b");
    assert_eq!(markup("a[b]c"), r"a\[b\]c");
    assert_eq!(markup("a<b>c"), r"a\<b\>c");
    assert_eq!(markup("a@b"), r"a\@b");
    assert_eq!(markup("a~b"), r"a\~b");
}

#[test]
fn quotes_are_not_escaped_because_smart_quotes_are_wanted() {
    assert_eq!(
        markup("the \"estimator\"'s value"),
        "the \"estimator\"'s value"
    );
}

#[test]
fn a_lone_hyphen_or_dot_is_not_escaped() {
    assert_eq!(markup("well-known"), "well-known");
    assert_eq!(markup("e.g. 0.42"), "e.g. 0.42");
    assert_eq!(markup("a/b"), "a/b");
}

#[test]
fn hyphen_runs_are_escaped_so_they_do_not_become_dashes() {
    assert_eq!(markup("a--b"), r"a\-\-b");
    assert_eq!(markup("a---b"), r"a\-\-\-b");
}

#[test]
fn dot_runs_are_escaped_so_they_do_not_become_an_ellipsis() {
    assert_eq!(markup("wait..."), r"wait\.\.\.");
}

#[test]
fn a_slash_that_would_open_a_comment_is_escaped() {
    // Escaping the opening slash is enough to break the comment; the second
    // character no longer begins anything.
    assert_eq!(markup("a//b"), r"a\//b");
    assert_eq!(markup("a/*b"), r"a\/\*b");
}

// -------------------------------------------------------- line-start markup

#[test]
fn line_start_escaping_also_guards_block_constructs() {
    assert_eq!(markup_at_line_start("- not a list"), r"\- not a list");
    assert_eq!(markup_at_line_start("+ not a list"), r"\+ not a list");
    assert_eq!(markup_at_line_start("= not a heading"), r"\= not a heading");
    assert_eq!(markup_at_line_start("/ not a term"), r"\/ not a term");
}

#[test]
fn line_start_escaping_guards_enumerations() {
    assert_eq!(markup_at_line_start("1. not an enum"), r"1\. not an enum");
    assert_eq!(markup_at_line_start("42) not an enum"), r"42\) not an enum");
}

#[test]
fn line_start_escaping_only_applies_to_the_first_character() {
    assert_eq!(markup_at_line_start("a - b = c"), "a - b = c");
    assert_eq!(markup_at_line_start("see 1. above"), "see 1. above");
}

#[test]
fn line_start_escaping_still_escapes_the_ordinary_set() {
    assert_eq!(markup_at_line_start("*bold* text"), r"\*bold\* text");
}

#[test]
fn line_start_escaping_of_empty_text_is_empty() {
    assert_eq!(markup_at_line_start(""), "");
}

// -------------------------------------------------------------------- strings

#[test]
fn string_contents_escape_quotes_and_backslashes() {
    assert_eq!(string(r#"say "hi""#), r#"say \"hi\""#);
    assert_eq!(string(r"C:\path"), r"C:\\path");
}

#[test]
fn string_contents_escape_control_characters() {
    assert_eq!(string("a\nb"), r"a\nb");
    assert_eq!(string("a\tb"), r"a\tb");
    assert_eq!(string("a\rb"), r"a\rb");
}

#[test]
fn string_contents_leave_markup_characters_alone() {
    // Inside a string literal, markup is not interpreted.
    assert_eq!(string("a*b#c$d"), "a*b#c$d");
}

#[test]
fn string_literal_wraps_in_quotes() {
    assert_eq!(string_literal("rust"), r#""rust""#);
    assert_eq!(string_literal(r#"a"b"#), r#""a\"b""#);
}

// ----------------------------------------------- verified against real Typst

#[test]
fn escaped_markup_round_trips_through_typst() {
    let hostile = r##"\ # $ * _ ` [ ] < > @ ~ -- ... // /* 1. - + ="##;
    let Some(text) = rendered_text(&markup_at_line_start(hostile)) else {
        return; // Typst is not installed; the unit assertions above still hold.
    };

    // Typst's smart quotes and ligatures do not apply to any of these, so the
    // text must come back exactly as written.
    assert_eq!(text.trim(), hostile);
}

#[test]
fn escaped_markup_cannot_inject_typst_code() {
    let attack = r#"#panic("owned") and *bold* and $x$"#;
    let Some(text) = rendered_text(&markup(attack)) else {
        return;
    };
    assert_eq!(text.trim(), attack);
}
