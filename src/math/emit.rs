//! RaTeX AST to Typst mathematics.
//!
//! Translation is structural: every node becomes the Typst construct with the
//! same meaning, and a node that has no faithful Typst equivalent is reported
//! rather than approximated. There are no string substitutions on the LaTeX
//! source anywhere in this file.
//!
//! The one subtlety worth knowing is *joining*. Typst reads a run of adjacent
//! letters as a single identifier, so `xy` would be the symbol `xy` rather than
//! two variables. Fragments are therefore separated by spaces, except when both
//! sides are numeric, which keeps `0.42` from becoming `0 . 4 2`.

use ratex_parser::parse_node::{Measurement, ParseNode, StyleStr};

use crate::ir::MathMode;

use super::error::MathError;
use super::symbols;

type Result<T> = std::result::Result<T, MathError>;

/// A piece of emitted Typst, plus what it needs to join correctly.
struct Fragment {
    text: String,
    /// Whether the fragment is digits and dots only.
    numeric: bool,
}

impl Fragment {
    fn new(text: String) -> Self {
        let numeric = !text.is_empty()
            && text
                .chars()
                .all(|character| character.is_ascii_digit() || character == '.');
        Self { text, numeric }
    }

    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            numeric: false,
        }
    }

    fn empty() -> Self {
        Self {
            text: String::new(),
            numeric: false,
        }
    }
}

/// Translate a parsed equation into Typst mathematics.
pub fn emit(nodes: &[ParseNode], mode: MathMode) -> Result<String> {
    let _ = mode;
    Emitter.sequence(nodes).map(|fragment| fragment.text)
}

struct Emitter;

impl Emitter {
    /// Emit a sequence of sibling nodes, separated so Typst reads them apart.
    fn sequence(&self, nodes: &[ParseNode]) -> Result<Fragment> {
        let mut out = String::new();
        let mut previous_numeric = false;
        let mut after_unary_sign = false;
        let mut count = 0;

        for node in nodes {
            let fragment = self.node(node)?;
            if fragment.text.is_empty() {
                continue;
            }

            let joins = (previous_numeric && fragment.numeric) || after_unary_sign;
            if !out.is_empty() && !joins {
                out.push(' ');
            }

            // A leading `-` or `+` signs what follows rather than separating two
            // operands, so it stays attached: `x^(-1)`, not `x^(- 1)`.
            after_unary_sign = out.is_empty() && matches!(fragment.text.as_str(), "-" | "+");
            out.push_str(&fragment.text);
            previous_numeric = fragment.numeric;
            count += 1;
        }

        // A lone fragment keeps its own joining behaviour, so that `12` inside a
        // group still counts as numeric to its neighbours.
        let numeric = count == 1 && previous_numeric;
        Ok(Fragment { text: out, numeric })
    }

