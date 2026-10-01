//! Unit tests for Markdown-to-IR parsing.

use crate::diagnostics::{Severity, SourceSpan};
use crate::ir::{Alignment, Block, Inline, ListItem, MathMode};
use crate::markdown::parser::parse;
use crate::metadata::DocumentType;

fn blocks(source: &str) -> Vec<Block> {
    parse(source, 0, DocumentType::Document).blocks
}

fn slide_blocks(source: &str) -> Vec<Block> {
    parse(source, 0, DocumentType::Slides).blocks
}

fn text(value: &str) -> Inline {
    Inline::text(value)
}

fn only_paragraph(source: &str) -> Vec<Inline> {
    match blocks(source).into_iter().next() {
        Some(Block::Paragraph(content)) => content,
        other => panic!("expected a single paragraph, got {other:?}"),
    }
}

// ---------------------------------------------------------------- paragraphs

#[test]
fn a_paragraph_of_plain_text() {
    assert_eq!(
        blocks("Hello world.\n"),
        vec![Block::paragraph(vec![text("Hello world.")])]
    );
}

#[test]
fn blank_input_produces_no_blocks() {
    assert!(blocks("").is_empty());
    assert!(blocks("\n\n   \n").is_empty());
}

#[test]
fn successive_paragraphs_are_separate_blocks() {
    let parsed = blocks("One.\n\nTwo.\n");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0], Block::paragraph(vec![text("One.")]));
    assert_eq!(parsed[1], Block::paragraph(vec![text("Two.")]));
}

// ------------------------------------------------------------------ emphasis

#[test]
fn strong_emphasis_nests_inside_a_paragraph() {
    // The IR shape the specification gives as an example.
    assert_eq!(
        only_paragraph("This is **bold**.\n"),
        vec![
            text("This is "),
            Inline::Strong(vec![text("bold")]),
            text(".")
        ]
    );
}

#[test]
fn emphasis_strike_and_code_are_distinguished() {
    assert_eq!(
        only_paragraph("*italic*\n"),
        vec![Inline::Emphasis(vec![text("italic")])]
    );
    assert_eq!(
        only_paragraph("~~gone~~\n"),
        vec![Inline::Strike(vec![text("gone")])]
    );
    assert_eq!(only_paragraph("`code`\n"), vec![Inline::code("code")]);
}

#[test]
fn emphasis_nests_arbitrarily() {
    assert_eq!(
        only_paragraph("***both***\n"),
        vec![Inline::Emphasis(vec![Inline::Strong(vec![text("both")])])]
    );
}

#[test]
fn soft_and_hard_breaks_are_preserved() {
    assert_eq!(
        only_paragraph("one\ntwo\n"),
        vec![text("one"), Inline::SoftBreak, text("two")]
    );
    assert_eq!(
        only_paragraph("one  \ntwo\n"),
        vec![text("one"), Inline::HardBreak, text("two")]
    );
}

// ------------------------------------------------------------------ headings

#[test]
fn headings_record_their_level_and_content() {
    let parsed = blocks("# One\n\n### Three\n");
    assert!(matches!(&parsed[0], Block::Heading { level: 1, .. }));
    assert!(matches!(&parsed[1], Block::Heading { level: 3, .. }));

    let Block::Heading { content, .. } = &parsed[0] else {
        unreachable!()
    };
    assert_eq!(content, &vec![text("One")]);
}

#[test]
fn a_heading_span_points_at_its_source() {
    let source = "intro\n\n## Notes\n";
    let parsed = blocks(source);
    let Block::Heading { span, .. } = &parsed[1] else {
        panic!("expected a heading")
    };
    assert!(source[span.start..span.end].starts_with("## Notes"));
}

#[test]
fn heading_spans_are_shifted_by_the_body_offset() {
    // The body began 20 bytes into the file, after front matter.
    let parsed = parse("# Title\n", 20, DocumentType::Document).blocks;
    let Block::Heading { span, .. } = &parsed[0] else {
        panic!("expected a heading")
    };
    assert_eq!(span.start, 20);
}

#[test]
fn headings_may_contain_inline_markup_and_math() {
    let parsed = blocks("# The $\\beta$ *estimator*\n");
    let Block::Heading { content, .. } = &parsed[0] else {
        panic!("expected a heading")
    };
    assert_eq!(content.len(), 4);
    assert!(matches!(content[1], Inline::Math(_)));
    assert!(matches!(content[3], Inline::Emphasis(_)));
}

