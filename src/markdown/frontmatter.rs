//! The front-matter pre-pass.
//!
//! YAML front matter is recognized only at the very beginning of the file,
//! delimited by `---` lines. Removing it before the Markdown parser runs keeps
//! YAML handling out of the parser, and resolves the ambiguity between a YAML
//! delimiter and a slide separator: by the time slide segmentation sees a `---`,
//! any front matter is already gone.
//!
//! A file that opens with `---` but never closes the block is ordinary Markdown
//! — a thematic break — not malformed front matter.

use serde_yaml_ng::Value;

use crate::diagnostics::{CompileError, Diagnostic, Result, SourceSpan};
use crate::metadata::{DocumentType, Metadata};

/// A source file split into metadata and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatter<'a> {
    pub metadata: Metadata,
    /// The document body, with any front matter removed.
    pub body: &'a str,
    /// The byte offset of `body` within the original source.
    ///
    /// Spans produced while parsing the body are shifted by this amount so that
    /// every diagnostic points at the original file.
    pub body_offset: usize,
    /// Warnings raised while reading the metadata, such as unknown fields.
    pub diagnostics: Vec<Diagnostic>,
}

const DELIMITER: &str = "---";

/// Split `source` into metadata and body.
pub fn split(source: &str) -> Result<FrontMatter<'_>> {
    // A byte-order mark is invisible to the user, so it must not hide the
    // opening delimiter.
    let bom = if source.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };

    let Some(block) = locate(source, bom) else {
        return Ok(FrontMatter {
            metadata: Metadata::default(),
            body: source,
            body_offset: 0,
            diagnostics: Vec::new(),
        });
    };

    let yaml = &source[block.yaml.start..block.yaml.end];
    let (metadata, diagnostics) = parse_yaml(yaml, block.yaml.start)?;

    Ok(FrontMatter {
        metadata,
        body: &source[block.body_offset..],
        body_offset: block.body_offset,
        diagnostics,
    })
}

/// The byte ranges of a located front-matter block.
struct Block {
    /// The YAML text between the delimiters.
    yaml: SourceSpan,
    /// Where the document body starts.
    body_offset: usize,
}

/// Find a front-matter block, if the file opens with one.
fn locate(source: &str, bom: usize) -> Option<Block> {
    let rest = &source[bom..];
    let (first_line, after_first) = take_line(rest);
    if first_line.trim_end() != DELIMITER {
        return None;
    }

    let yaml_start = bom + after_first;
    let mut cursor = yaml_start;

    while cursor < source.len() {
        let (line, consumed) = take_line(&source[cursor..]);
        if line.trim_end() == DELIMITER {
            return Some(Block {
                yaml: SourceSpan::new(yaml_start, cursor),
                body_offset: cursor + consumed,
            });
        }
        cursor += consumed;
    }

    // Unterminated: treat the leading `---` as an ordinary thematic break.
    None
}

/// Split off the first line, returning it without its terminator plus the number
/// of bytes consumed including the terminator.
fn take_line(text: &str) -> (&str, usize) {
    match text.find('\n') {
        Some(index) => (&text[..index], index + 1),
        None => (text, text.len()),
    }
}

/// Parse the YAML block into metadata, collecting warnings for unknown fields.
///
/// `offset` is the position of the YAML text within the original source, used to
/// translate YAML error locations into document spans.
fn parse_yaml(yaml: &str, offset: usize) -> Result<(Metadata, Vec<Diagnostic>)> {
    let value: Value = serde_yaml_ng::from_str(yaml).map_err(|error| {
        let span = error.location().map(|location| {
            let start = location.index().min(yaml.len());
            SourceSpan::new(offset + start, offset + yaml.len())
        });
        CompileError::FrontMatter {
            message: error.to_string(),
            span,
        }
    })?;

    // A block holding only comments deserializes as null, which is simply empty.
    if value.is_null() {
        return Ok((Metadata::default(), Vec::new()));
    }

    let mapping = value
        .as_mapping()
        .ok_or_else(|| CompileError::FrontMatter {
            message: "expected a mapping of fields, such as `title: My Paper`".into(),
            span: Some(SourceSpan::new(offset, offset + yaml.len())),
        })?;

    let mut metadata = Metadata::default();
    let mut diagnostics = Vec::new();

    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            diagnostics.push(Diagnostic::warning(
                "ignoring a front-matter key that is not a name",
            ));
            continue;
        };
        let field = name.trim().to_ascii_lowercase();

        // An explicitly null value means the field was left blank; treat it as
        // absent rather than as the string "null".
        if value.is_null() {
            continue;
        }

        let text = scalar_to_string(value).ok_or_else(|| CompileError::FrontMatter {
            message: format!("field `{field}` must be a single value, not a list or mapping"),
            span: key_span(yaml, offset, name),
        })?;

        match field.as_str() {
            "title" => metadata.title = Some(text),
            "subtitle" => metadata.subtitle = Some(text),
            "author" => metadata.author = Some(text),
            "date" => metadata.date = Some(text),
            "style" => metadata.style = Some(text),
            "type" => {
                metadata.document_type =
                    text.parse::<DocumentType>()
                        .map_err(|error| CompileError::FrontMatter {
                            message: error.to_string(),
                            span: key_span(yaml, offset, name),
                        })?;
            }
            _ => {
                let mut warning =
                    Diagnostic::warning(format!("unknown front-matter field `{name}`"));
                warning.span = key_span(yaml, offset, name);
                diagnostics.push(warning);
            }
        }
    }

    Ok((metadata, diagnostics))
}

/// Render a scalar YAML value as text, or `None` for sequences and mappings.
fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Locate a key within the YAML text so a diagnostic can point at it.
///
/// Only keys at the start of a line are matched, so a value that happens to
/// contain the key name cannot be mistaken for the key itself.
fn key_span(yaml: &str, offset: usize, name: &str) -> Option<SourceSpan> {
    let mut cursor = 0;
    while cursor < yaml.len() {
        let (line, consumed) = take_line(&yaml[cursor..]);
        let indent = line.len() - line.trim_start().len();
        if line.trim_start().starts_with(name) {
            let start = offset + cursor + indent;
            return Some(SourceSpan::new(start, start + name.len()));
        }
        cursor += consumed;
    }
    None
}