    fn node(&self, node: &ParseNode) -> Result<Fragment> {
        match node {
            // ---------------------------------------------------- symbols
            ParseNode::Atom { text, .. }
            | ParseNode::MathOrd { text, .. }
            | ParseNode::TextOrd { text, .. }
            | ParseNode::OpToken { text, .. }
            | ParseNode::AccentToken { text, .. } => self.symbol(text),

            ParseNode::SpacingNode { text, .. } => match text.trim() {
                "" => Ok(Fragment::plain("space")),
                other => self.symbol(other),
            },

            // ------------------------------------------------- structure
            ParseNode::OrdGroup { body, .. } => self.sequence(body),

            ParseNode::SupSub { base, sup, sub, .. } => {
                self.supsub(base.as_deref(), sup.as_deref(), sub.as_deref())
            }

            ParseNode::GenFrac {
                numer,
                denom,
                has_bar_line,
                left_delim,
                right_delim,
                ..
            } => self.fraction(numer, denom, *has_bar_line, left_delim, right_delim),

            ParseNode::Sqrt { body, index, .. } => {
                let body = self.argument(body)?;
                Ok(Fragment::plain(match index {
                    Some(index) => format!("root({}, {body})", self.argument(index)?),
                    None => format!("sqrt({body})"),
                }))
            }

            ParseNode::Accent { label, base, .. } | ParseNode::AccentUnder { label, base, .. } => {
                let function = symbols::accent(label)
                    .ok_or_else(|| MathError::unsupported(format!("the accent `{label}`")))?;
                Ok(Fragment::plain(format!(
                    "{function}({})",
                    self.argument(base)?
                )))
            }

            ParseNode::Overline { body, .. } => Ok(Fragment::plain(format!(
                "overline({})",
                self.argument(body)?
            ))),
            ParseNode::Underline { body, .. } => Ok(Fragment::plain(format!(
                "underline({})",
                self.argument(body)?
            ))),

            ParseNode::HorizBrace {
                label,
                is_over,
                base,
                ..
            } => {
                let function = if *is_over { "overbrace" } else { "underbrace" };
                let _ = label;
                Ok(Fragment::plain(format!(
                    "{function}({})",
                    self.argument(base)?
                )))
            }

            // ------------------------------------------------- operators
            ParseNode::Op {
                name,
                symbol,
                limits,
                body,
                ..
            } => self.operator(name.as_deref(), *symbol, *limits, body.as_deref()),

            ParseNode::OperatorName {
                body,
                limits,
                always_handle_sup_sub,
                ..
            } => {
                let text = literal_text(body);
                // `\operatorname*` — and the operators defined with it, such as
                // `\argmax` — set their scripts as limits.
                let limits = if *limits || *always_handle_sup_sub {
                    ", limits: #true"
                } else {
                    ""
                };
                Ok(Fragment::plain(format!(
                    "op(\"{}\"{limits})",
                    crate::typst::escape::string(&text)
                )))
            }

            // ----------------------------------------------------- fonts
            ParseNode::Font { font, body, .. } => {
                let function = symbols::font(font)
                    .ok_or_else(|| MathError::unsupported(format!("the font `\\{font}`")))?;
                Ok(Fragment::plain(format!(
                    "{function}({})",
                    self.argument(body)?
                )))
            }

            ParseNode::Pmb { body, .. } => Ok(Fragment::plain(format!(
                "bold({})",
                self.sequence(body)?.text
            ))),

            ParseNode::Text { body, .. } => {
                let text = literal_text(body);
                Ok(Fragment::plain(format!(
                    "\"{}\"",
                    crate::typst::escape::string(&text)
                )))
            }

            ParseNode::Styling { style, body, .. } => {
                let function = match style {
                    StyleStr::Display => "display",
                    StyleStr::Text => "inline",
                    StyleStr::Script => "script",
                    StyleStr::Scriptscript => "sscript",
                };
                Ok(Fragment::plain(format!(
                    "{function}({})",
                    self.sequence(body)?.text
                )))
            }

            // Sizing is presentational and has no mathematical content.
            ParseNode::Sizing { body, .. } => self.sequence(body),

            // ------------------------------------------------ delimiters
            ParseNode::LeftRight {
                body, left, right, ..
            } => self.left_right(body, left, right),

            ParseNode::DelimSizing { delim, .. } | ParseNode::LeftRightRight { delim, .. } => {
                self.symbol(delim)
            }

            ParseNode::Middle { delim, .. } => Ok(Fragment::plain(format!(
                "mid({})",
                self.symbol(delim)?.text
            ))),

            // ---------------------------------------------------- arrays
            ParseNode::Array {
                body,
                cols,
                col_separation_type,
                ..
            } => self.array(body, cols.as_deref(), col_separation_type.as_deref(), None),

            // ---------------------------------------------------- spacing
            ParseNode::Kern { dimension, .. } => Ok(Fragment::plain(spacing(dimension))),

            // ------------------------------------------------ transparent
            ParseNode::MClass { body, .. }
            | ParseNode::HBox { body, .. }
            | ParseNode::MathChoice { text: body, .. } => self.sequence(body),

            ParseNode::Smash { body, .. }
            | ParseNode::Lap { body, .. }
            | ParseNode::RaiseBox { body, .. }
            | ParseNode::VCenter { body, .. } => Ok(Fragment::plain(self.argument(body)?)),

            ParseNode::Phantom { body, .. } => Ok(Fragment::plain(format!(
                "hide({})",
                self.sequence(body)?.text
            ))),
            ParseNode::VPhantom { body, .. } => {
                Ok(Fragment::plain(format!("hide({})", self.argument(body)?)))
            }

            ParseNode::Enclose { label, body, .. } => match label.as_str() {
                r"\cancel" => Ok(Fragment::plain(format!("cancel({})", self.argument(body)?))),
                r"\bcancel" => Ok(Fragment::plain(format!(
                    "cancel({}, inverted: #true)",
                    self.argument(body)?
                ))),
                r"\xcancel" => Ok(Fragment::plain(format!(
                    "cancel({}, cross: #true)",
                    self.argument(body)?
                ))),
                other => Err(MathError::unsupported(format!("`{other}`"))),
            },

            ParseNode::XArrow {
                label, body, below, ..
            } => {
                let arrow = symbols::extensible_arrow(label)
                    .ok_or_else(|| MathError::unsupported(format!("`{label}`")))?;
                let mut out = format!("attach({arrow}, t: {}", self.argument(body)?);
                if let Some(below) = below {
                    out.push_str(&format!(", b: {}", self.argument(below)?));
                }
                out.push(')');
                Ok(Fragment::plain(out))
            }

            ParseNode::Verb { body, .. } => Ok(Fragment::plain(format!(
                "mono(\"{}\")",
                crate::typst::escape::string(body)
            ))),

            // A hard line break inside an equation.
            ParseNode::Cr { .. } => Ok(Fragment::plain("\\")),

            // Carry no mathematical content.
            ParseNode::Internal { .. } | ParseNode::NoNumber { .. } | ParseNode::Size { .. } => {
                Ok(Fragment::empty())
            }

            // ------------------------------------------------ unsupported
            ParseNode::Color { .. } | ParseNode::ColorToken { .. } => {
                Err(MathError::unsupported("coloured mathematics"))
            }
            ParseNode::Environment { name, .. } => {
                Err(MathError::unsupported(format!("the `{name}` environment")))
            }
            ParseNode::CdLabel { .. }
            | ParseNode::CdLabelParent { .. }
            | ParseNode::CdArrow { .. } => Err(MathError::unsupported("the CD environment")),
            ParseNode::ProofTree { .. } => Err(MathError::unsupported("proof trees")),
            // KaTeX composes a few symbols — `\neq`, `\notin` — out of
            // overlapping glyphs for HTML, while carrying the single clean
            // character in the MathML branch. That branch is what Typst wants.
            ParseNode::HtmlMathMl { mathml, .. } => self.sequence(mathml),

            ParseNode::Html { body, .. } => self.sequence(body),
            ParseNode::Href { .. } | ParseNode::Url { .. } => {
                Err(MathError::unsupported("links inside mathematics"))
            }
            ParseNode::IncludeGraphics { .. } => Err(MathError::unsupported("`\\includegraphics`")),
            ParseNode::Rule { .. } => Err(MathError::unsupported("`\\rule`")),
            ParseNode::Tag { .. } => Err(MathError::unsupported("equation tags")),
            ParseNode::Infix { replace_with, .. } => {
                Err(MathError::unsupported(format!("`{replace_with}`")))
            }
            ParseNode::Raw { .. } => Err(MathError::unsupported("raw text inside mathematics")),
        }
    }

