//! Unit tests for LaTeX-to-Typst mathematics translation.

use crate::ir::MathMode;
use crate::math::{to_typst, MathError};

/// Translate display mathematics, panicking on failure.
fn typst(latex: &str) -> String {
    to_typst(latex, MathMode::Display)
        .unwrap_or_else(|error| panic!("{latex:?} failed to translate: {error}"))
}

fn failure(latex: &str) -> MathError {
    match to_typst(latex, MathMode::Display) {
        Ok(typst) => panic!("expected {latex:?} to fail, got {typst:?}"),
        Err(error) => error,
    }
}

// ------------------------------------------------------- variables and joining

#[test]
fn a_single_variable() {
    assert_eq!(typst("x"), "x");
}

#[test]
fn adjacent_variables_are_separated_so_typst_reads_them_apart() {
    // `xy` would be the single unknown symbol `xy` in Typst.
    assert_eq!(typst("xy"), "x y");
    assert_eq!(typst("abc"), "a b c");
}

#[test]
fn digits_of_a_number_stay_together() {
    assert_eq!(typst("123"), "123");
    assert_eq!(typst("0.42"), "0.42");
}

#[test]
fn a_number_next_to_a_variable_is_separated() {
    assert_eq!(typst("2x"), "2 x");
    assert_eq!(typst("x2"), "x 2");
}

#[test]
fn operators_are_spaced() {
    assert_eq!(typst("a + b - c"), "a + b - c");
    assert_eq!(typst("x = y"), "x = y");
}

// -------------------------------------------------------------------- scripts

#[test]
fn subscripts_and_superscripts() {
    assert_eq!(typst("x_i"), "x_i");
    assert_eq!(typst("x^2"), "x^2");
    assert_eq!(typst("x_i^2"), "x_i^2");
}

#[test]
fn a_multi_token_script_is_parenthesized() {
    assert_eq!(typst("x^{2n}"), "x^(2 n)");
    assert_eq!(typst("x_{i,j}"), "x_(i , j)");
    assert_eq!(typst("x^{-1}"), "x^(-1)");
}

#[test]
fn a_symbol_script_needs_no_parentheses() {
    assert_eq!(typst(r"X^\top"), "X^⊤");
}

#[test]
fn a_compound_base_keeps_its_grouping() {
    assert_eq!(typst(r"(X^\top X)^{-1}"), "( X^⊤ X )^(-1)");
}

#[test]
fn primes_are_written_postfix() {
    assert_eq!(typst("x'"), "x'");
    assert_eq!(typst("x''"), "x''");
    assert_eq!(typst(r"\mathbf{x}'\beta"), "bold(x)' β");
}

// ------------------------------------------------------- fractions and roots

#[test]
fn fractions() {
    assert_eq!(typst(r"\frac{x}{y}"), "frac(x, y)");
    assert_eq!(typst(r"\frac{x_i^2}{\sqrt{n}}"), "frac(x_i^2, sqrt(n))");
}

#[test]
fn display_and_inline_fraction_variants_keep_their_style() {
    assert_eq!(typst(r"\dfrac{a}{b}"), "display(frac(a, b))");
    assert_eq!(typst(r"\tfrac{a}{b}"), "inline(frac(a, b))");
}

#[test]
fn roots() {
    assert_eq!(typst(r"\sqrt{x}"), "sqrt(x)");
    assert_eq!(typst(r"\sqrt[n]{x}"), "root(n, x)");
}

#[test]
fn binomial_coefficients() {
    assert_eq!(typst(r"\binom{n}{k}"), "binom(n, k)");
}

// --------------------------------------------------------------- Greek letters

#[test]
fn greek_letters_become_their_characters() {
    assert_eq!(typst(r"\alpha + \beta"), "α + β");
    assert_eq!(typst(r"\Gamma"), "Γ");
    assert_eq!(typst(r"\varepsilon"), "ε");
    assert_eq!(typst(r"\epsilon"), "ϵ");
}

// ------------------------------------------------------------ large operators

#[test]
fn sums_and_products_keep_their_limits() {
    assert_eq!(typst(r"\sum_{i=1}^n x_i"), "∑_(i = 1)^n x_i");
    assert_eq!(typst(r"\prod_{i=1}^n x_i"), "∏_(i = 1)^n x_i");
}

#[test]
fn integrals() {
    assert_eq!(typst(r"\int_0^1 f(x)\,dx"), "∫_0^1 f ( x ) thin d x");
    assert_eq!(typst(r"\int_{-\infty}^{\infty}"), "∫_(-∞)^∞");
}

#[test]
fn word_operators_use_typst_builtins() {
    assert_eq!(typst(r"\log x"), "log x");
    assert_eq!(typst(r"\lim_{n\to\infty} x_n"), "lim_(n → ∞) x_n");
    assert_eq!(typst(r"\Pr(Y = 1)"), "Pr ( Y = 1 )");
}

