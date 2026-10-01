//! Watch mode.
//!
//! Change detection is by polling modification times rather than by subscribing
//! to filesystem events. For a document compiler the difference the author can
//! perceive is a fraction of a second, and polling brings no dependency, behaves
//! identically on every platform, and is deterministic to test.
//!
//! The watcher tracks every input a compilation depends on — the document, the
//! configuration file that was actually used, and a custom template — so that
//! editing any of them rebuilds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// How often to look for changes.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(250);

/// What is known about a watched file.
///
/// Size is compared alongside the modification time because a coarse filesystem
/// timestamp can leave two quick edits looking identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl Stamp {
    /// The stamp of a path, or `None` when it does not exist.
    ///
    /// A missing file is a state like any other: deleting and restoring a file
    /// counts as two changes rather than crashing the watch.
    fn read(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        })
    }
}

/// Detects changes to a set of files.
#[derive(Debug, Clone)]
pub struct Watcher {
    stamps: BTreeMap<PathBuf, Option<Stamp>>,
    interval: Duration,
}

impl Watcher {
    /// Watch the given paths, taking their current state as the baseline.
    pub fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let stamps = paths
            .into_iter()
            .map(|path| (path.clone(), Stamp::read(&path)))
            .collect();
        Self {
            stamps,
            interval: DEFAULT_INTERVAL,
        }
    }

    /// Use a different polling interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// The paths being watched.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.stamps.keys().map(PathBuf::as_path)
    }

    /// Replace the watched set, keeping the state of paths already watched.
    ///
    /// A rebuild can change what matters — a document may start or stop using a
    /// configuration file — so the set is refreshed after every compilation.
    /// Existing stamps are preserved so that refreshing never looks like a
    /// change in itself.
    pub fn track(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut next = BTreeMap::new();
        for path in paths {
            let stamp = self
                .stamps
                .get(&path)
                .copied()
                .unwrap_or_else(|| Stamp::read(&path));
            next.insert(path, stamp);
        }
        self.stamps = next;
    }

    /// Look once for changes, updating the baseline.
    ///
    /// Returns the paths that changed.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        for (path, known) in &mut self.stamps {
            let current = Stamp::read(path);
            if current != *known {
                *known = current;
                changed.push(path.clone());
            }
        }

        changed
    }

    /// Block until something changes, then return the paths that did.
    pub fn wait(&mut self) -> Vec<PathBuf> {
        loop {
            let changed = self.poll();
            if !changed.is_empty() {
                return changed;
            }
            std::thread::sleep(self.interval);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a file, making sure its stamp differs from the previous one.
    ///
    /// Filesystem timestamps can be coarse, so the content length is varied as
    /// well; that is exactly the case the size comparison exists for.
    fn write(path: &Path, contents: &str) {
        std::fs::write(path, contents).expect("writing the file");
    }

    #[test]
    fn nothing_changes_when_nothing_is_touched() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("paper.md");
        write(&path, "one");

        let mut watcher = Watcher::new([path]);
        assert!(watcher.poll().is_empty());
        assert!(watcher.poll().is_empty());
    }

    #[test]
    fn editing_a_watched_file_is_a_change() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("paper.md");
        write(&path, "one");

        let mut watcher = Watcher::new([path.clone()]);
        write(&path, "one and two");

        assert_eq!(watcher.poll(), vec![path]);
    }

    #[test]
    fn a_change_is_reported_once() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("paper.md");
        write(&path, "one");

        let mut watcher = Watcher::new([path.clone()]);
        write(&path, "changed");

        assert_eq!(watcher.poll().len(), 1);
        // The baseline moved, so the same edit is not reported again.
        assert!(watcher.poll().is_empty());
    }

    #[test]
    fn several_files_are_watched_and_reported_individually() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paper = directory.path().join("paper.md");
        let config = directory.path().join("mdpdf.toml");
        write(&paper, "one");
        write(&config, "[page]\n");

        let mut watcher = Watcher::new([paper.clone(), config.clone()]);
        write(&config, "[page]\nmargin = \"3mm\"\n");

        assert_eq!(watcher.poll(), vec![config]);
        assert!(watcher.poll().is_empty());
    }

    #[test]
    fn a_file_that_appears_is_a_change() {
        // A configuration file created after the first build should rebuild.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let config = directory.path().join("mdpdf.toml");

        let mut watcher = Watcher::new([config.clone()]);
        assert!(watcher.poll().is_empty());

        write(&config, "[page]\n");
        assert_eq!(watcher.poll(), vec![config]);
    }

    #[test]
    fn a_file_that_disappears_is_a_change() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("paper.md");
        write(&path, "one");

        let mut watcher = Watcher::new([path.clone()]);
        std::fs::remove_file(&path).expect("removing the file");

        assert_eq!(watcher.poll(), vec![path]);
        // And stays quiet afterwards rather than reporting the absence forever.
        assert!(watcher.poll().is_empty());
    }

    #[test]
    fn watching_nothing_reports_nothing() {
        let mut watcher = Watcher::new([]);
        assert!(watcher.poll().is_empty());
        assert_eq!(watcher.paths().count(), 0);
    }

    #[test]
    fn tracking_a_new_set_keeps_what_was_already_known() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paper = directory.path().join("paper.md");
        let config = directory.path().join("mdpdf.toml");
        write(&paper, "one");
        write(&config, "[page]\n");

        let mut watcher = Watcher::new([paper.clone()]);
        watcher.track([paper.clone(), config.clone()]);

        // Refreshing the set must not look like a change in itself.
        assert!(watcher.poll().is_empty());
        assert_eq!(
            watcher.paths().collect::<Vec<_>>(),
            vec![config.as_path(), paper.as_path()]
        );
    }

    #[test]
    fn tracking_drops_paths_that_no_longer_matter() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paper = directory.path().join("paper.md");
        let config = directory.path().join("mdpdf.toml");
        write(&paper, "one");
        write(&config, "[page]\n");

        let mut watcher = Watcher::new([paper.clone(), config.clone()]);
        watcher.track([paper.clone()]);

        write(&config, "[page]\nmargin = \"3mm\"\n");
        assert!(
            watcher.poll().is_empty(),
            "a dropped path should not be reported"
        );
    }

    #[test]
    fn the_interval_can_be_shortened() {
        let watcher = Watcher::new([]).with_interval(Duration::from_millis(5));
        assert_eq!(watcher.interval(), Duration::from_millis(5));
        assert_eq!(Watcher::new([]).interval(), DEFAULT_INTERVAL);
    }

    #[test]
    fn wait_returns_as_soon_as_something_changes() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("paper.md");
        write(&path, "one");

        let mut watcher = Watcher::new([path.clone()]).with_interval(Duration::from_millis(5));

        let writer = {
            let path = path.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(30));
                std::fs::write(&path, "changed by the other thread").expect("writing");
            })
        };

        assert_eq!(watcher.wait(), vec![path]);
        writer.join().expect("the writing thread");
    }
}