// ---------------------------------------------------------------------- math

#[test]
fn inline_math_captures_its_source_without_the_delimiters() {
    let content = only_paragraph("The coefficient is $\\beta_1$.\n");
    let Inline::Math(math) = &content[1] else {
        panic!("expected inline math, got {content:?}")
    };

    assert_eq!(math.source, r"\beta_1");
    assert_eq!(math.mode, MathMode::Inline);
}

#[test]
fn an_inline_math_span_covers_only_the_mathematics() {
    let source = "We have $\\foo{x}$ here.\n";
    let content = only_paragraph(source);
    let Inline::Math(math) = &content[1] else {
        panic!("expected inline math")
    };

    assert_eq!(&source[math.span.start..math.span.end], r"\foo{x}");
}

#[test]
fn display_math_becomes_a_block_of_its_own() {
    let parsed = blocks("$$\n\\frac{x}{y}\n$$\n");
    let [Block::Math(math)] = parsed.as_slice() else {
        panic!("expected one math block, got {parsed:?}")
    };

    assert_eq!(math.source, r"\frac{x}{y}");
    assert_eq!(math.mode, MathMode::Display);
}

#[test]
fn a_display_math_span_covers_the_trimmed_mathematics() {
    let source = "$$\n\\frac{x}{y}\n$$\n";
    let parsed = blocks(source);
    let [Block::Math(math)] = parsed.as_slice() else {
        panic!("expected one math block")
    };

    assert_eq!(&source[math.span.start..math.span.end], r"\frac{x}{y}");
}

#[test]
fn display_math_surrounded_by_prose_stays_inline_in_its_paragraph() {
    // Only a paragraph consisting solely of display math is hoisted to a block.
    let content = only_paragraph("Thus $$x = 1$$ holds.\n");
    assert_eq!(content.len(), 3);
    let Inline::Math(math) = &content[1] else {
        panic!("expected math, got {content:?}")
    };
    assert_eq!(math.mode, MathMode::Display);
}

#[test]
fn multiline_display_math_keeps_its_internal_newlines() {
    let parsed = blocks("$$\n\\hat\\beta =\n(X^\\top X)^{-1}\n$$\n");
    let [Block::Math(math)] = parsed.as_slice() else {
        panic!("expected one math block")
    };
    assert_eq!(math.source, "\\hat\\beta =\n(X^\\top X)^{-1}");
}

#[test]
fn a_dollar_sign_in_prose_is_not_mathematics() {
    let content = only_paragraph("It costs 5 dollars.\n");
    assert_eq!(content, vec![text("It costs 5 dollars.")]);
}

// --------------------------------------------------------------------- lists

#[test]
fn an_unordered_list_of_simple_items() {
    assert_eq!(
        blocks("- First\n- Second\n"),
        vec![Block::UnorderedList(vec![
            ListItem::text(vec![text("First")]),
            ListItem::text(vec![text("Second")]),
        ])]
    );
}

#[test]
fn an_ordered_list_records_its_start() {
    let parsed = blocks("3. Third\n4. Fourth\n");
    let [Block::OrderedList { start, items }] = parsed.as_slice() else {
        panic!("expected an ordered list, got {parsed:?}")
    };
    assert_eq!(*start, 3);
    assert_eq!(items.len(), 2);
}

#[test]
fn an_ordered_list_defaults_to_starting_at_one() {
    let parsed = blocks("1. One\n");
    let [Block::OrderedList { start, .. }] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert_eq!(*start, 1);
}