#[test]
fn operatorname_becomes_an_explicit_operator() {
    assert_eq!(typst(r"\operatorname{Var}(X)"), "op(\"Var\") ( X )");
    assert_eq!(typst(r"\operatorname{logit}^{-1}"), "op(\"logit\")^(-1)");
}

#[test]
fn an_operator_unknown_to_typst_becomes_an_explicit_operator() {
    assert_eq!(typst(r"\argmax"), "op(\"argmax\", limits: #true)");
}

// ------------------------------------------------------------- font variants

#[test]
fn math_font_variants() {
    assert_eq!(typst(r"\mathbb{R}"), "bb(R)");
    assert_eq!(typst(r"\mathbf{x}"), "bold(x)");
    assert_eq!(typst(r"\mathcal{F}"), "cal(F)");
    assert_eq!(typst(r"\mathrm{d}"), "upright(d)");
    assert_eq!(typst(r"\mathfrak{g}"), "frak(g)");
    assert_eq!(typst(r"\mathsf{S}"), "sans(S)");
    assert_eq!(typst(r"\mathtt{T}"), "mono(T)");
}

#[test]
fn blackboard_bold_with_a_multi_letter_body() {
    assert_eq!(typst(r"\mathbb{RK}"), "bb(R K)");
}

// -------------------------------------------------------------------- accents

#[test]
fn accents() {
    assert_eq!(typst(r"\hat\beta"), "hat(β)");
    assert_eq!(typst(r"\bar{x}"), "macron(x)");
    assert_eq!(typst(r"\tilde{y}"), "tilde(y)");
    assert_eq!(typst(r"\vec{v}"), "arrow(v)");
    assert_eq!(typst(r"\dot{z}"), "dot(z)");
    assert_eq!(typst(r"\ddot{z}"), "dot.double(z)");
}

#[test]
fn overline_and_underline() {
    assert_eq!(typst(r"\overline{x}"), "overline(x)");
    assert_eq!(typst(r"\underline{x}"), "underline(x)");
}

#[test]
fn an_accent_over_a_compound_body() {
    assert_eq!(typst(r"\hat{\beta_1}"), "hat(β_1)");
}

// ----------------------------------------------------------------- delimiters

#[test]
fn paired_delimiters_scale() {
    // Fences are escaped so that Typst reads them as symbols rather than as
    // grouping syntax; they still scale.
    assert_eq!(typst(r"\left(\frac{x}{y}\right)"), r"lr(\( frac(x, y) \))");
    assert_eq!(typst(r"\left[x\right]"), r"lr(\[ x \])");
    assert_eq!(typst(r"\left\{x\right\}"), r"lr(\{ x \})");
}

#[test]
fn a_null_delimiter_is_omitted_on_that_side() {
    assert_eq!(typst(r"\left. x \right|"), "lr(x |)");
    assert_eq!(typst(r"\left( x \right."), r"lr(\( x)");
}

#[test]
fn plain_delimiters_pass_through() {
    assert_eq!(typst("(x)"), "( x )");
    assert_eq!(typst("[x]"), "[ x ]");
    assert_eq!(typst(r"\{x\}"), "{ x }");
}

// -------------------------------------------------------------------- matrices

