//! The embedded Typst compiler.
//!
//! Compiles in-process using Typst's own library rather than invoking the
//! `typst` executable. That removes the external dependency entirely — a
//! `maddy` binary is self-contained — and removes the dominant cost of the
//! subprocess backend, which re-enumerates every font on the machine on each
//! invocation.
//!
//! Typst's compiler reaches the outside world through its [`World`] trait:
//! where the standard library lives, which fonts exist, and how to read a file.
//! [`MaddyWorld`] implements it over the generated source plus the directory
//! holding the author's Markdown.
//!
//! Only Typst's bundled fonts are available, by design. They are compiled into
//! the binary, so a document renders identically on every machine — the
//! determinism the specification asks for — and nothing is scanned at startup.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use typst::diag::{FileError, FileResult, SourceDiagnostic, Warned};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

use crate::diagnostics::{CompileError, Result};

use super::backend::{CompileContext, PdfBackend};

/// Compiles Typst in-process.
///
/// Fonts are parsed once for the lifetime of the program and shared by every
/// compilation, which is what makes repeated builds — watch mode especially —
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct EmbeddedTypstBackend;

impl EmbeddedTypstBackend {
    pub fn new() -> Self {
        Self
    }
}

impl PdfBackend for EmbeddedTypstBackend {
    fn compile(&self, typst_source: &str, context: &CompileContext) -> Result<Vec<u8>> {
        // Bundled fonts are the fast path. The machine's own fonts are scanned
        // only when the document asks for a family that is not bundled, or
        // contains a character the bundled faces cannot render.
        let fonts = if context.system_fonts || !bundled_fonts_cover(typst_source) {
            if context.verbose {
                eprintln!("Loading system fonts...");
            }
            system_fonts()
        } else {
            bundled_fonts()
        };

        let world = MaddyWorld::new(typst_source, &context.root, fonts);

        let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);

        if context.verbose && !warnings.is_empty() {
            eprint!("{}", format_diagnostics(&world, &warnings, "warning"));
        }

        let document = output.map_err(|errors| CompileError::TypstCompile {
            details: format_diagnostics(&world, &errors, "error"),
        })?;

        let pdf =
            typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
                CompileError::TypstCompile {
                    details: format_diagnostics(&world, &errors, "error"),
                }
            })?;

        // Typst memoizes aggressively across compilations. Without eviction a
        // long watch session grows without bound.
        comemo::evict(DEPTH);

        Ok(pdf)
    }
}

/// How many compilations a memoized value survives unused.
const DEPTH: usize = 8;

/// The fonts Typst bundles, parsed once per process.
///
/// Seventeen faces, compiled into the binary, so a document renders identically
/// on every machine and nothing is scanned at startup.
fn bundled_fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::embedded());
        store
    })
}

/// The bundled fonts plus every font installed on this machine.
///
/// Scanning costs a few hundred milliseconds, so this is reached for only when
/// the document actually needs it.
fn system_fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::embedded());
        store.extend(typst_kit::fonts::system());
        store
    })
}

/// Whether the bundled fonts can render every character in `source`.
///
/// Typst renders an uncovered character as a blank box rather than failing, so
/// without this check a document containing, say, Chinese or an emoji would
/// quietly come out wrong. ASCII is covered by construction and is the
/// overwhelming majority of any source, so it is skipped before the lookup.
pub(crate) fn bundled_fonts_cover(source: &str) -> bool {
    let store = bundled_fonts();
    let mut seen = std::collections::HashSet::new();

    source
        .chars()
        .filter(|c| !c.is_ascii() && !c.is_whitespace())
        .filter(|c| seen.insert(*c))
        .all(|c| covered(store, c))
}

/// Whether any face in `store` has a glyph for `c`.
///
/// Read from the font metadata the book already holds, so no face is parsed.
fn covered(store: &FontStore, c: char) -> bool {
    let book = store.book();
    (0..)
        .map_while(|index| book.info(index))
        .any(|info| info.coverage.contains(c as u32))
}

