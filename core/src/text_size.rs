//! Omarchy's desktop-wide text size, read the way the Omarchy shell reads it.
//!
//! `omarchy-display-text-size` writes the size as `[font] base-size = N` into
//! `~/.config/omarchy/shell.toml`, in pixels, and `reset` deletes the line.
//! The shell treats 12 as its default and every other value relative to it.
//!
//! This is deliberately not a TOML parser. The shell's own reader,
//! `parseShell` in `/usr/share/omarchy/shell/Commons/Color.qml`, accepts lines
//! TOML refuses, such as bare role names (`active = red`) and bare width lists
//! (`border = 1 2`), and a theme or a person may well have written one of those
//! into another section of the same file. A strict parser would then lose the
//! text size over a line about colours. So the walk below follows `parseShell`
//! instead:
//!
//! - the file is decoded as Qt decodes it, a byte that is not UTF-8 becoming
//!   a replacement character rather than costing the whole file;
//! - lines are trimmed of whitespace as JavaScript's `\s` counts it, which
//!   takes in the byte-order mark Rust's own whitespace leaves out, and blank
//!   lines and `#` comments are skipped;
//! - `[name]` opens a section, optionally followed by a `#` comment;
//! - `key = value` counts when the value is quoted, a number, two to four
//!   numbers, or a bare word, each optionally followed by a `#` comment; any
//!   other line is skipped and changes nothing;
//! - the last `base-size` in `[font]` wins, and its value is read as
//!   JavaScript's `parseInt` reads it, so `14.5` is 14 and a word is no number;
//! - a value below 1 counts as 1, the shell's own floor.
//!
//! A missing file, section, line, or number leaves the default of 12.

use std::{fs, path::Path};

/// The base size the shell uses when nothing sets one, in pixels.
pub const DEFAULT_BASE_SIZE: u32 = 12;

/// The section and key `omarchy-display-text-size` writes.
const FONT_SECTION: &str = "font";
const BASE_SIZE_KEY: &str = "base-size";

/// Reads the base size from the shell configuration at `shell_config`.
///
/// Never fails: an unreadable file is the same as one that sets nothing.
#[must_use]
pub fn read_base_size(shell_config: &Path) -> u32 {
    fs::read(shell_config).map_or(DEFAULT_BASE_SIZE, |bytes| {
        base_size(&String::from_utf8_lossy(&bytes))
    })
}

/// The base size a shell configuration's text sets, or the default.
fn base_size(contents: &str) -> u32 {
    let mut section = None;
    let mut value = None;
    for line in contents.lines().map(|line| line.trim_matches(is_blank)) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = section_header(line) {
            section = Some(name);
        } else if section == Some(FONT_SECTION)
            && let Some((BASE_SIZE_KEY, raw)) = key_value(line)
        {
            value = Some(raw);
        }
    }
    value
        .and_then(leading_integer)
        .map_or(DEFAULT_BASE_SIZE, |size| size.max(1))
}

/// The section a `[name]` line opens, if `line` is one.
fn section_header(line: &str) -> Option<&str> {
    let (name, rest) = line.strip_prefix('[')?.split_once(']')?;
    (is_name(name) && is_comment_or_nothing(rest)).then_some(name)
}

/// The key and raw value of a `key = value` line, if the shell would read it.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim_end_matches(is_blank);
    if !is_name(key) {
        return None;
    }
    let value = value.trim_start_matches(is_blank);
    match value.chars().next()? {
        '"' | '\'' => quoted(value),
        _ => unquoted(value),
    }
    .map(|raw| (key, raw))
}

/// The text between a value's quotes: any quote closes it, as in the shell.
fn quoted(value: &str) -> Option<&str> {
    let (inner, rest) = value[1..].split_once(['"', '\''])?;
    (!inner.is_empty() && is_comment_or_nothing(rest)).then_some(inner)
}

/// An unquoted value, without its comment, if it has one of the shell's shapes.
fn unquoted(value: &str) -> Option<&str> {
    let body = value
        .split_once('#')
        .map_or(value, |(body, _)| body)
        .trim_end_matches(is_blank);
    let numbers = body.split_whitespace().count();
    let shaped = if numbers == 1 {
        is_number(body) || is_bare_word(body)
    } else {
        (2..=4).contains(&numbers) && body.split_whitespace().all(is_number)
    };
    shaped.then_some(body)
}

