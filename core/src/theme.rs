//! Omarchy theme discovery without application-layer color fallbacks.

use std::{fs, path::Path, time::SystemTime};

use serde::Deserialize;

const COLORS_FILE: &str = "colors.toml";

/// The subset of Omarchy colors consumed by the application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Theme {
    Available {
        mode: Option<String>,
        background: Option<String>,
        foreground: Option<String>,
        accent: Option<String>,
        selection: Option<String>,
        lighter_background: Option<String>,
        muted: Option<String>,
    },
    Unavailable,
}

#[derive(Deserialize)]
struct ThemeFile {
    mode: Option<String>,
    background: Option<String>,
    foreground: Option<String>,
    accent: Option<String>,
    selection: Option<String>,
    lighter_background: Option<String>,
    muted: Option<String>,
}

/// Returns the current `colors.toml` modification time with one metadata lookup.
///
/// The app polls this before calling [`read_theme`], so unchanged ticks do not parse the file.
#[must_use]
pub fn colors_mtime(theme_base: &Path) -> Option<SystemTime> {
    fs::metadata(theme_base.join(COLORS_FILE))
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Reads `colors.toml` by resolving it from `theme_base` on every call.
///
/// `omarchy-theme-set` removes the active theme directory and renames `next-theme` over it
/// (`/usr/share/omarchy/bin/omarchy-theme-set`, lines 291-293). Inode-bound watches therefore
/// die on a switch. The app must poll [`colors_mtime`] and re-read by path instead of retaining
/// a file handle or watch tied to the replaced directory.
#[must_use]
pub fn read_theme(theme_base: &Path) -> Theme {
    fs::read_to_string(theme_base.join(COLORS_FILE))
        .map_or(Theme::Unavailable, |contents| parse_theme(&contents))
}

fn parse_theme(contents: &str) -> Theme {
    toml::from_str::<ThemeFile>(contents).map_or(Theme::Unavailable, |file| Theme::Available {
        mode: file.mode,
        background: file.background,
        foreground: file.foreground,
        accent: file.accent,
        selection: file.selection,
        lighter_background: file.lighter_background,
        muted: file.muted,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{Theme, colors_mtime, parse_theme, read_theme};

    #[test]
    fn real_fixture_parses_when_unknown_omarchy_keys_are_present() {
        // Given
        let colors = include_str!("../tests/fixtures/colors-nord.toml");

        // When
        let theme = parse_theme(colors);

        // Then
        assert!(matches!(
            theme,
            Theme::Available {
                mode: Some(ref mode),
                accent: Some(ref accent),
                ..
            } if mode == "dark" && accent == "#81a1c1"
        ));
    }

    #[test]
    fn minimal_theme_parses_when_optional_colors_are_absent() {
        // Given
        let colors = "mode = \"dark\"\nbackground = \"#000000\"\n";

        // When
        let theme = parse_theme(colors);

        // Then
        assert_eq!(
            theme,
            Theme::Available {
                mode: Some("dark".to_owned()),
                background: Some("#000000".to_owned()),
                foreground: None,
                accent: None,
                selection: None,
                lighter_background: None,
                muted: None,
            }
        );
    }

    #[test]
    fn unavailable_is_returned_when_theme_directory_is_absent() {
        // Given
        let directory = tempdir().expect("temporary directory should be created");
        let absent_theme = directory.path().join("absent-theme");

        // When
        let theme = read_theme(&absent_theme);

        // Then
        assert_eq!(theme, Theme::Unavailable);
    }

    #[test]
    fn junk_line_never_panics_when_theme_is_read() {
        // Given
        let directory = tempdir().expect("temporary directory should be created");
        let theme_directory = directory.path().join("theme");
        fs::create_dir(&theme_directory).expect("theme directory should be created");
        fs::write(
            theme_directory.join("colors.toml"),
            "mode = \"dark\"\nthis is not toml\n",
        )
        .expect("junk fixture should be written");

        // When
        let theme = read_theme(&theme_directory);

        // Then
        assert_eq!(theme, Theme::Unavailable);
    }

    #[test]
    fn replacement_directory_is_resolved_again_when_theme_is_read() {
        // Given
        let directory = tempdir().expect("temporary directory should be created");
        let current_theme = directory.path().join("theme");
        let next_theme = directory.path().join("next-theme");
        fs::create_dir(&current_theme).expect("current theme directory should be created");
        fs::write(
            current_theme.join("colors.toml"),
            "mode = \"light\"\nbackground = \"#ffffff\"\n",
        )
        .expect("current theme fixture should be written");
        assert!(matches!(
            read_theme(&current_theme),
            Theme::Available {
                mode: Some(ref mode),
                ..
            } if mode == "light"
        ));
        fs::create_dir(&next_theme).expect("next theme directory should be created");
        fs::write(
            next_theme.join("colors.toml"),
            "mode = \"dark\"\nbackground = \"#000000\"\n",
        )
        .expect("next theme fixture should be written");

        // When
        fs::remove_dir_all(&current_theme).expect("current theme directory should be removed");
        fs::rename(&next_theme, &current_theme).expect("next theme should replace current theme");
        let theme = read_theme(&current_theme);

        // Then
        assert!(matches!(
            theme,
            Theme::Available {
                mode: Some(ref mode),
                background: Some(ref background),
                ..
            } if mode == "dark" && background == "#000000"
        ));
    }

    #[test]
    fn colors_mtime_is_exposed_when_colors_file_exists() {
        // Given
        let directory = tempdir().expect("temporary directory should be created");
        fs::write(directory.path().join("colors.toml"), "mode = \"dark\"\n")
            .expect("theme fixture should be written");

        // When
        let modified = colors_mtime(directory.path());

        // Then
        assert!(modified.is_some());
    }
}
