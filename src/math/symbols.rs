//! LaTeX symbol names to Typst math.
//!
//! Typst accepts Unicode mathematical characters directly, so a symbol is
//! translated by resolving its LaTeX name to a codepoint and emitting that
//! character. The codepoints come from `ratex-font`, which carries KaTeX's
//! symbol table, so coverage is KaTeX's coverage rather than a table maintained
//! by hand — and it cannot drift from the parser that produced the name.
//!
//! Two kinds of entry still need deciding here:
//!
//! * characters that are *syntax* in Typst math and must be escaped or named;
//! * LaTeX operator words such as `\lim`, which Typst spells as functions.

use std::borrow::Cow;

/// Resolve a symbol token to Typst math markup.
///
/// `token` is either a LaTeX command (`\beta`) or a literal character (`+`).
/// Returns `None` when the symbol is unknown, which the caller reports as an
/// unsupported construct rather than guessing.
pub fn symbol(token: &str) -> Option<Cow<'static, str>> {
    if let Some(fixed) = syntax_override(token) {
        return Some(Cow::Borrowed(fixed));
    }

    // A plain character that is not Typst syntax passes straight through.
    let mut chars = token.chars();
    if let (Some(character), None) = (chars.next(), chars.next()) {
        if let Some(escaped) = escape_math_character(character) {
            return Some(escaped);
        }
    }

    let info = ratex_font::get_math_symbol(token)?;
    let codepoint = info.codepoint?;
    escape_math_character(codepoint)
}

/// Characters that mean something to Typst's math parser.
///
/// Each is either escaped or replaced by the name of the symbol, so that a
/// literal `&` in mathematics cannot become an alignment point.
fn escape_math_character(character: char) -> Option<Cow<'static, str>> {
    Some(match character {
        '&' => Cow::Borrowed("amp"),
        '#' => Cow::Borrowed(r"\#"),
        '$' => Cow::Borrowed(r"\$"),
        '"' => Cow::Borrowed(r#"\""#),
        '_' => Cow::Borrowed(r"\_"),
        '^' => Cow::Borrowed(r"\^"),
        '\\' => Cow::Borrowed(r"\\"),
        // A space is spacing, not a symbol.
        ' ' => Cow::Borrowed("space"),
        other => Cow::Owned(other.to_string()),
    })
}

/// Symbols whose Typst spelling is a name rather than a character.
fn syntax_override(token: &str) -> Option<&'static str> {
    Some(match token {
        // KaTeX resolves these to characters Typst reserves for syntax.
        r"\&" | "&" => "amp",
        r"\#" | "#" => r"\#",
        r"\$" | "$" => r"\$",
        r"\%" | "%" => "%",
        r"\_" => r"\_",
        r"\{" => "{",
        r"\}" => "}",
        r"\vert" | "|" => "|",
        r"\Vert" | r"\|" => "bar.v.double",
        r"\backslash" | r"\setminus" => "without",
        // A prime is written postfix in Typst.
        r"\prime" => "'",
        // `\colon` is a punctuation colon, not the relation `:`.
        r"\colon" => ":",
        // Typst spells these as words; the characters alone set badly.
        r"\quad" => "quad",
        r"\qquad" => "wide",
        r"\ " | r"\space" | r"\nobreakspace" | "~" => "space.nobreak",
        _ => return None,
    })
}

/// Typst's built-in upright operator functions, by LaTeX name.
///
/// Using the built-in rather than `op("...")` gets Typst's own spacing and
/// limit placement, which is what makes `\lim_{n\to\infty}` set correctly.
pub fn operator(name: &str) -> Option<&'static str> {
    let bare = name.strip_prefix('\\').unwrap_or(name);
    Some(match bare {
        "arccos" => "arccos",
        "arcsin" => "arcsin",
        "arctan" => "arctan",
        "arg" => "arg",
        "cos" => "cos",
        "cosh" => "cosh",
        "cot" => "cot",
        "coth" => "coth",
        "csc" => "csc",
        "deg" => "deg",
        "det" => "det",
        "dim" => "dim",
        "exp" => "exp",
        "gcd" => "gcd",
        "hom" => "hom",
        "inf" => "inf",
        "ker" => "ker",
        "lg" => "lg",
        "lim" => "lim",
        "liminf" => "liminf",
        "limsup" => "limsup",
        "ln" => "ln",
        "log" => "log",
        "max" => "max",
        "min" => "min",
        "mod" => "mod",
        "Pr" => "Pr",
        "sec" => "sec",
        "sin" => "sin",
        "sinh" => "sinh",
        "sup" => "sup",
        "tan" => "tan",
        "tanh" => "tanh",
        _ => return None,
    })
}

/// Typst's accent functions, by LaTeX accent command.
pub fn accent(label: &str) -> Option<&'static str> {
    Some(match label {
        r"\hat" | r"\widehat" => "hat",
        r"\bar" => "macron",
        r"\tilde" | r"\widetilde" => "tilde",
        r"\vec" | r"\overrightarrow" => "arrow",
        r"\dot" => "dot",
        r"\ddot" => "dot.double",
        r"\dddot" => "dot.triple",
        r"\ddddot" => "dot.quad",
        r"\check" => "caron",
        r"\breve" => "breve",
        r"\acute" => "acute",
        r"\grave" => "grave",
        r"\mathring" => "circle",
        r"\overline" => "overline",
        r"\underline" => "underline",
        _ => return None,
    })
}

/// Typst's math font functions, by LaTeX font command.
pub fn font(name: &str) -> Option<&'static str> {
    Some(match name.strip_prefix('\\').unwrap_or(name) {
        "mathbb" | "Bbb" => "bb",
        "mathbf" | "bold" | "boldsymbol" | "bm" | "pmb" | "textbf" => "bold",
        "mathrm" | "textrm" | "mathup" | "textup" => "upright",
        "mathit" | "textit" => "italic",
        "mathcal" => "cal",
        "mathscr" => "scr",
        "mathfrak" | "frak" => "frak",
        "mathsf" | "textsf" => "sans",
        "mathtt" | "texttt" => "mono",
        "mathnormal" => "italic",
        _ => return None,
    })
}

/// The arrow character for an extensible-arrow command.
pub fn extensible_arrow(label: &str) -> Option<&'static str> {
    Some(match label {
        r"\xrightarrow" => "arrow.r.long",
        r"\xleftarrow" => "arrow.l.long",
        r"\xRightarrow" => "arrow.r.double.long",
        r"\xLeftarrow" => "arrow.l.double.long",
        r"\xleftrightarrow" => "arrow.l.r.long",
        r"\xLeftrightarrow" => "arrow.l.r.double.long",
        r"\xmapsto" => "arrow.r.long.bar",
        r"\xhookrightarrow" => "arrow.r.long.hook",
        r"\xhookleftarrow" => "arrow.l.long.hook",
        r"\xrightleftharpoons" => "harpoons.rtlb",
        r"\xtwoheadrightarrow" => "arrow.r.long.twohead",
        r"\xtwoheadleftarrow" => "arrow.l.long.twohead",
        _ => return None,
    })
}
