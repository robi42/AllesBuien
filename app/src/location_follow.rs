//! Whether the desktop's location setting has moved since a bridge adopted it.
//!
//! Polled rather than watched for the same reason the theme is: Omarchy removes
//! and rewrites `weather.json` rather than editing it in place, so an inode-bound
//! watcher stops hearing about it after the first `--clear`.
//!
//! Both bridges poll the same settings independently rather than one telling the
//! other. Each compares against the location it last adopted, so neither can miss
//! a change the other consumed first, and there is no shared cell for them to race
//! over.

use allesbuien_core::{location::LocationConfig, location_settings::LocationSettings};

/// The freshly resolved location, when it differs from the one last adopted.
///
/// Returns `None` both when nothing moved and when the bridge was never given
/// settings to poll, so a caller has exactly one "nothing to do" branch.
pub(crate) fn moved_to(
    settings: Option<&LocationSettings>,
    adopted: Option<&LocationConfig>,
) -> Option<LocationConfig> {
    let resolved = settings?.resolve();
    (adopted != Some(&resolved)).then_some(resolved)
}

#[cfg(test)]
mod tests {
    use allesbuien_core::{
        location::{LocationConfig, LocationSource},
        location_settings::LocationSettings,
    };

    use super::moved_to;

    fn utrecht() -> LocationConfig {
        LocationConfig {
            latitude: 52.09,
            longitude: 5.12,
            label: String::from("Utrecht"),
            source: LocationSource::DesktopSetting,
        }
    }

    #[test]
    fn a_location_is_reported_only_when_it_differs_from_the_adopted_one() {
        // Given
        let settings = LocationSettings::pinned(utrecht());
        let elsewhere = LocationConfig {
            label: String::from("Groningen"),
            ..utrecht()
        };

        // When / Then
        assert_eq!(moved_to(Some(&settings), None), Some(utrecht()));
        assert_eq!(moved_to(Some(&settings), Some(&elsewhere)), Some(utrecht()));
        assert_eq!(moved_to(Some(&settings), Some(&utrecht())), None);
    }

    #[test]
    fn a_bridge_without_settings_never_moves() {
        // Given / When / Then
        assert_eq!(moved_to(None, Some(&utrecht())), None);
    }
}
