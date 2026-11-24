//! The bounded on-disk PNG store behind the condition-icon cache.
//!
//! Nothing here is ever written into the source tree: Buienradar's artwork is
//! licensed for use, not for redistribution, so it lives only under the user's
//! XDG cache root.

use std::path::{Path, PathBuf};

/// PNG signature every stored body must start with to count as complete.
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Ceiling for one icon body, applied as it downloads and again to every
/// stored file. The 43 icons the CDN served on 24 November 2026 measured 886
/// to 2 818 B, so this is about ninety times the largest.
pub(crate) const MAX_ICON_BYTES: usize = 256 * 1024;

/// Ceiling for the whole store, comfortably above the 22 documented codes and
/// their night variants, so pruning only ever fires on a vendor expansion.
pub(crate) const MAX_CACHED_ICONS: usize = 64;

/// Resolves the XDG store location, taking both roots as arguments so the
/// precedence is testable without mutating this process's environment.
#[must_use]
pub fn cache_directory(xdg_cache_home: Option<&Path>, home: &Path) -> PathBuf {
    xdg_cache_home
        .map_or_else(|| home.join(".cache"), Path::to_path_buf)
        .join("allesbuien")
        .join("icons")
}

#[must_use]
pub(crate) fn entry_path(directory: &Path, key: &str) -> PathBuf {
    directory.join(format!("{key}.png"))
}

/// Reads a stored entry, treating a truncated or non-PNG file as a miss so the
/// caller re-downloads it instead of publishing bytes Qt cannot decode.
#[must_use]
pub(crate) fn read(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path)
        .ok()
        .filter(|bytes| is_complete_png(bytes))
}

#[must_use]
pub(crate) fn is_complete_png(bytes: &[u8]) -> bool {
    bytes.starts_with(PNG_MAGIC) && bytes.len() <= MAX_ICON_BYTES
}

/// Writes through a temporary file so an interrupted run cannot leave a
/// half-written entry that a later read would mistake for a complete icon.
pub(crate) fn write(directory: &Path, path: &Path, bytes: &[u8]) {
    if std::fs::create_dir_all(directory).is_err() {
        return;
    }
    let pending = path.with_extension("part");
    if std::fs::write(&pending, bytes).is_ok() && std::fs::rename(&pending, path).is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    prune(directory);
}

fn prune(directory: &Path) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(MAX_CACHED_ICONS);
    for stale in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(stale);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::{MAX_CACHED_ICONS, MAX_ICON_BYTES, PNG_MAGIC, cache_directory, entry_path, read};

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbody";

    #[test]
    fn the_xdg_root_wins_over_the_home_fallback() {
        // Given
        let xdg = PathBuf::from("/xdg-cache");
        let home = PathBuf::from("/home/someone");

        // When / Then
        assert_eq!(
            cache_directory(Some(&xdg), &home),
            PathBuf::from("/xdg-cache/allesbuien/icons")
        );
        assert_eq!(
            cache_directory(None, &home),
            PathBuf::from("/home/someone/.cache/allesbuien/icons")
        );
    }

    #[test]
    fn a_complete_entry_round_trips_while_a_damaged_one_reads_as_a_miss() {
        // Given
        let directory = tempdir().expect("temp store");
        let intact = entry_path(directory.path(), "QQ");
        let truncated = entry_path(directory.path(), "Q");
        super::write(directory.path(), &intact, PNG);
        std::fs::write(&truncated, b"\x89PN").expect("partial entry");

        // When / Then
        assert_eq!(read(&intact).as_deref(), Some(PNG));
        assert!(read(&truncated).is_none());
        assert!(read(&entry_path(directory.path(), "ZZ")).is_none());
    }

    #[test]
    fn an_oversized_body_is_never_accepted() {
        // Given
        let mut oversized = PNG_MAGIC.to_vec();
        oversized.resize(MAX_ICON_BYTES + 1, 0);

        // When / Then
        assert!(!super::is_complete_png(&oversized));
        assert!(super::is_complete_png(PNG));
    }

    #[test]
    fn writing_leaves_no_partial_file_and_keeps_the_directory_bounded() {
        // Given
        let directory = tempdir().expect("temp store");
        for index in 0..MAX_CACHED_ICONS + 5 {
            std::fs::write(directory.path().join(format!("Z{index:03}.png")), PNG)
                .expect("filler entry");
        }

        // When
        super::write(directory.path(), &entry_path(directory.path(), "AA"), PNG);

        // Then
        let remaining: Vec<String> = std::fs::read_dir(directory.path())
            .expect("temp store")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(remaining.len(), MAX_CACHED_ICONS);
        assert!(
            remaining.iter().all(|name| {
                std::path::Path::new(name)
                    .extension()
                    .is_none_or(|extension| extension != "part")
            }),
            "a temporary write file survived: {remaining:?}"
        );
    }
}