    /// Resolve a symbol token, or report it.
    fn symbol(&self, token: &str) -> Result<Fragment> {
        match symbols::symbol(token) {
            Some(typst) => Ok(Fragment::new(typst.into_owned())),
            None => Err(MathError::unsupported(format!("the symbol `{token}`"))),
        }
    }

    /// Emit a node as a function argument, parenthesized when Typst would
    /// otherwise read it as several arguments or tokens.
    fn argument(&self, node: &ParseNode) -> Result<String> {
        Ok(self.node(node)?.text)
    }

    /// Emit a node where Typst needs a single token, as a script or an index.
    fn token(&self, node: &ParseNode) -> Result<String> {
        let text = self.node(node)?.text;
        Ok(if is_atomic(&text) {
            text
        } else {
            format!("({text})")
        })
    }

    fn supsub(
        &self,
        base: Option<&ParseNode>,
        sup: Option<&ParseNode>,
        sub: Option<&ParseNode>,
    ) -> Result<Fragment> {
        // `\overbrace{x}^{note}` arrives as a brace with a superscript; Typst
        // takes the annotation as the brace's second argument.
        if let (
            Some(ParseNode::HorizBrace {
                is_over,
                base: braced,
                ..
            }),
            Some(annotation),
        ) = (base, sup.or(sub))
        {
            let function = if *is_over { "overbrace" } else { "underbrace" };
            return Ok(Fragment::plain(format!(
                "{function}({}, {})",
                self.argument(braced)?,
                self.argument(annotation)?
            )));
        }

        let mut out = match base {
            Some(base) => self.token(base)?,
            // `{}^2` has no base; an empty string keeps the script attached to
            // something without printing anything.
            None => "\"\"".to_string(),
        };

        if let Some(sub) = sub {
            out.push('_');
            out.push_str(&self.token(sub)?);
        }

        if let Some(sup) = sup {
            // A run of primes is written postfix rather than as a superscript.
            match primes(sup) {
                Some(count) => out.push_str(&"'".repeat(count)),
                None => {
                    out.push('^');
                    out.push_str(&self.token(sup)?);
                }
            }
        }

        Ok(Fragment::plain(out))
    }