#[test]
fn nested_lists_become_nested_blocks() {
    let parsed = blocks("- Outer\n  - Inner\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].blocks.len(), 2);
    assert!(matches!(items[0].blocks[0], Block::Paragraph(_)));
    assert!(matches!(items[0].blocks[1], Block::UnorderedList(_)));
}

#[test]
fn a_list_item_may_contain_display_mathematics() {
    let parsed = blocks("- Consider\n\n  $$\n  x = 1\n  $$\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert!(
        items[0].blocks.iter().any(|b| matches!(b, Block::Math(_))),
        "{:?}",
        items[0].blocks
    );
}

#[test]
fn list_items_keep_inline_markup() {
    let parsed = blocks("- $X_i \\in \\mathbb R^K$.\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    let [Block::Paragraph(content)] = items[0].blocks.as_slice() else {
        panic!("expected a paragraph")
    };
    assert!(matches!(content[0], Inline::Math(_)));
    assert_eq!(content[1], text("."));
}

// ---------------------------------------------------------------- code blocks

#[test]
fn a_fenced_code_block_keeps_its_language_and_source() {
    let parsed = blocks("```rust\nfn main() {}\n```\n");
    let [Block::CodeBlock(code)] = parsed.as_slice() else {
        panic!("expected a code block, got {parsed:?}")
    };
    assert_eq!(code.language.as_deref(), Some("rust"));
    assert_eq!(code.source, "fn main() {}\n");
}

#[test]
fn a_fence_without_a_language_has_none() {
    let parsed = blocks("```\nplain\n```\n");
    let [Block::CodeBlock(code)] = parsed.as_slice() else {
        panic!("expected a code block")
    };
    assert_eq!(code.language, None);
}

#[test]
fn only_the_first_word_of_an_info_string_is_the_language() {
    let parsed = blocks("```rust,ignore extra\nx\n```\n");
    let [Block::CodeBlock(code)] = parsed.as_slice() else {
        panic!("expected a code block")
    };
    assert_eq!(code.language.as_deref(), Some("rust,ignore"));
}

#[test]
fn an_indented_code_block_is_recognized() {
    let parsed = blocks("    indented\n");
    let [Block::CodeBlock(code)] = parsed.as_slice() else {
        panic!("expected a code block, got {parsed:?}")
    };
    assert_eq!(code.language, None);
    assert_eq!(code.source, "indented\n");
}

#[test]
fn mathematics_inside_a_code_block_is_not_parsed_as_mathematics() {
    let parsed = blocks("```\n$x_i^2$\n```\n");
    let [Block::CodeBlock(code)] = parsed.as_slice() else {
        panic!("expected a code block")
    };
    assert_eq!(code.source, "$x_i^2$\n");
}

// --------------------------------------------------------------- blockquotes

#[test]
fn a_blockquote_holds_blocks() {
    let parsed = blocks("> Economic models are abstractions.\n");
    let [Block::BlockQuote(inner)] = parsed.as_slice() else {
        panic!("expected a blockquote, got {parsed:?}")
    };
    assert_eq!(
        inner,
        &vec![Block::paragraph(vec![text(
            "Economic models are abstractions."
        )])]
    );
}

#[test]
fn a_blockquote_may_nest() {
    let parsed = blocks("> outer\n>\n> > inner\n");
    let [Block::BlockQuote(inner)] = parsed.as_slice() else {
        panic!("expected a blockquote")
    };
    assert!(
        inner.iter().any(|b| matches!(b, Block::BlockQuote(_))),
        "{inner:?}"
    );
}

// --------------------------------------------------------------------- links

#[test]
fn a_link_keeps_its_destination_and_content() {
    let content = only_paragraph("See [the paper](https://example.com/p.pdf).\n");
    let Inline::Link {
        destination,
        content: label,
    } = &content[1]
    else {
        panic!("expected a link, got {content:?}")
    };
    assert_eq!(destination, "https://example.com/p.pdf");
    assert_eq!(label, &vec![text("the paper")]);
}

#[test]
fn a_reference_link_resolves_to_its_destination() {
    let content = only_paragraph("See [the paper][p].\n\n[p]: https://example.com\n");
    let Inline::Link { destination, .. } = &content[1] else {
        panic!("expected a link")
    };
    assert_eq!(destination, "https://example.com");
}

// -------------------------------------------------------------------- images

#[test]
fn an_image_alone_in_a_paragraph_becomes_a_block() {
    let parsed = blocks("![Results](figures/results.png)\n");
    let [Block::Image(image)] = parsed.as_slice() else {
        panic!("expected an image block, got {parsed:?}")
    };
    assert_eq!(image.destination, "figures/results.png");
    assert_eq!(image.alt, vec![text("Results")]);
}

#[test]
fn an_image_among_prose_stays_inline() {
    let content = only_paragraph("Look ![Results](r.png) here.\n");
    assert_eq!(content.len(), 3);
    assert!(matches!(content[1], Inline::Image(_)));
}

#[test]
fn an_image_title_is_captured() {
    let parsed = blocks("![Alt](r.png \"A caption\")\n");
    let [Block::Image(image)] = parsed.as_slice() else {
        panic!("expected an image block")
    };
    assert_eq!(image.title.as_deref(), Some("A caption"));
}

#[test]
fn an_image_span_points_at_its_source() {
    let source = "![Alt](r.png)\n";
    let parsed = blocks(source);
    let [Block::Image(image)] = parsed.as_slice() else {
        panic!("expected an image block")
    };
    assert_eq!(&source[image.span.start..image.span.end], "![Alt](r.png)");
}

// -------------------------------------------------------------------- tables

#[test]
fn a_table_captures_its_header_rows_and_alignments() {
    let parsed = blocks("| Model | Estimate |\n|---|---:|\n| OLS | 0.42 |\n| IV | 0.51 |\n");
    let [Block::Table(table)] = parsed.as_slice() else {
        panic!("expected a table, got {parsed:?}")
    };

    assert_eq!(table.alignments, vec![Alignment::None, Alignment::Right]);
    assert_eq!(
        table.header,
        vec![vec![text("Model")], vec![text("Estimate")]]
    );
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0], vec![vec![text("OLS")], vec![text("0.42")]]);
    assert_eq!(table.columns(), 2);
}