#[test]
fn a_parenthesized_matrix() {
    assert_eq!(
        typst(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
        "mat(delim: \"(\", a, b; c, d)"
    );
}

#[test]
fn each_matrix_environment_gets_its_delimiter() {
    assert_eq!(
        typst(r"\begin{bmatrix} a \end{bmatrix}"),
        "mat(delim: \"[\", a)"
    );
    assert_eq!(
        typst(r"\begin{vmatrix} a \end{vmatrix}"),
        "mat(delim: \"|\", a)"
    );
    assert_eq!(
        typst(r"\begin{Bmatrix} a \end{Bmatrix}"),
        "mat(delim: \"{\", a)"
    );
    assert_eq!(
        typst(r"\begin{Vmatrix} a \end{Vmatrix}"),
        "mat(delim: \"\u{2016}\", a)"
    );
}

#[test]
fn a_bare_matrix_has_no_delimiters() {
    assert_eq!(
        typst(r"\begin{matrix} a & b \\ c & d \end{matrix}"),
        "mat(delim: #none, a, b; c, d)"
    );
}

#[test]
fn an_array_keeps_a_uniform_column_alignment() {
    assert_eq!(
        typst(r"\begin{array}{ll} a & b \end{array}"),
        "mat(delim: #none, align: #left, a, b)"
    );
}

// --------------------------------------------------------- aligned and cases

#[test]
fn aligned_equations_use_typst_alignment_points() {
    assert_eq!(
        typst(r"\begin{aligned} y &= x + 1 \\ z &= y + 2 \end{aligned}"),
        "y & = x + 1 \\\nz & = y + 2"
    );
}

#[test]
fn cases() {
    assert_eq!(
        typst(r"\begin{cases} x & y > 0 \\ z & \text{otherwise} \end{cases}"),
        "cases(x & y > 0, z & \"otherwise\")"
    );
}

// ------------------------------------------------------------ text and spacing

#[test]
fn text_inside_mathematics_becomes_a_string() {
    assert_eq!(typst(r"\text{if }x"), "\"if \" x");
    assert_eq!(typst(r"\text{robust}"), "\"robust\"");
}

#[test]
fn text_with_a_quotation_mark_is_escaped() {
    assert_eq!(typst(r#"\text{a"b}"#), r#""a\"b""#);
}

#[test]
fn spacing_macros_map_to_typst_widths() {
    assert_eq!(typst(r"a\,b"), "a thin b");
    assert_eq!(typst(r"a\;b"), "a thick b");
    assert_eq!(typst(r"a\quad b"), "a quad b");
    assert_eq!(typst(r"a\qquad b"), "a wide b");
}

#[test]
fn a_negative_thin_space_becomes_an_explicit_width() {
    assert_eq!(typst(r"a\!b"), "a #h(-0.1667em) b");
}

// --------------------------------------------------------------- relations etc

#[test]
fn relations_and_set_notation() {
    assert_eq!(typst(r"x \le y"), "x ≤ y");
    assert_eq!(typst(r"x \ge y"), "x ≥ y");
    assert_eq!(typst(r"x \neq y"), "x ≠ y");
    assert_eq!(typst(r"x \approx y"), "x ≈ y");
    assert_eq!(typst(r"x \in A"), "x ∈ A");
    assert_eq!(typst(r"A \subseteq B"), "A ⊆ B");
    assert_eq!(typst(r"A \cup B"), "A ∪ B");
    assert_eq!(typst(r"A \cap B"), "A ∩ B");
}

#[test]
fn conditional_expectation_notation() {
    assert_eq!(typst(r"E[Y \mid X]"), "E [ Y ∣ X ]");
    assert_eq!(typst(r"\mathbb E[Y \mid X]"), "bb(E) [ Y ∣ X ]");
}

#[test]
fn arrows() {
    assert_eq!(typst(r"a \to b"), "a → b");
    assert_eq!(typst(r"a \Rightarrow b"), "a ⇒ b");
    assert_eq!(typst(r"a \mapsto b"), "a ↦ b");
}

#[test]
fn calculus_notation() {
    assert_eq!(
        typst(r"\frac{\partial f(x)}{\partial x}"),
        "frac(∂ f ( x ), ∂ x)"
    );
    assert_eq!(typst(r"\nabla f"), "∇ f");
}

#[test]
fn an_ampersand_outside_an_array_cannot_become_an_alignment_point() {
    assert_eq!(typst(r"a \& b"), "a amp b");
}

// ------------------------------------------------------------------- braces

#[test]
fn over_and_underbraces_take_their_annotation_as_an_argument() {
    assert_eq!(typst(r"\overbrace{x+y}^{s}"), "overbrace(x + y, s)");
    assert_eq!(typst(r"\underbrace{x+y}_{s}"), "underbrace(x + y, s)");
}

#[test]
fn an_unannotated_brace_still_works() {
    assert_eq!(typst(r"\overbrace{x}"), "overbrace(x)");
}

// ----------------------------------------------------------------- failures

#[test]
fn an_unknown_command_is_a_parse_error_with_an_offset() {
    let error = failure(r"We \foo{x}");
    assert!(matches!(error, MathError::Parse { .. }), "{error:?}");
    assert!(error.to_string().contains("foo"), "{error}");
    assert!(error.offset().is_some());
}

#[test]
fn a_parse_error_offset_points_into_the_equation() {
    let error = failure(r"x + \foo");
    assert_eq!(error.offset(), Some(4));
}

#[test]
fn an_untranslatable_construct_is_reported_rather_than_approximated() {
    let error = failure(r"\color{red} x");
    assert!(matches!(error, MathError::Unsupported { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        "coloured mathematics is not yet supported"
    );
}

#[test]
fn an_unsupported_error_names_what_was_parsed() {
    let error = failure(r"\begin{CD} A @>>> B \end{CD}");
    assert!(matches!(error, MathError::Unsupported { .. }), "{error:?}");
    assert_eq!(
        error.note().as_deref(),
        Some("Parsed as: the CD environment")
    );
}

#[test]
fn an_unbalanced_group_is_a_parse_error() {
    assert!(matches!(failure(r"\frac{x"), MathError::Parse { .. }));
}

// ------------------------------------------------------- inline versus display

#[test]
fn the_mode_does_not_change_the_translation() {
    // The mode decides how Typst *sets* the result, not how the LaTeX is read.
    let inline = to_typst(r"\sum_{i=1}^n x_i", MathMode::Inline).unwrap();
    let display = to_typst(r"\sum_{i=1}^n x_i", MathMode::Display).unwrap();
    assert_eq!(inline, display);
}

#[test]
fn empty_mathematics_translates_to_nothing() {
    assert_eq!(typst(""), "");
    assert_eq!(typst("   "), "");
}