    fn fraction(
        &self,
        numer: &ParseNode,
        denom: &ParseNode,
        has_bar_line: bool,
        left: &Option<String>,
        right: &Option<String>,
    ) -> Result<Fragment> {
        let numer = self.argument(numer)?;
        let denom = self.argument(denom)?;

        if has_bar_line {
            return Ok(Fragment::plain(format!("frac({numer}, {denom})")));
        }

        // `\binom` and `\choose` are a barless fraction in parentheses.
        match (left.as_deref(), right.as_deref()) {
            (Some("("), Some(")")) => Ok(Fragment::plain(format!("binom({numer}, {denom})"))),
            _ => Err(MathError::unsupported("a fraction without a bar")),
        }
    }

    fn operator(
        &self,
        name: Option<&str>,
        is_symbol: bool,
        limits: bool,
        body: Option<&[ParseNode]>,
    ) -> Result<Fragment> {
        // `\mathop{...}` supplies its own body.
        if let Some(body) = body {
            let inner = self.sequence(body)?.text;
            let limits = if limits { ", limits: #true" } else { "" };
            return Ok(Fragment::plain(format!("op({inner}{limits})")));
        }

        let name = name.ok_or_else(|| MathError::unsupported("an unnamed operator"))?;

        if is_symbol {
            // A large operator such as `\sum`: the character carries the
            // behaviour, including limit placement.
            return self.symbol(name);
        }

        // A word operator such as `\lim`. Typst's built-ins bring the right
        // spacing and limits; anything else becomes an explicit `op`.
        match symbols::operator(name) {
            Some(function) => Ok(Fragment::plain(function)),
            None => {
                let bare = name.strip_prefix('\\').unwrap_or(name);
                let limits = if limits { ", limits: #true" } else { "" };
                Ok(Fragment::plain(format!(
                    "op(\"{}\"{limits})",
                    crate::typst::escape::string(bare)
                )))
            }
        }
    }

    fn left_right(&self, body: &[ParseNode], left: &str, right: &str) -> Result<Fragment> {
        // `\begin{pmatrix}` and friends arrive as a delimited array. Typst has a
        // dedicated construct for each, which sets far better than wrapping a
        // bare matrix in delimiters.
        if let [ParseNode::Array {
            body: rows,
            cols,
            col_separation_type,
            ..
        }] = body
        {
            if let Some(delimiter) = matrix_delimiter(left, right) {
                return self.array(
                    rows,
                    cols.as_deref(),
                    col_separation_type.as_deref(),
                    Some(delimiter),
                );
            }
            if normalize_delimiter(left) == "{" && right == "." {
                return self.cases(rows);
            }
        }

        let inner = self.sequence(body)?.text;
        let left = self.delimiter(left)?;
        let right = self.delimiter(right)?;

        match (left, right) {
            (None, None) => Ok(Fragment::plain(inner)),
            (left, right) => {
                let left = left.map(|d| format!("{d} ")).unwrap_or_default();
                let right = right.map(|d| format!(" {d}")).unwrap_or_default();
                Ok(Fragment::plain(format!("lr({left}{inner}{right})")))
            }
        }
    }

    /// An `lr` fence, or `None` for LaTeX's null delimiter `.`.
    fn delimiter(&self, token: &str) -> Result<Option<String>> {
        if token == "." {
            return Ok(None);
        }
        Ok(Some(escape_fence(&self.symbol(token)?.text)))
    }