/// The standard library, built once.
fn library() -> &'static LazyHash<Library> {
    static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIBRARY.get_or_init(|| LazyHash::new(<Library as LibraryExt>::default()))
}

/// Typst's view of a maddy compilation.
struct MaddyWorld {
    main: FileId,
    source: Source,
    /// Either the bundled faces or those plus the machine's own.
    fonts: &'static FontStore,
    /// The directory relative paths resolve against: where the Markdown lives.
    root: PathBuf,
    /// Files read from disk, such as images, cached per compilation.
    files: Mutex<HashMap<FileId, FileResult<Bytes>>>,
}

impl MaddyWorld {
    fn new(source: &str, root: &Path, fonts: &'static FontStore) -> Self {
        // The generated source has no file of its own; a virtual name is what
        // Typst's diagnostics will refer to.
        let vpath = VirtualPath::new("<generated>").expect("a valid virtual path");
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();
        Self {
            main,
            source: Source::new(main, source.to_string()),
            fonts,
            root: root.to_path_buf(),
            files: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve a file id to a path inside the root.
    ///
    /// Typst normalizes a virtual path before this point, so `..` cannot climb
    /// out of the root. Package imports are refused: v0.1 does not touch the
    /// network.
    fn locate(&self, id: FileId) -> FileResult<PathBuf> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(FileError::Other(Some(
                "packages are not supported; maddy does not access the network".into(),
            )));
        }
        id.vpath()
            .realize(&self.root)
            .map_err(|_| FileError::Other(Some("the path leaves the project root".into())))
    }

    fn read(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(cached) = self.files.lock().expect("the file cache").get(&id) {
            return cached.clone();
        }

        let result = self.locate(id).and_then(|path| {
            std::fs::read(&path)
                .map(Bytes::new)
                .map_err(|error| FileError::from_io(error, &path))
        });

        self.files
            .lock()
            .expect("the file cache")
            .insert(id, result.clone());
        result
    }
}

impl World for MaddyWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            return Ok(self.source.clone());
        }
        let bytes = self.read(id)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_string()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.read(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;
        // `decompose` yields whole weeks, days, hours, minutes and seconds.
        let shift = offset.map_or(0, |offset| {
            let [weeks, days, hours, minutes, seconds] = offset.decompose();
            ((weeks * 7 + days) * 24 + hours) * 3600 + minutes * 60 + seconds
        });
        let shifted = now + shift;
        let (year, month, day) = civil_from_days(shifted.div_euclid(86_400));
        Datetime::from_ymd(year, month, day)
    }
}

/// Convert a count of days since the Unix epoch into a civil date.
///
/// Howard Hinnant's algorithm, which is exact for the proleptic Gregorian
/// calendar. Written out rather than taken from a dependency: it is the only
/// date arithmetic in the program.
fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u8;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u8;
    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month, day)
}

/// Render Typst diagnostics the way its command line does, so that the rest of
/// the compiler can treat both backends identically.
fn format_diagnostics(
    world: &MaddyWorld,
    diagnostics: &[SourceDiagnostic],
    severity: &str,
) -> String {
    use typst::WorldExt;

    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&format!("{severity}: {}\n", diagnostic.message));

        // Point at the generated source, which `--keep-typst` lets the user read.
        if let Some(range) = world.range(diagnostic.span) {
            let line = world
                .source
                .lines()
                .byte_to_line(range.start)
                .map_or(0, |l| l + 1);
            let column = world
                .source
                .lines()
                .byte_to_column(range.start)
                .map_or(0, |c| c + 1);
            out.push_str(&format!("  --> <generated>:{line}:{column}\n"));
        }

        for hint in &diagnostic.hints {
            out.push_str(&format!("  = hint: {}\n", hint.v));
        }
    }
    out
}
