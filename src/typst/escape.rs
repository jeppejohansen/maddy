//! Escaping, centralized.
//!
//! Ordinary Markdown text must never accidentally become Typst syntax. Four
//! contexts need four different rules, and they all live here so that no
//! rendering function invents its own:
//!
//! ```text
//! normal text   markup escaping, with line-start constructs guarded
//! math          built by the math emitter, never by escaping text
//! code/raw      passed to `raw()` as a string literal
//! URLs          passed to `link()` as a string literal
//! ```
//!
//! Note that quotation marks and apostrophes are deliberately *not* escaped:
//! Typst's smart quotes are a typographic feature we want in prose.

/// Characters that are special anywhere in Typst markup.
const ALWAYS: &[char] = &['\\', '#', '$', '*', '_', '`', '[', ']', '<', '>', '@', '~'];

/// Escape text for use in Typst markup, mid-line.
pub fn markup(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes: Vec<char> = input.chars().collect();

    for (index, character) in bytes.iter().copied().enumerate() {
        let next = bytes.get(index + 1).copied();
        let previous = index.checked_sub(1).and_then(|i| bytes.get(i)).copied();

        let escape = ALWAYS.contains(&character)
            // `--` and `---` become dashes; `...` becomes an ellipsis.
            || (character == '-' && (next == Some('-') || previous == Some('-')))
            || (character == '.' && (next == Some('.') || previous == Some('.')))
            // `//` opens a comment and `/*` a block comment.
            || (character == '/' && matches!(next, Some('/') | Some('*')))
            || (character == '*' && previous == Some('/'));

        if escape {
            out.push('\\');
        }
        out.push(character);
    }

    out
}

/// Escape text that will begin a line of generated Typst markup.
///
/// At the start of a line several further characters introduce block
/// constructs — a list, an enumeration, a heading or a term list — so the first
/// character needs guarding as well.
pub fn markup_at_line_start(input: &str) -> String {
    let escaped = markup(input);

    let Some(first) = input.chars().next() else {
        return escaped;
    };

    // `- `, `+ `, `= ` and `/ ` open a block construct.
    if matches!(first, '-' | '+' | '=' | '/') && !escaped.starts_with('\\') {
        return format!("\\{escaped}");
    }

    // A leading run of digits followed by `.` or `)` opens an enumeration.
    if first.is_ascii_digit() {
        let digits = input.chars().take_while(char::is_ascii_digit).count();
        if matches!(input.chars().nth(digits), Some('.') | Some(')')) {
            let (head, tail) = escaped.split_at(digits);
            return format!("{head}\\{tail}");
        }
    }

    escaped
}

/// Escape the contents of a Typst string literal.
pub fn string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            other => out.push(other),
        }
    }
    out
}

/// A complete, quoted Typst string literal.
pub fn string_literal(input: &str) -> String {
    format!("\"{}\"", string(input))
}
