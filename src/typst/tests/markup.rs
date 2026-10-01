//! Unit tests for IR-to-Typst markup rendering.

use crate::diagnostics::{CompileError, Diagnostics, Severity, SourceSpan};
use crate::ir::{
    Alignment, Block, CodeBlock, Image, Inline, ListItem, MathMode, MathSource, Table,
};
use crate::typst::markup::Renderer;

/// Render blocks, requiring success.
fn render(blocks: &[Block]) -> String {
    let mut diagnostics = Diagnostics::new();
    Renderer::new(&mut diagnostics)
        .blocks(blocks)
        .unwrap_or_else(|error| panic!("rendering failed: {error}"))
}

/// Render blocks, returning the markup and any diagnostics.
fn render_with_diagnostics(blocks: &[Block]) -> (String, Diagnostics) {
    let mut diagnostics = Diagnostics::new();
    let markup = Renderer::new(&mut diagnostics)
        .blocks(blocks)
        .expect("rendering failed");
    (markup, diagnostics)
}

fn text(value: &str) -> Inline {
    Inline::text(value)
}

fn paragraph(content: Vec<Inline>) -> Block {
    Block::Paragraph(content)
}

// ------------------------------------------------------------------ paragraphs

#[test]
fn a_paragraph_is_one_line_of_markup() {
    assert_eq!(
        render(&[paragraph(vec![text("Hello world.")])]),
        "Hello world."
    );
}

#[test]
fn blocks_are_separated_by_a_blank_line() {
    let blocks = [paragraph(vec![text("One.")]), paragraph(vec![text("Two.")])];
    assert_eq!(render(&blocks), "One.\n\nTwo.");
}

#[test]
fn nothing_renders_as_nothing() {
    assert_eq!(render(&[]), "");
}

// -------------------------------------------------------------------- escaping

#[test]
fn paragraph_text_is_escaped() {
    assert_eq!(
        render(&[paragraph(vec![text("costs #5 and *more*")])]),
        r"costs \#5 and \*more\*"
    );
}

#[test]
fn a_paragraph_beginning_with_a_marker_character_is_guarded() {
    // Without the guard, Typst would read this as a list item.
    assert_eq!(
        render(&[paragraph(vec![text("- not a list")])]),
        r"\- not a list"
    );
    assert_eq!(
        render(&[paragraph(vec![text("= not a heading")])]),
        r"\= not a heading"
    );
}

#[test]
fn only_the_first_run_of_a_paragraph_is_line_start_guarded() {
    let blocks = [paragraph(vec![text("see "), text("- here")])];
    assert_eq!(render(&blocks), "see - here");
}

// --------------------------------------------------------------------- inlines

#[test]
fn emphasis_and_strong_use_typst_markup() {
    let blocks = [paragraph(vec![
        text("This is "),
        Inline::Strong(vec![text("bold")]),
        text(" and "),
        Inline::Emphasis(vec![text("italic")]),
        text("."),
    ])];
    assert_eq!(render(&blocks), "This is *bold* and _italic_.");
}

#[test]
fn strikethrough_uses_the_function_form() {
    let blocks = [paragraph(vec![Inline::Strike(vec![text("gone")])])];
    assert_eq!(render(&blocks), "#strike[gone]");
}