#[test]
fn all_four_column_alignments_are_recognized() {
    let parsed = blocks("| a | b | c | d |\n|---|:--|:-:|--:|\n| 1 | 2 | 3 | 4 |\n");
    let [Block::Table(table)] = parsed.as_slice() else {
        panic!("expected a table")
    };
    assert_eq!(
        table.alignments,
        vec![
            Alignment::None,
            Alignment::Left,
            Alignment::Center,
            Alignment::Right
        ]
    );
}

#[test]
fn table_cells_keep_inline_markup_and_math() {
    let parsed = blocks("| Term | Value |\n|---|---|\n| $\\beta$ | **0.42** |\n");
    let [Block::Table(table)] = parsed.as_slice() else {
        panic!("expected a table")
    };
    assert!(matches!(table.rows[0][0][0], Inline::Math(_)));
    assert!(matches!(table.rows[0][1][0], Inline::Strong(_)));
}

// ---------------------------------------------------- rules and slide breaks

#[test]
fn a_thematic_break_is_a_rule_in_document_mode() {
    let parsed = blocks("One.\n\n---\n\nTwo.\n");
    assert_eq!(parsed[1], Block::Rule);
}

#[test]
fn a_thematic_break_is_a_slide_break_in_slides_mode() {
    let parsed = slide_blocks("One.\n\n---\n\nTwo.\n");
    assert_eq!(parsed[1], Block::SlideBreak);
}

#[test]
fn slides_mode_changes_nothing_except_the_separator() {
    let source = "# Model\n\n$$\nx = 1\n$$\n\n- a\n";
    assert_eq!(blocks(source), slide_blocks(source));
}

#[test]
fn other_thematic_break_spellings_are_also_separators() {
    for separator in ["---", "***", "___", "- - -"] {
        let source = format!("One.\n\n{separator}\n\nTwo.\n");
        assert_eq!(slide_blocks(&source)[1], Block::SlideBreak, "{separator}");
    }
}

// ------------------------------------------------------------------ raw HTML

#[test]
fn a_raw_html_block_warns_and_is_dropped() {
    let outcome = parse("<div>hello</div>\n", 0, DocumentType::Document);

    assert!(outcome.blocks.is_empty(), "{:?}", outcome.blocks);
    assert_eq!(outcome.diagnostics.len(), 1);
    assert_eq!(outcome.diagnostics[0].severity, Severity::Warning);
    assert!(outcome.diagnostics[0].message.contains("raw HTML"));
}

#[test]
fn a_raw_html_warning_points_at_the_markup() {
    let source = "text\n\n<div>hello</div>\n";
    let outcome = parse(source, 0, DocumentType::Document);
    let span = outcome.diagnostics[0].span.expect("expected a span");
    assert!(
        source[span.start..span.end].starts_with("<div>"),
        "{:?}",
        &source[span.start..]
    );
}

