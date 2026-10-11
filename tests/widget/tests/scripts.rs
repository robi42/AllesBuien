//! Runs the bar widget's shell proof and lints the shell behind it.
//!
//! `run.sh` prints one `ok` or `FAIL` line per case, so a failure here carries
//! its whole output, and the case that failed is named in it.

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
