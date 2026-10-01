//! Project configuration.
//!
//! Configuration is deliberately small. It covers the handful of things an
//! author changes for a whole project — paper, fonts, margins, heading
//! numbering, slide style — and stops there. Users who want complete visual
//! control use a custom template, which is why `--template` exists.
//!
//! Settings reach the renderer as Typst `set` rules emitted at the top of the
//! document body. Because a template's own rules run before the body, a rule
//! here overrides it. That means configuration needs no change to the theme
//! contract and works just as well with a custom template as with a built-in
//! one.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::diagnostics::{io_error, CompileError, Diagnostic, Result};
use crate::typst::escape;

/// The configuration file's name, looked for beside the document.
pub const FILE_NAME: &str = "maddy.toml";

/// Project configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    #[serde(default)]
    pub document: DocumentConfig,
    #[serde(default)]
    pub page: PageConfig,
    #[serde(default)]
    pub headings: HeadingsConfig,
    #[serde(default)]
    pub code: CodeConfig,
    #[serde(default)]
    pub slides: SlidesConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DocumentConfig {
    /// A Typst paper name, such as `a4` or `us-letter`.
    pub paper: Option<String>,
    /// The body font family.
    pub font: Option<String>,
    /// The body font size, as a Typst length.
    pub font_size: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PageConfig {
    /// The page margin, as a Typst length.
    pub margin: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct HeadingsConfig {
    /// Whether headings are numbered.
    pub numbered: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CodeConfig {
    /// The font family for code.
    pub font: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SlidesConfig {
    /// The default slide style for this project.
    ///
    /// Sits between the built-in default and the document's own front matter in
    /// the precedence chain.
    pub style: Option<String>,
}

impl Config {
    /// Parse configuration from TOML.
    pub fn parse(source: &str) -> Result<Self> {
        let config: Config = toml::from_str(source)
            .map_err(|error| CompileError::Config(describe(source, &error)))?;
        config.validate()?;
        Ok(config)
    }

    /// Read configuration from a file.
    pub fn read(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path).map_err(io_error(format!(
            "reading configuration {}",
            path.display()
        )))?;
        Self::parse(&source).map_err(|error| match error {
            CompileError::Config(message) => {
                CompileError::Config(format!("{}: {message}", path.display()))
            }
            other => other,
        })
    }

    /// Find and read configuration for a document.
    ///
    /// An explicit path must exist. Otherwise `maddy.toml` is looked for beside
    /// the document and in each parent directory, so a project can carry one
    /// file at its root; finding none is not an error.
    pub fn discover(input: &Path, explicit: Option<&Path>) -> Result<(Self, Option<PathBuf>)> {
        if let Some(path) = explicit {
            return Ok((Self::read(path)?, Some(path.to_path_buf())));
        }

        let start = input.parent().filter(|path| !path.as_os_str().is_empty());
        let start = start.unwrap_or(Path::new("."));

        for directory in start.ancestors() {
            let candidate = directory.join(FILE_NAME);
            if candidate.is_file() {
                return Ok((Self::read(&candidate)?, Some(candidate)));
            }
        }

        Ok((Self::default(), None))
    }

    /// Reject values that are not well-formed, with a message naming the key.
    ///
    /// Values are interpolated into generated Typst, so this is also what keeps
    /// a malformed length from becoming syntax.
    fn validate(&self) -> Result<()> {
        if let Some(paper) = &self.document.paper {
            check(
                paper.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "document.paper",
                paper,
            )?;
        }
        if let Some(size) = &self.document.font_size {
            check(is_length(size), "document.font-size", size)?;
        }
        if let Some(margin) = &self.page.margin {
            check(is_length(margin), "page.margin", margin)?;
        }
        Ok(())
    }

    /// Whether anything at all was configured.
    pub fn is_empty(&self) -> bool {
        self == &Config::default()
    }

    /// The Typst `set` rules this configuration implies.
    ///
    /// Emitted at the top of the document body, after the template has applied
    /// its own rules, so these win.
    pub fn prelude(&self) -> String {
        let mut rules = Vec::new();

        let mut page = Vec::new();
        if let Some(paper) = &self.document.paper {
            page.push(format!("paper: {}", escape::string_literal(paper)));
        }
        if let Some(margin) = &self.page.margin {
            page.push(format!("margin: {margin}"));
        }
        if !page.is_empty() {
            rules.push(format!("#set page({})", page.join(", ")));
        }

        let mut text = Vec::new();
        if let Some(font) = &self.document.font {
            text.push(format!("font: {}", escape::string_literal(font)));
        }
        if let Some(size) = &self.document.font_size {
            text.push(format!("size: {size}"));
        }
        if !text.is_empty() {
            rules.push(format!("#set text({})", text.join(", ")));
        }

        if let Some(numbered) = self.headings.numbered {
            let numbering = if numbered { "\"1.1\"" } else { "none" };
            rules.push(format!("#set heading(numbering: {numbering})"));
        }

        if let Some(font) = &self.code.font {
            // A show rule, because `raw` takes its font from a text rule scoped
            // to it rather than from a parameter.
            rules.push(format!(
                "#show raw: set text(font: {})",
                escape::string_literal(font)
            ));
        }

        if rules.is_empty() {
            return String::new();
        }

        format!("// From {FILE_NAME}\n{}\n", rules.join("\n"))
    }
}

/// Reject a malformed value, naming the key and what was wrong with it.
fn check(ok: bool, key: &str, value: &str) -> Result<()> {
    if ok {
        return Ok(());
    }
    Err(CompileError::Config(format!(
        "`{key}` has an invalid value `{value}`"
    )))
}

/// Whether a string is a Typst length, such as `11pt` or `2.5cm`.
fn is_length(value: &str) -> bool {
    let value = value.trim();
    let Some(digits) = value.find(|c: char| c.is_ascii_alphabetic() || c == '%') else {
        return false;
    };
    let (number, unit) = value.split_at(digits);

    number.parse::<f64>().is_ok_and(|number| number >= 0.0)
        && matches!(unit, "pt" | "mm" | "cm" | "in" | "em" | "%")
}

/// Describe a TOML failure in one line.
///
/// The error's `Display` leads with a location header and then draws a span
/// diagram, which renders badly inside a diagnostic and buries the sentence that
/// actually says what is wrong. The message is taken directly instead, with the
/// line number appended.
fn describe(source: &str, error: &toml::de::Error) -> String {
    let message = error.message().trim().replace('\n', " ");
    match error.span() {
        Some(span) => {
            let line = crate::diagnostics::line_column(source, span.start).line;
            format!("{message} (line {line})")
        }
        None => message,
    }
}

/// A warning naming where configuration came from, for verbose reporting.
pub fn loaded_from(path: &Path) -> Diagnostic {
    Diagnostic::note(format!("using configuration from {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example from the specification.
    const EXAMPLE: &str = r#"
[document]
paper = "a4"
font = "Libertinus Serif"
font-size = "11pt"

[page]
margin = "25mm"

[headings]
numbered = false

[code]
font = "DejaVu Sans Mono"
"#;

    fn parse(source: &str) -> Config {
        Config::parse(source).unwrap_or_else(|error| panic!("parsing failed: {error}"))
    }

    fn failure(source: &str) -> String {
        match Config::parse(source) {
            Ok(config) => panic!("expected a failure, got {config:?}"),
            Err(error) => error.to_string(),
        }
    }

    // ------------------------------------------------------------- parsing

    #[test]
    fn the_specifications_example_parses() {
        let config = parse(EXAMPLE);

        assert_eq!(config.document.paper.as_deref(), Some("a4"));
        assert_eq!(config.document.font.as_deref(), Some("Libertinus Serif"));
        assert_eq!(config.document.font_size.as_deref(), Some("11pt"));
        assert_eq!(config.page.margin.as_deref(), Some("25mm"));
        assert_eq!(config.headings.numbered, Some(false));
        assert_eq!(config.code.font.as_deref(), Some("DejaVu Sans Mono"));
    }

    #[test]
    fn an_empty_file_is_the_default_configuration() {
        assert_eq!(parse(""), Config::default());
        assert!(parse("").is_empty());
        assert!(!parse(EXAMPLE).is_empty());
    }

    #[test]
    fn sections_are_independent() {
        let config = parse("[page]\nmargin = \"30mm\"\n");
        assert_eq!(config.page.margin.as_deref(), Some("30mm"));
        assert_eq!(config.document.paper, None);
    }

    #[test]
    fn a_project_can_set_the_slide_style() {
        assert_eq!(
            parse("[slides]\nstyle = \"dark\"\n")
                .slides
                .style
                .as_deref(),
            Some("dark")
        );
    }

    // -------------------------------------------------------------- errors

    #[test]
    fn an_unknown_key_is_rejected_rather_than_ignored() {
        // Silently ignoring a misspelled key would leave the author wondering
        // why nothing changed.
        let message = failure("[document]\npapers = \"a4\"\n");
        assert!(message.contains("papers"), "{message}");
    }

    #[test]
    fn an_unknown_section_is_rejected() {
        assert!(failure("[typography]\nfont = \"x\"\n").contains("typography"));
    }

    #[test]
    fn malformed_toml_reports_only_the_first_line() {
        // The rest of a TOML error is a span diagram that renders badly inside a
        // diagnostic.
        let message = failure("[document\n");
        assert_eq!(message.lines().count(), 1, "{message}");
    }

    #[test]
    fn a_wrong_type_is_rejected() {
        let message = failure("[headings]\nnumbered = \"yes\"\n");
        assert!(message.contains("boolean"), "{message}");
        assert!(message.contains("line 2"), "{message}");
    }

    #[test]
    fn an_invalid_length_is_rejected_naming_the_key() {
        let message = failure("[page]\nmargin = \"25 furlongs\"\n");
        assert!(message.contains("page.margin"), "{message}");
        assert!(message.contains("25 furlongs"), "{message}");
    }

    #[test]
    fn a_length_that_would_inject_typst_is_rejected() {
        // Values are interpolated into generated Typst, so a malformed one must
        // not be able to become syntax.
        assert!(
            failure("[document]\nfont-size = \"11pt); #panic(\\\"x\\\"); (\"\n")
                .contains("document.font-size")
        );
    }

    #[test]
    fn an_invalid_paper_name_is_rejected() {
        assert!(failure("[document]\npaper = \"a4\\\"); #x; (\"\n").contains("document.paper"));
    }

    #[test]
    fn lengths_accept_the_usual_units() {
        for value in ["11pt", "2.5cm", "25mm", "1in", "1.2em", "80%", "0pt"] {
            assert!(is_length(value), "{value} should be a length");
        }
        for value in ["11", "pt", "", "-3mm", "11 pt ohno", "11px"] {
            assert!(!is_length(value), "{value} should not be a length");
        }
    }

    #[test]
    fn a_font_name_may_contain_anything_because_it_becomes_a_string() {
        let config = parse("[document]\nfont = \"A \\\"quoted\\\" Font\"\n");
        assert!(
            config.prelude().contains(r#"font: "A \"quoted\" Font""#),
            "{}",
            config.prelude()
        );
    }

    // ------------------------------------------------------------- prelude

    #[test]
    fn an_empty_configuration_emits_no_rules() {
        assert_eq!(Config::default().prelude(), "");
    }

    #[test]
    fn the_prelude_groups_related_settings_into_one_rule_each() {
        let prelude = parse(EXAMPLE).prelude();

        assert!(
            prelude.contains(r#"#set page(paper: "a4", margin: 25mm)"#),
            "{prelude}"
        );
        assert!(
            prelude.contains(r#"#set text(font: "Libertinus Serif", size: 11pt)"#),
            "{prelude}"
        );
        assert!(
            prelude.contains("#set heading(numbering: none)"),
            "{prelude}"
        );
        assert!(
            prelude.contains(r#"#show raw: set text(font: "DejaVu Sans Mono")"#),
            "{prelude}"
        );
    }

    #[test]
    fn the_prelude_says_where_it_came_from() {
        assert!(parse(EXAMPLE).prelude().starts_with("// From maddy.toml\n"));
    }

    #[test]
    fn numbered_headings_get_a_numbering_pattern() {
        assert!(parse("[headings]\nnumbered = true\n")
            .prelude()
            .contains(r#"#set heading(numbering: "1.1")"#));
    }

    #[test]
    fn only_configured_settings_appear() {
        let prelude = parse("[page]\nmargin = \"30mm\"\n").prelude();
        assert!(prelude.contains("#set page(margin: 30mm)"), "{prelude}");
        assert!(!prelude.contains("#set text"), "{prelude}");
        assert!(!prelude.contains("#set heading"), "{prelude}");
    }

    #[test]
    fn the_slide_style_is_not_a_typst_rule() {
        // It selects a template; it is not something Typst can be told.
        assert_eq!(parse("[slides]\nstyle = \"dark\"\n").prelude(), "");
    }

    // ------------------------------------------------------------ discovery

    #[test]
    fn a_document_with_no_configuration_nearby_uses_defaults() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let input = directory.path().join("paper.md");

        let (config, path) = Config::discover(&input, None).expect("discovery");
        assert_eq!(config, Config::default());
        assert_eq!(path, None);
    }

    #[test]
    fn configuration_beside_the_document_is_found() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join(FILE_NAME),
            "[page]\nmargin = \"30mm\"\n",
        )
        .expect("writing config");
        let input = directory.path().join("paper.md");

        let (config, path) = Config::discover(&input, None).expect("discovery");
        assert_eq!(config.page.margin.as_deref(), Some("30mm"));
        assert_eq!(path, Some(directory.path().join(FILE_NAME)));
    }

    #[test]
    fn configuration_at_a_project_root_is_found_from_a_subdirectory() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join(FILE_NAME),
            "[page]\nmargin = \"30mm\"\n",
        )
        .expect("writing config");
        let nested = directory.path().join("chapters/one");
        std::fs::create_dir_all(&nested).expect("creating directories");

        let (config, _) = Config::discover(&nested.join("paper.md"), None).expect("discovery");
        assert_eq!(config.page.margin.as_deref(), Some("30mm"));
    }

    #[test]
    fn the_nearest_configuration_wins() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join(FILE_NAME),
            "[page]\nmargin = \"10mm\"\n",
        )
        .expect("writing the root config");
        let nested = directory.path().join("chapters");
        std::fs::create_dir_all(&nested).expect("creating directories");
        std::fs::write(nested.join(FILE_NAME), "[page]\nmargin = \"30mm\"\n")
            .expect("writing the nearer config");

        let (config, _) = Config::discover(&nested.join("paper.md"), None).expect("discovery");
        assert_eq!(config.page.margin.as_deref(), Some("30mm"));
    }

    #[test]
    fn an_explicit_path_is_used_instead_of_discovery() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            directory.path().join(FILE_NAME),
            "[page]\nmargin = \"10mm\"\n",
        )
        .expect("writing the discovered config");
        let explicit = directory.path().join("other.toml");
        std::fs::write(&explicit, "[page]\nmargin = \"99mm\"\n")
            .expect("writing the explicit config");

        let (config, path) = Config::discover(&directory.path().join("paper.md"), Some(&explicit))
            .expect("discovery");
        assert_eq!(config.page.margin.as_deref(), Some("99mm"));
        assert_eq!(path, Some(explicit));
    }

    #[test]
    fn an_explicit_path_that_does_not_exist_is_an_error() {
        let error = Config::discover(Path::new("paper.md"), Some(Path::new("missing.toml")))
            .expect_err("a missing explicit config should fail");
        assert_eq!(error.exit_code(), 3);
    }

    #[test]
    fn a_broken_configuration_file_names_itself() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join(FILE_NAME);
        std::fs::write(&path, "[document]\npapers = 1\n").expect("writing config");

        let error = Config::discover(&directory.path().join("paper.md"), None)
            .expect_err("a broken config should fail");
        assert!(error.to_string().contains(FILE_NAME), "{error}");
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn a_loaded_note_names_the_file() {
        let note = loaded_from(Path::new("/p/maddy.toml"));
        assert!(note.message.contains("/p/maddy.toml"));
    }
}