/// The integer `value` starts with, as `parseInt(value, 10)` reads it.
///
/// A negative number is reported as 0, since every caller floors it to 1
/// anyway, and one too large for `u32` saturates rather than wrapping.
fn leading_integer(value: &str) -> Option<u32> {
    let value = value.trim_start_matches(is_blank);
    let (negative, unsigned) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let digits = unsigned.bytes().take_while(u8::is_ascii_digit);
    let mut parsed = None;
    for digit in digits {
        let sum = parsed.unwrap_or(0_u32).saturating_mul(10);
        parsed = Some(sum.saturating_add(u32::from(digit - b'0')));
    }
    parsed.map(|size| if negative { 0 } else { size })
}

/// A section or key name: letters, digits, `_` and `-`, at least one.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

/// A number as the shell matches one: an optional minus, digits, an optional
/// fraction.
fn is_number(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, "1"));
    is_digits(whole) && is_digits(fraction)
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// A bare word, such as a colour role's name: a letter, then name characters.
fn is_bare_word(text: &str) -> bool {
    text.starts_with(|character: char| character.is_ascii_alphabetic()) && is_name(text)
}

/// Whitespace as JavaScript's `\s` matches it: Unicode's, and the byte-order
/// mark, which a file saved with one carries before its first line.
fn is_blank(character: char) -> bool {
    character.is_whitespace() || character == '\u{feff}'
}

