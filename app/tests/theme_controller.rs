use std::{
    fs::{self, FileTimes},
    path::Path,
    sync::Arc,
    time::{Duration, SystemTime},
};

use allesbuien::TimerIntervals;
use allesbuien_harness::HarnessBuilder;
use allesbuien_testkit::StubFeedApi;

struct ThemeFixture<'a> {
    name: &'a str,
    mode: &'a str,
    background: &'a str,
    accent: &'a str,
    modified: Duration,
}

fn write_theme(path: &Path, fixture: &ThemeFixture<'_>) {
    fs::create_dir(path).expect("theme directory should be created");
    let colors_path = path.join("colors.toml");
    fs::write(
        &colors_path,
        format!(
            "mode = \"{}\"\nbackground = \"{}\"\nforeground = \"#d8dee9\"\naccent = \"{}\"\nselection = \"#434c5e\"\nlighter_background = \"#3b4252\"\nmuted = \"#4c566a\"\n",
            fixture.mode, fixture.background, fixture.accent
        ),
    )
    .expect("theme fixture should be written");
    fs::File::options()
        .write(true)
        .open(colors_path)
        .expect("theme fixture should reopen")
        .set_times(FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + fixture.modified))
        .expect("theme fixture mtime should be set");
}

fn replace_theme(root: &Path, current: &Path, fixture: &ThemeFixture<'_>) {
    let next = root.join(fixture.name);
    write_theme(&next, fixture);
    fs::remove_dir_all(current).expect("current theme should be removed before replacement");
    fs::rename(next, current).expect("replacement theme should become current");
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "this one-process GUI regression keeps the ordered theme replacement, disappearance, recreation, and property assertions together"
)]
fn exposes_colors_across_replacements_and_unavailability() {
    // Given: one GUI application and an injected Omarchy theme directory.
    let root = tempfile::tempdir().expect("theme test root should be created");
    let current = root.path().join("current-theme");
    write_theme(
        &current,
        &ThemeFixture {
            name: "theme-a",
            mode: "dark",
            background: "#2e3440",
            accent: "#81a1c1",
            modified: Duration::from_secs(1),
        },
    );
    let intervals = TimerIntervals {
        theme_poll: Duration::from_millis(25),
        ..TimerIntervals::default()
    };

    // When: construction reads Nord, two rm+rename switches occur, then the path vanishes/returns.
    let mut booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .theme_path(current.clone())
        .intervals(intervals)
        .boot();
    let controller = booted.theme_controller().expect("controller should exist");
    assert!(*controller.theme_available());
    assert_eq!(controller.theme_mode().to_string(), "dark");
    assert_eq!(controller.theme_background().to_string(), "#2e3440");
    assert_eq!(controller.theme_foreground().to_string(), "#d8dee9");
    assert_eq!(controller.theme_accent().to_string(), "#81a1c1");
    assert_eq!(controller.theme_selection().to_string(), "#434c5e");
    assert_eq!(controller.theme_lighter_background().to_string(), "#3b4252");
    assert_eq!(controller.theme_muted().to_string(), "#4c566a");
    assert_eq!(*controller.poll_interval_ms(), 25);

    replace_theme(
        root.path(),
        &current,
        &ThemeFixture {
            name: "theme-b",
            mode: "dark",
            background: "#111111",
            accent: "#222222",
            modified: Duration::from_secs(2),
        },
    );
    booted.poll_theme();
    assert_eq!(
        booted
            .theme_controller()
            .expect("controller should exist")
            .theme_background()
            .to_string(),
        "#111111"
    );

    replace_theme(
        root.path(),
        &current,
        &ThemeFixture {
            name: "theme-c",
            mode: "dark",
            background: "#333333",
            accent: "#444444",
            modified: Duration::from_secs(3),
        },
    );
    booted.poll_theme();
    assert_eq!(
        booted
            .theme_controller()
            .expect("controller should exist")
            .theme_accent()
            .to_string(),
        "#444444"
    );

    fs::remove_dir_all(&current).expect("active theme should be removable");
    booted.poll_theme();
    assert!(
        !*booted
            .theme_controller()
            .expect("controller should exist")
            .theme_available()
    );

    write_theme(
        &current,
        &ThemeFixture {
            name: "theme-d",
            mode: "light",
            background: "#f0f0f0",
            accent: "#0066cc",
            modified: Duration::from_secs(4),
        },
    );
    booted.poll_theme();

    // Then: path re-resolution restores availability and exposes the recreated colors.
    let controller = booted.theme_controller().expect("controller should exist");
    assert!(*controller.theme_available());
    assert_eq!(controller.theme_mode().to_string(), "light");
    assert_eq!(controller.theme_background().to_string(), "#f0f0f0");
    assert_eq!(controller.theme_accent().to_string(), "#0066cc");
}