#[test]
fn inline_code_uses_the_function_form_so_backticks_are_safe() {
    let blocks = [paragraph(vec![Inline::code("a ` b")])];
    assert_eq!(render(&blocks), r#"#raw("a ` b")"#);
}

#[test]
fn a_link_keeps_its_destination_as_a_string() {
    let blocks = [paragraph(vec![Inline::link(
        "https://example.com/a_b",
        vec![text("the paper")],
    )])];
    assert_eq!(
        render(&blocks),
        r#"#link("https://example.com/a_b")[the paper]"#
    );
}

#[test]
fn nested_emphasis_renders_inside_out() {
    let blocks = [paragraph(vec![Inline::Strong(vec![Inline::Emphasis(
        vec![text("deep")],
    )])])];
    assert_eq!(render(&blocks), "*_deep_*");
}

#[test]
fn a_soft_break_becomes_a_space_keeping_the_paragraph_on_one_line() {
    let blocks = [paragraph(vec![text("one"), Inline::SoftBreak, text("two")])];
    assert_eq!(render(&blocks), "one two");
}

#[test]
fn a_hard_break_uses_the_function_form() {
    let blocks = [paragraph(vec![text("one"), Inline::HardBreak, text("two")])];
    assert_eq!(render(&blocks), "one#linebreak()two");
}

#[test]
fn text_after_a_soft_break_is_still_line_start_guarded() {
    // The paragraph is emitted on one line, but a leading soft break means the
    // following run is still the first visible content.
    let blocks = [paragraph(vec![Inline::SoftBreak, text("- not a list")])];
    assert_eq!(render(&blocks), r" \- not a list");
}

// -------------------------------------------------------------------- headings

#[test]
fn headings_use_equals_markers() {
    let blocks = [
        Block::heading(1, vec![text("One")]),
        Block::heading(3, vec![text("Three")]),
    ];
    assert_eq!(render(&blocks), "= One\n\n=== Three");
}

#[test]
fn a_heading_level_beyond_six_is_clamped() {
    assert_eq!(
        render(&[Block::heading(9, vec![text("Deep")])]),
        "====== Deep"
    );
}

#[test]
fn heading_content_is_escaped() {
    assert_eq!(
        render(&[Block::heading(2, vec![text("A #tag")])]),
        r"== A \#tag"
    );
}

// ----------------------------------------------------------------- mathematics

#[test]
fn inline_mathematics_uses_tight_delimiters() {
    let blocks = [paragraph(vec![
        text("is "),
        Inline::Math(MathSource::inline(r"\beta_1")),
    ])];
    assert_eq!(render(&blocks), "is $β_1$");
}

#[test]
fn display_mathematics_uses_spaced_delimiters() {
    // The spaces are what make Typst set the equation as a block.
    let blocks = [Block::Math(MathSource::display(r"\frac{x}{y}"))];
    assert_eq!(render(&blocks), "$ frac(x, y) $");
}

#[test]
fn a_math_failure_becomes_an_error_pointing_at_the_markdown() {
    let math = MathSource::new(r"\foo{x}", MathMode::Inline, SourceSpan::new(17, 24));
    let mut diagnostics = Diagnostics::new();
    let error = Renderer::new(&mut diagnostics)
        .blocks(&[paragraph(vec![Inline::Math(math)])])
        .expect_err("expected a math error");

    assert!(matches!(error, CompileError::MathParse(_)), "{error:?}");
    let diagnostic = &error.diagnostics()[0];
    // The parser reported offset 0, so the span starts where the equation does.
    assert_eq!(diagnostic.span, Some(SourceSpan::new(17, 24)));
}

#[test]
fn a_math_parse_offset_is_shifted_onto_the_document() {
    let math = MathSource::new(r"x + \foo", MathMode::Display, SourceSpan::new(100, 108));
    let mut diagnostics = Diagnostics::new();
    let error = Renderer::new(&mut diagnostics)
        .blocks(&[Block::Math(math)])
        .expect_err("expected a math error");

    // The parser reported offset 4 within the equation.
    assert_eq!(error.diagnostics()[0].span, Some(SourceSpan::new(104, 108)));
}

#[test]
fn an_unsupported_construct_is_an_emit_error_that_names_what_was_parsed() {
    let math = MathSource::new(r"\color{red} x", MathMode::Display, SourceSpan::new(0, 13));
    let mut diagnostics = Diagnostics::new();
    let error = Renderer::new(&mut diagnostics)
        .blocks(&[Block::Math(math)])
        .expect_err("expected a math error");

    assert!(matches!(error, CompileError::MathEmit(_)), "{error:?}");
    assert!(error.diagnostics()[0].notes[0].starts_with("Parsed as:"));
}

// ----------------------------------------------------------------------- lists

#[test]
fn an_unordered_list_uses_hyphen_markers() {
    let blocks = [Block::UnorderedList(vec![
        ListItem::text(vec![text("First")]),
        ListItem::text(vec![text("Second")]),
    ])];
    assert_eq!(render(&blocks), "- First\n- Second");
}

#[test]
fn an_ordered_list_starting_at_one_uses_plus_markers() {
    let blocks = [Block::OrderedList {
        start: 1,
        items: vec![ListItem::text(vec![text("One")])],
    }];
    assert_eq!(render(&blocks), "+ One");
}

#[test]
fn an_ordered_list_with_another_start_carries_a_scoped_set_rule() {
    let blocks = [Block::OrderedList {
        start: 3,
        items: vec![
            ListItem::text(vec![text("Third")]),
            ListItem::text(vec![text("Fourth")]),
        ],
    }];
    assert_eq!(
        render(&blocks),
        "#[\n  #set enum(start: 3)\n  + Third\n  + Fourth\n]"
    );
}

#[test]
fn a_nested_list_is_indented_under_its_parent_item() {
    let blocks = [Block::UnorderedList(vec![ListItem::new(vec![
        paragraph(vec![text("Outer")]),
        Block::UnorderedList(vec![ListItem::text(vec![text("Inner")])]),
    ])])];
    assert_eq!(render(&blocks), "- Outer\n\n  - Inner");
}

#[test]
fn a_list_item_may_hold_display_mathematics() {
    let blocks = [Block::UnorderedList(vec![ListItem::new(vec![
        paragraph(vec![text("Consider")]),
        Block::Math(MathSource::display("x = 1")),
    ])])];
    assert_eq!(render(&blocks), "- Consider\n\n  $ x = 1 $");
}

// ----------------------------------------------------------------- code blocks

#[test]
fn a_code_block_uses_the_raw_function_with_its_language() {
    let blocks = [Block::CodeBlock(CodeBlock {
        language: Some("rust".into()),
        source: "fn main() {}\n".into(),
    })];
    assert_eq!(
        render(&blocks),
        r#"#raw(block: true, lang: "rust", "fn main() {}")"#
    );
}

#[test]
fn a_code_block_without_a_language_omits_the_argument() {
    let blocks = [Block::CodeBlock(CodeBlock {
        language: None,
        source: "plain\n".into(),
    })];
    assert_eq!(render(&blocks), r#"#raw(block: true, "plain")"#);
}

#[test]
fn an_awkward_info_string_cannot_corrupt_the_output() {
    // A fenced block would take only `rust` here and leak `,ignore` into the
    // source; the function form cannot.
    let blocks = [Block::CodeBlock(CodeBlock {
        language: Some("rust,ignore".into()),
        source: "x\n".into(),
    })];
    assert_eq!(
        render(&blocks),
        r#"#raw(block: true, lang: "rust,ignore", "x")"#
    );
}

#[test]
fn code_containing_backticks_and_quotes_is_escaped() {
    let blocks = [Block::CodeBlock(CodeBlock {
        language: None,
        source: "let s = \"```\";\n".into(),
    })];
    assert_eq!(render(&blocks), r#"#raw(block: true, "let s = \"```\";")"#);
}

// ----------------------------------------------------------------- blockquotes

#[test]
fn a_blockquote_wraps_its_blocks() {
    let blocks = [Block::BlockQuote(vec![paragraph(vec![text(
        "Models are abstractions.",
    )])])];
    assert_eq!(
        render(&blocks),
        "#quote(block: true)[\n  Models are abstractions.\n]"
    );
}

#[test]
fn a_nested_blockquote_indents() {
    let blocks = [Block::BlockQuote(vec![Block::BlockQuote(vec![paragraph(
        vec![text("deep")],
    )])])];
    assert_eq!(
        render(&blocks),
        "#quote(block: true)[\n  #quote(block: true)[\n    deep\n  ]\n]"
    );
}

// ---------------------------------------------------------------------- tables

#[test]
fn a_table_renders_columns_alignments_and_a_header() {
    let table = Table {
        alignments: vec![Alignment::None, Alignment::Right],
        header: vec![vec![text("Model")], vec![text("Estimate")]],
        rows: vec![vec![vec![text("OLS")], vec![text("0.42")]]],
    };
    assert_eq!(
        render(&[Block::Table(table)]),
        "#table(\n  columns: 2,\n  align: (auto, right),\n  \
         table.header([Model], [Estimate]),\n  [OLS], [0.42],\n)"
    );
}

#[test]
fn all_four_alignments_are_rendered() {
    let table = Table {
        alignments: vec![
            Alignment::None,
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
        ],
        header: vec![],
        rows: vec![vec![
            vec![text("a")],
            vec![text("b")],
            vec![text("c")],
            vec![text("d")],
        ]],
    };
    assert!(render(&[Block::Table(table)]).contains("align: (auto, left, center, right)"));
}

#[test]
fn a_ragged_row_is_padded_to_the_table_width() {
    let table = Table {
        alignments: vec![Alignment::None, Alignment::None],
        header: vec![vec![text("a")], vec![text("b")]],
        rows: vec![vec![vec![text("only")]]],
    };
    assert!(render(&[Block::Table(table)]).contains("[only], []"));
}

#[test]
fn table_cells_keep_inline_markup() {
    let table = Table {
        alignments: vec![Alignment::None],
        header: vec![],
        rows: vec![vec![vec![Inline::Math(MathSource::inline(r"\beta"))]]],
    };
    assert!(render(&[Block::Table(table)]).contains("[$β$]"));
}

// ---------------------------------------------------------------------- images

#[test]
fn a_block_image_becomes_an_image_call() {
    let blocks = [Block::Image(Image::new(
        "figures/results.png",
        vec![text("Results")],
    ))];
    assert_eq!(
        render(&blocks),
        r#"image("figures/results.png", alt: "Results")"#
    );
}

#[test]
fn an_image_without_alt_text_omits_the_argument() {
    let blocks = [Block::Image(Image::new("r.png", vec![]))];
    assert_eq!(render(&blocks), r#"image("r.png")"#);
}

#[test]
fn an_inline_image_is_boxed_so_it_sits_in_the_line() {
    let blocks = [paragraph(vec![Inline::Image(Image::new("r.png", vec![]))])];
    assert_eq!(render(&blocks), r#"#box(image("r.png"))"#);
}

#[test]
fn a_remote_image_warns_and_renders_nothing() {
    let image = Image {
        destination: "https://example.com/plot.png".into(),
        alt: vec![],
        title: None,
        span: SourceSpan::new(10, 40),
    };
    let (markup, diagnostics) = render_with_diagnostics(&[Block::Image(image)]);

    assert_eq!(markup, "#box[]");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics.entries()[0].severity, Severity::Warning);
    assert!(diagnostics.entries()[0].message.contains("remote images"));
    assert_eq!(diagnostics.entries()[0].span, Some(SourceSpan::new(10, 40)));
}

// ----------------------------------------------------------------------- rules

#[test]
fn a_thematic_break_becomes_a_rule() {
    assert_eq!(render(&[Block::Rule]), "#line(length: 100%)");
}