/// What may follow a section header or a value: nothing, or a comment.
fn is_comment_or_nothing(rest: &str) -> bool {
    let rest = rest.trim_start_matches(is_blank);
    rest.is_empty() || rest.starts_with('#')
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{DEFAULT_BASE_SIZE, base_size, read_base_size};

    #[test]
    fn the_file_as_the_command_writes_it_sets_the_size() {
        // Given: what `omarchy-display-text-size 16` writes into a fresh file.
        let contents = "[font]\nbase-size = 16\n";

        // When / Then
        assert_eq!(base_size(contents), 16);
    }

    #[test]
    fn the_file_as_reset_leaves_it_means_the_default() {
        // Given: `reset` deletes the line and keeps everything else.
        let contents = "[bar]\nbackground-alpha = 0.9\n\n[font]\n";

        // When / Then
        assert_eq!(base_size(contents), DEFAULT_BASE_SIZE);
    }

    #[test]
    fn a_missing_file_means_the_default() {
        // Given
        let directory = tempdir().expect("a temporary directory");

        // When / Then
        assert_eq!(
            read_base_size(&directory.path().join("shell.toml")),
            DEFAULT_BASE_SIZE
        );
    }

    #[test]
    fn a_file_on_disk_is_read() {
        // Given
        let directory = tempdir().expect("a temporary directory");
        let path = directory.path().join("shell.toml");
        fs::write(&path, "[font]\nbase-size = 18\n").expect("the setting should be written");

        // When / Then
        assert_eq!(read_base_size(&path), 18);
    }

    #[test]
    fn the_size_counts_only_inside_the_font_section() {
        // Given / When / Then
        assert_eq!(base_size("base-size = 16\n"), DEFAULT_BASE_SIZE);
        assert_eq!(base_size("[bar]\nbase-size = 16\n"), DEFAULT_BASE_SIZE);
        assert_eq!(
            base_size("[font]\nbase-size = 16\n[bar]\nbase-size = 9\n"),
            16
        );
    }

    #[test]
    fn lines_toml_refuses_elsewhere_in_the_file_cost_nothing() {
        // Given: bare role names and width lists, which the shell accepts.
        let contents = "[bar]\nactive = red\nnormal-border-width = 1 2\n\n\
            [controls]\nnormal-border = rgba(1,2,3)\n\n[font]\nbase-size = 18\n";

        // When / Then
        assert_eq!(base_size(contents), 18);
    }

    #[test]
    fn a_quoted_value_and_a_comment_are_read_as_the_shell_reads_them() {
        // Given / When / Then
        assert_eq!(base_size("[font]\nbase-size = \"15\"\n"), 15);
        assert_eq!(base_size("[font]\nbase-size = '15' # quoted\n"), 15);
        assert_eq!(base_size("[font]\nbase-size = 17 # bigger\n"), 17);
        assert_eq!(base_size("[font]\nbase-size=17#tight\n"), 17);
        assert_eq!(base_size("[font] # sizes\nbase-size = 13\n"), 13);
    }

    #[test]
    fn a_decimal_keeps_its_integer_part() {
        // Given / When / Then
        assert_eq!(base_size("[font]\nbase-size = 14.5\n"), 14);
        assert_eq!(base_size("[font]\nbase-size = \"14.9px\"\n"), 14);
    }

    #[test]
    fn the_last_line_wins_even_when_it_holds_no_number() {
        // Given / When / Then
        assert_eq!(base_size("[font]\nbase-size = 9\nbase-size = 20\n"), 20);
        assert_eq!(
            base_size("[font]\nbase-size = 16\nbase-size = large\n"),
            DEFAULT_BASE_SIZE
        );
    }

    #[test]
    fn a_line_the_shell_would_not_read_changes_nothing() {
        // Given: values of no shape the shell accepts, after a good one.
        let contents = "[font]\nbase-size = 16\nbase-size = 14 junk!\n\
            base-size = \"\"\nbase-size = 1 2 3 4 5\nbase size = 9\n[font.x]\nbase-size = 9\n";

        // When / Then: `[font.x]` is no section header either, so `[font]` holds.
        assert_eq!(base_size(contents), 9);
        assert_eq!(
            base_size(&contents.replace("[font.x]\nbase-size = 9\n", "")),
            16
        );
    }

    #[test]
    fn a_byte_order_mark_before_the_first_section_is_whitespace() {
        // Given: a file an editor saved with a byte-order mark.
        let contents = "\u{feff}[font]\nbase-size = 16\n";

        // When / Then: the shell reads 16, and so does this.
        assert_eq!(base_size(contents), 16);
    }

    #[test]
    fn a_byte_that_is_not_utf8_costs_only_its_own_character() {
        // Given: a Latin-1 é in a comment, which is no UTF-8.
        let directory = tempdir().expect("a temporary directory");
        let path = directory.path().join("shell.toml");
        fs::write(&path, b"# caf\xe9\n[font]\nbase-size = 16\n")
            .expect("the setting should be written");

        // When / Then: the shell decodes it leniently and reads 16, and so does this.
        assert_eq!(read_base_size(&path), 16);
    }

    #[test]
    fn a_size_below_one_counts_as_one() {
        // Given / When / Then
        assert_eq!(base_size("[font]\nbase-size = 0\n"), 1);
        assert_eq!(base_size("[font]\nbase-size = -3\n"), 1);
    }

    #[test]
    fn a_huge_size_saturates_instead_of_wrapping() {
        // Given / When / Then
        assert_eq!(base_size("[font]\nbase-size = 99999999999999\n"), u32::MAX);
    }

    #[test]
    fn junk_never_panics_and_means_the_default() {
        // Given / When / Then
        for contents in [
            "",
            "[",
            "[]",
            "[font",
            "=",
            "[font]\n=",
            "[font]\nbase-size =",
            "[font]\n\"\n",
            "[font]\nbase-size = '",
            "[font]\nbase-size = -",
            "[font]\nbase-size = é\n",
            "\u{feff}[font]\nbase-size = \u{2014}\n",
        ] {
            assert_eq!(base_size(contents), DEFAULT_BASE_SIZE, "{contents:?}");
        }
    }

    #[test]
    fn the_shipped_theme_template_reads_as_the_default() {
        // Given: the shape of Omarchy's generated theme `shell.toml`.
        let contents = "# Omarchy shell surfaces.\n[bar]\nbackground = \"#2e3440\"\n\
            scale-with-font  = true\nsize-horizontal  = 26\n\n[font]\n\
            # base-size is the rem root for the type scale.\nbase-size = 12\n";

        // When / Then
        assert_eq!(base_size(contents), 12);
    }
}
