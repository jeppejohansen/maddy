//! Document metadata, as supplied by front matter.
//!
//! This module holds only the *data*. Extracting it from a source file is the
//! job of [`crate::markdown::frontmatter`], and resolving it against
//! configuration and CLI arguments is the job of [`crate::config`].

use std::fmt;
use std::str::FromStr;

/// The two output modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentType {
    /// A typeset document. The default.
    #[default]
    Document,
    /// A widescreen presentation.
    Slides,
}

impl DocumentType {
    pub fn is_slides(self) -> bool {
        matches!(self, DocumentType::Slides)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DocumentType::Document => "document",
            DocumentType::Slides => "slides",
        }
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DocumentType {
    type Err = UnknownDocumentType;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "document" | "doc" | "article" => Ok(DocumentType::Document),
            "slides" | "slide" | "presentation" | "deck" => Ok(DocumentType::Slides),
            other => Err(UnknownDocumentType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown document type `{0}`, expected `document` or `slides`")]
pub struct UnknownDocumentType(pub String);

/// Metadata describing a document.
///
/// Every field is optional except the document type, which has a default. A
/// document with no front matter at all is represented by
/// [`Metadata::default`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub document_type: DocumentType,
    pub style: Option<String>,
}

impl Metadata {
    /// Whether a title slide should be generated.
    ///
    /// A title is the trigger: the remaining fields merely fill the slide in.
    pub fn has_title_page(&self) -> bool {
        self.title.as_ref().is_some_and(|t| !t.trim().is_empty())
    }

    /// Whether any field other than the document type was supplied.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.subtitle.is_none()
            && self.author.is_none()
            && self.date.is_none()
            && self.style.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_is_the_default_type() {
        assert_eq!(DocumentType::default(), DocumentType::Document);
        assert!(!DocumentType::default().is_slides());
        assert!(DocumentType::Slides.is_slides());
    }

    #[test]
    fn document_type_parses_its_canonical_spellings() {
        assert_eq!("document".parse(), Ok(DocumentType::Document));
        assert_eq!("slides".parse(), Ok(DocumentType::Slides));
    }

    #[test]
    fn document_type_parsing_is_case_and_whitespace_insensitive() {
        assert_eq!("  Slides \n".parse(), Ok(DocumentType::Slides));
        assert_eq!("DOCUMENT".parse(), Ok(DocumentType::Document));
    }

    #[test]
    fn document_type_accepts_common_synonyms() {
        for alias in ["slide", "presentation", "deck"] {
            assert_eq!(alias.parse(), Ok(DocumentType::Slides), "{alias}");
        }
        for alias in ["doc", "article"] {
            assert_eq!(alias.parse(), Ok(DocumentType::Document), "{alias}");
        }
    }

    #[test]
    fn an_unknown_document_type_names_the_valid_options() {
        let error = "poster".parse::<DocumentType>().unwrap_err();
        assert_eq!(
            error.to_string(),
            "unknown document type `poster`, expected `document` or `slides`"
        );
    }

    #[test]
    fn document_type_displays_as_its_front_matter_spelling() {
        assert_eq!(DocumentType::Document.to_string(), "document");
        assert_eq!(DocumentType::Slides.to_string(), "slides");
    }

    #[test]
    fn default_metadata_is_an_untitled_document() {
        let metadata = Metadata::default();
        assert!(metadata.is_empty());
        assert!(!metadata.has_title_page());
        assert_eq!(metadata.document_type, DocumentType::Document);
    }

    #[test]
    fn a_title_requests_a_title_page() {
        let metadata = Metadata {
            title: Some("Peer Effects".into()),
            ..Metadata::default()
        };
        assert!(metadata.has_title_page());
        assert!(!metadata.is_empty());
    }

    #[test]
    fn a_blank_title_does_not_request_a_title_page() {
        let metadata = Metadata {
            title: Some("   ".into()),
            ..Metadata::default()
        };
        assert!(!metadata.has_title_page());
    }

    #[test]
    fn metadata_is_not_empty_when_any_field_is_set() {
        let fields: [fn(&mut Metadata); 4] = [
            |m| m.subtitle = Some("s".into()),
            |m| m.author = Some("a".into()),
            |m| m.date = Some("d".into()),
            |m| m.style = Some("dark".into()),
        ];
        for set in fields {
            let mut metadata = Metadata::default();
            set(&mut metadata);
            assert!(!metadata.is_empty());
        }
    }

    #[test]
    fn the_document_type_alone_does_not_make_metadata_non_empty() {
        let metadata = Metadata {
            document_type: DocumentType::Slides,
            ..Metadata::default()
        };
        assert!(metadata.is_empty());
    }
}