    fn array(
        &self,
        rows: &[Vec<ParseNode>],
        cols: Option<&[ratex_parser::parse_node::AlignSpec]>,
        separation: Option<&str>,
        delimiter: Option<&str>,
    ) -> Result<Fragment> {
        // `aligned` and `align` set rows with alignment points, which Typst
        // writes with `&` exactly as LaTeX does.
        if separation == Some("align") {
            let mut lines = Vec::with_capacity(rows.len());
            for row in rows {
                let mut cells = Vec::with_capacity(row.len());
                for cell in row {
                    cells.push(self.cell(cell)?);
                }
                lines.push(cells.join(" & "));
            }
            return Ok(Fragment::plain(lines.join(" \\\n")));
        }

        let mut lines = Vec::with_capacity(rows.len());
        for row in rows {
            let mut cells = Vec::with_capacity(row.len());
            for cell in row {
                cells.push(self.cell(cell)?);
            }
            lines.push(cells.join(", "));
        }
        // A trailing empty row is how `\\` before `\end` parses; it would add a
        // blank line to the matrix.
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        let mut arguments = Vec::new();
        if let Some(delimiter) = delimiter {
            arguments.push(format!("delim: {delimiter}"));
        } else {
            arguments.push("delim: #none".to_string());
        }
        if let Some(align) = column_alignment(cols) {
            arguments.push(format!("align: {align}"));
        }
        arguments.push(lines.join("; "));

        Ok(Fragment::plain(format!("mat({})", arguments.join(", "))))
    }

    fn cases(&self, rows: &[Vec<ParseNode>]) -> Result<Fragment> {
        let mut lines = Vec::with_capacity(rows.len());
        for row in rows {
            let mut cells = Vec::with_capacity(row.len());
            for cell in row {
                cells.push(self.cell(cell)?);
            }
            let line = cells.join(" & ");
            if !line.is_empty() {
                lines.push(line);
            }
        }
        Ok(Fragment::plain(format!("cases({})", lines.join(", "))))
    }

    /// Emit one array cell.
    ///
    /// Cells arrive wrapped in a styling node that merely records the array's
    /// own style; emitting it would wrap every cell in `inline(...)` for no
    /// reason, so it is unwrapped here.
    fn cell(&self, cell: &ParseNode) -> Result<String> {
        match cell {
            ParseNode::Styling { body, .. } => Ok(self.sequence(body)?.text),
            other => Ok(self.node(other)?.text),
        }
    }
}

/// Whether Typst will read `text` as a single unit, so that it can follow `^` or
/// `_` without another layer of parentheses.
///
/// Being wrong in the conservative direction only adds parentheses, which change
/// nothing in the output, so this errs towards wrapping.
fn is_atomic(text: &str) -> bool {
    let mut characters = text.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    // A single character, unless it is structural. A *closing* delimiter is
    // allowed: it is how `(X^top X)^(-1)` reaches here, as a script on `)`.
    if characters.next().is_none() {
        return !"([{,;^_$#\"\\ ".contains(first);
    }

    // A symbol name such as `epsilon.alt`.
    let is_name = |text: &str| {
        text.starts_with(|c: char| c.is_ascii_alphabetic())
            && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')
            && !text.ends_with('.')
    };
    if is_name(text) {
        return true;
    }

    // A quoted string, such as the output of `\text`.
    if text.starts_with('"') && text.ends_with('"') && text.len() > 1 {
        return true;
    }

    // A parenthesized group, `(x + y)`, or a call, `frac(x, y)`.
    match text.split_once('(') {
        Some((head, _)) if head.is_empty() || is_name(head) => {
            text.ends_with(')') && closes_at_end(text)
        }
        _ => false,
    }
}

/// Whether the parenthesis opened first in `text` is the one closed at its end.
fn closes_at_end(text: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    for (index, character) in text.char_indices() {
        match character {
            '"' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return index + character.len_utf8() == text.len();
                }
            }
            _ => {}
        }
    }
    false
}

/// The number of primes, when a superscript is nothing but primes.
fn primes(sup: &ParseNode) -> Option<usize> {
    let body = match sup {
        ParseNode::OrdGroup { body, .. } => body.as_slice(),
        single => std::slice::from_ref(single),
    };

    if body.is_empty() {
        return None;
    }

    body.iter()
        .all(|node| matches!(node, ParseNode::TextOrd { text, .. } if text == r"\prime"))
        .then_some(body.len())
}

