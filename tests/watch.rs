//! Watch mode, driven as a real process.
//!
//! The loop never exits on its own, so these tests start the binary, change a
//! file, wait for the output to be rewritten, and then kill it. They assert on
//! the files produced rather than on log output, because a killed process can
//! lose buffered stdout.

mod support;

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use assert_cmd::prelude::*;

/// How long to let a rebuild happen before giving up.
const TIMEOUT: Duration = Duration::from_secs(20);

/// A child process that is killed when it goes out of scope, so a failing
/// assertion cannot leave a watcher running.
struct Watching(Child);

impl Drop for Watching {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_watching(input: &Path) -> Watching {
    let child = Command::cargo_bin("mdpdf")
        .expect("the mdpdf binary should be built")
        .arg("--watch")
        .arg(input)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("starting the watcher");
    Watching(child)
}

/// Wait until `path` exists and was modified after `since`.
fn wait_for_rebuild(path: &Path, since: SystemTime) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if let Ok(modified) = std::fs::metadata(path).and_then(|m| m.modified()) {
            if modified > since {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Wait until `path` exists at all.
fn wait_for(path: &Path) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn editing_the_document_rebuilds_it() {
    if !support::typst_available() {
        support::skip("editing_the_document_rebuilds_it");
        return;
    }

    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "paper.md", "# One\n\nText.\n");
    let output = directory.path().join("paper.typ");

    // Emitting Typst keeps the test independent of how long a PDF takes.
    let _watcher = {
        let child = Command::cargo_bin("mdpdf")
            .expect("the binary")
            .arg("--watch")
            .args(["--emit", "typst"])
            .arg(&input)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("starting the watcher");
        Watching(child)
    };

    assert!(wait_for(&output), "the first build should produce output");
    let first = std::fs::metadata(&output).unwrap().modified().unwrap();
    assert!(std::fs::read_to_string(&output).unwrap().contains("= One"));

    // Sleep past any filesystem timestamp granularity before editing.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&input, "# Two\n\nChanged.\n").expect("editing the document");

    assert!(
        wait_for_rebuild(&output, first),
        "the edit should trigger a rebuild"
    );
    assert!(std::fs::read_to_string(&output).unwrap().contains("= Two"));
}

#[test]
fn a_failed_build_does_not_stop_the_watcher() {
    if !support::typst_available() {
        support::skip("a_failed_build_does_not_stop_the_watcher");
        return;
    }

    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "paper.md", "# One\n\nText.\n");
    let output = directory.path().join("paper.pdf");

    let _watcher = start_watching(&input);
    assert!(wait_for(&output), "the first build should produce a PDF");

    // A document that cannot compile.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&input, "# Two\n\nBad $\\foo$ math.\n").expect("breaking the document");
    std::thread::sleep(Duration::from_millis(1500));

    // Recovering from the mistake must rebuild, without restarting the watcher.
    let before = std::fs::metadata(&output).unwrap().modified().unwrap();
    std::fs::write(&input, "# Three\n\nFixed.\n").expect("fixing the document");

    assert!(
        wait_for_rebuild(&output, before),
        "the watcher should survive a failed build"
    );
}

#[test]
fn editing_the_configuration_rebuilds_the_document() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let input = support::write_markdown(directory.path(), "paper.md", "# One\n\nText.\n");
    let config = directory.path().join("mdpdf.toml");
    std::fs::write(&config, "[page]\nmargin = \"25mm\"\n").expect("writing config");
    let output = directory.path().join("paper.typ");

    let _watcher = {
        let child = Command::cargo_bin("mdpdf")
            .expect("the binary")
            .arg("--watch")
            .args(["--emit", "typst"])
            .arg(&input)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("starting the watcher");
        Watching(child)
    };

    assert!(wait_for(&output), "the first build should produce output");
    assert!(std::fs::read_to_string(&output)
        .unwrap()
        .contains("margin: 25mm"));
    let first = std::fs::metadata(&output).unwrap().modified().unwrap();

    // The document itself is untouched; only its configuration changes.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&config, "[page]\nmargin = \"7mm\"\n").expect("editing config");

    assert!(
        wait_for_rebuild(&output, first),
        "a configuration edit should rebuild"
    );
    assert!(std::fs::read_to_string(&output)
        .unwrap()
        .contains("margin: 7mm"));
}
