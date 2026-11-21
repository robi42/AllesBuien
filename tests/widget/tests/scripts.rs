//! Runs the bar widget's shell proof, lints the shell behind it, and runs the
//! check of the widget's probe schedule.
//!
//! `run.sh` prints one `ok` or `FAIL` line per case, and `qmltestrunner` one
//! `PASS` or `FAIL!` line, so a failure here carries its whole output, and the
//! case that failed is named in it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn report(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn widget_cases_pass() {
    let output = Command::new("bash")
        .arg("tests/widget/run.sh")
        .current_dir(repository_root())
        .output()
        .expect("bash should start");
    assert!(
        output.status.success(),
        "tests/widget/run.sh failed:\n{}",
        report(&output)
    );
}

#[test]
fn widget_scripts_pass_shellcheck() {
    let root = repository_root();
    let mut fakes: Vec<PathBuf> = fs::read_dir(root.join("tests/widget/fakes"))
        .expect("tests/widget/fakes should be readable")
        .map(|entry| entry.expect("a fake should be listable").path())
        .collect();
    fakes.sort();
    assert!(!fakes.is_empty(), "tests/widget/fakes holds no fakes");
    let output = Command::new("shellcheck")
        .args(["plugin/widget.sh", "tests/widget/run.sh"])
        .args(&fakes)
        .current_dir(&root)
        .output()
        .expect(
            "shellcheck is not installed. Its version is pinned in mise.toml; run: mise install",
        );
    assert!(
        output.status.success(),
        "shellcheck refused the widget's scripts:\n{}",
        report(&output)
    );
}

/// Where `qmltestrunner` is, looked for as `bin/test` looks for qmllint: Qt's
/// own directory first, where Arch's `qt6-declarative` puts both, then `PATH`.
fn qmltestrunner() -> PathBuf {
    let qt = PathBuf::from("/usr/lib/qt6/bin/qmltestrunner");
    if qt.is_file() {
        return qt;
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join("qmltestrunner"))
        .find(|candidate| candidate.is_file())
        .expect(
            "qmltestrunner is in neither /usr/lib/qt6/bin nor PATH. It comes with qt6-declarative; install that package",
        )
}

#[test]
fn widget_probe_schedule_passes() {
    let output = Command::new(qmltestrunner())
        .args([
            "-import",
            "tests/widget/qml/fakes",
            "-input",
            "tests/widget/qml/tst_schedule.qml",
        ])
        .env("QT_QPA_PLATFORM", "offscreen")
        .current_dir(repository_root())
        .output()
        .expect("qmltestrunner should start");
    assert!(
        output.status.success(),
        "tests/widget/qml/tst_schedule.qml failed:\n{}",
        report(&output)
    );
}