/// The Typst `mat` delimiter for a LaTeX matrix environment's fences.
///
/// KaTeX reports the fences as written, so `\{` and `{` both occur.
fn matrix_delimiter(left: &str, right: &str) -> Option<&'static str> {
    match (normalize_delimiter(left), normalize_delimiter(right)) {
        ("(", ")") => Some("\"(\""),
        ("[", "]") => Some("\"[\""),
        ("{", "}") => Some("\"{\""),
        ("|", "|") => Some("\"|\""),
        // Typst's `delim` takes a single character, so the double bar is the
        // U+2016 glyph rather than two pipes.
        ("||", "||") => Some("\"\u{2016}\""),
        _ => None,
    }
}

/// Escape a bracket used as an `lr` fence.
///
/// Typst parses brackets structurally, so `\left( x \right.` would otherwise
/// emit an unclosed delimiter and fail to compile. An escaped fence is still
/// recognized and scaled by `lr` — measured to the same height as an unescaped
/// one — so escaping unconditionally is both safe and uniform.
fn escape_fence(delimiter: &str) -> String {
    match delimiter {
        "(" | ")" | "[" | "]" | "{" | "}" => format!("\\{delimiter}"),
        other => other.to_string(),
    }
}

/// Reduce a delimiter token to its bare character.
fn normalize_delimiter(token: &str) -> &str {
    match token {
        r"\{" | r"\lbrace" => "{",
        r"\}" | r"\rbrace" => "}",
        r"\|" | r"\Vert" | r"\lVert" | r"\rVert" => "||",
        r"\vert" | r"\lvert" | r"\rvert" => "|",
        other => other,
    }
}

/// The Typst `align` argument for an array's column specification.
///
/// Only a uniform alignment is expressed; Typst's default centring is right for
/// a matrix, and a mixed specification is left to the default rather than
/// silently misrepresented.
fn column_alignment(cols: Option<&[ratex_parser::parse_node::AlignSpec]>) -> Option<&'static str> {
    let cols = cols?;
    let mut alignments = cols.iter().filter_map(|spec| spec.align.as_deref());
    let first = alignments.next()?;
    if !alignments.all(|align| align == first) {
        return None;
    }
    // The value is a Typst alignment, so it is written in code mode: inside a
    // math call, a bare `left` would be read as an unknown symbol.
    match first {
        "l" => Some("#left"),
        "r" => Some("#right"),
        _ => None,
    }
}

/// Translate a kern into Typst spacing.
///
/// LaTeX's math units are eighteenths of an em. The four named widths cover the
/// spacing macros users actually write; anything else becomes an explicit
/// horizontal space so the author's intent survives.
fn spacing(dimension: &Measurement) -> String {
    let em = match dimension.unit.as_str() {
        "mu" => dimension.number / 18.0,
        "em" => dimension.number,
        other => return format!("#h({}{other})", trim_number(dimension.number)),
    };

    for (width, name) in [
        (1.0 / 6.0, "thin"),
        (2.0 / 9.0, "med"),
        (5.0 / 18.0, "thick"),
        (1.0, "quad"),
        (2.0, "wide"),
    ] {
        if (em - width).abs() < 1e-9 {
            return name.to_string();
        }
    }

    format!("#h({}em)", trim_number(em))
}

/// Format a number without a trailing `.0`.
fn trim_number(value: f64) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text.is_empty() || text == "-" {
        "0".to_string()
    } else {
        text.to_string()
    }
}

/// The literal characters of a text-mode body, for `\text` and `\operatorname`.
fn literal_text(body: &[ParseNode]) -> String {
    let mut out = String::new();
    for node in body {
        match node {
            ParseNode::TextOrd { text, .. }
            | ParseNode::MathOrd { text, .. }
            | ParseNode::Atom { text, .. } => match text.strip_prefix('\\') {
                // A command inside text, such as `\&`.
                Some(_) => {
                    if let Some(info) = ratex_font::get_text_symbol(text) {
                        if let Some(codepoint) = info.codepoint {
                            out.push(codepoint);
                        }
                    }
                }
                None => out.push_str(text),
            },
            ParseNode::SpacingNode { .. } => out.push(' '),
            ParseNode::OrdGroup { body, .. } | ParseNode::Text { body, .. } => {
                out.push_str(&literal_text(body));
            }
            _ => {}
        }
    }
    out
}