#[test]
fn inline_html_warns_once_and_is_dropped() {
    let outcome = parse("Some <b>bold</b> text.\n", 0, DocumentType::Document);

    assert_eq!(outcome.diagnostics.len(), 2, "{:?}", outcome.diagnostics);
    assert!(outcome
        .diagnostics
        .iter()
        .all(|d| d.message.contains("raw HTML")));
    // The surrounding prose survives; only the tags are dropped.
    let [Block::Paragraph(content)] = outcome.blocks.as_slice() else {
        panic!("expected a paragraph, got {:?}", outcome.blocks)
    };
    assert_eq!(crate::ir::plain_text(content), "Some bold text.");
}

#[test]
fn html_diagnostic_spans_are_shifted_by_the_body_offset() {
    let outcome = parse("<div>x</div>\n", 40, DocumentType::Document);
    assert_eq!(
        outcome.diagnostics[0].span.unwrap(),
        SourceSpan::new(40, 53)
    );
}

// --------------------------------------------------------------- integration

#[test]
fn the_first_stopping_point_document_parses_completely() {
    let source = "# Heading\n\n\
                  Some **bold** and *italic* text.\n\n\
                  - First\n- Second\n\n\
                  Inline mathematics: $x_i^2$.\n\n\
                  $$\n\\frac{x}{y}\n$$\n";
    let outcome = parse(source, 0, DocumentType::Document);

    assert!(outcome.diagnostics.is_empty(), "{:?}", outcome.diagnostics);
    assert!(matches!(outcome.blocks[0], Block::Heading { level: 1, .. }));
    assert!(matches!(outcome.blocks[1], Block::Paragraph(_)));
    assert!(matches!(outcome.blocks[2], Block::UnorderedList(_)));
    assert!(matches!(outcome.blocks[3], Block::Paragraph(_)));
    assert!(matches!(outcome.blocks[4], Block::Math(_)));
    assert_eq!(outcome.blocks.len(), 5);
}

// --------------------------------------------- tight lists and loose content

#[test]
fn a_tight_list_item_still_gets_a_paragraph() {
    // Tight items emit inline events with no enclosing paragraph; the builder
    // has to supply one so every list item has the same shape.
    let parsed = blocks("- one\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert_eq!(items[0].blocks, vec![Block::paragraph(vec![text("one")])]);
}

#[test]
fn a_loose_list_item_is_shaped_like_a_tight_one() {
    let tight = blocks("- one\n- two\n");
    let loose = blocks("- one\n\n- two\n");
    assert_eq!(tight, loose);
}

#[test]
fn text_followed_by_a_nested_list_yields_two_sibling_blocks() {
    let parsed = blocks("- Outer\n  - Inner\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert_eq!(items[0].blocks[0], Block::paragraph(vec![text("Outer")]));
    let Block::UnorderedList(inner) = &items[0].blocks[1] else {
        panic!("expected a nested list")
    };
    assert_eq!(inner[0].blocks, vec![Block::paragraph(vec![text("Inner")])]);
}

#[test]
fn a_tight_item_containing_only_display_math_is_hoisted() {
    let parsed = blocks("- $$x = 1$$\n");
    let [Block::UnorderedList(items)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    assert!(
        matches!(items[0].blocks[0], Block::Math(_)),
        "{:?}",
        items[0].blocks
    );
}

#[test]
fn a_paragraph_of_only_whitespace_is_dropped() {
    assert!(blocks("   \n").is_empty());
}

#[test]
fn a_blockquote_with_a_tight_body_keeps_its_paragraph() {
    let parsed = blocks("> quoted\n");
    let [Block::BlockQuote(inner)] = parsed.as_slice() else {
        panic!("expected a blockquote")
    };
    assert_eq!(inner, &vec![Block::paragraph(vec![text("quoted")])]);
}

#[test]
fn deeply_nested_lists_keep_their_structure() {
    let parsed = blocks("- a\n  - b\n    - c\n");
    let [Block::UnorderedList(level1)] = parsed.as_slice() else {
        panic!("expected a list")
    };
    let Block::UnorderedList(level2) = &level1[0].blocks[1] else {
        panic!("expected level 2")
    };
    let Block::UnorderedList(level3) = &level2[0].blocks[1] else {
        panic!("expected level 3")
    };
    assert_eq!(level3[0].blocks, vec![Block::paragraph(vec![text("c")])]);
}
