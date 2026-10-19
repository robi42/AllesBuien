//! Colours as the GUI harnesses sample them off a rendered window, and how far
//! apart two have to be before a reader can tell them apart.
//!
//! `icon_skeleton` judges a loading placeholder against its card and
//! `radar_keys` a focus ring against the theme's foreground. Both read pixels
//! through `allesbuien-harness`, which links Qt, and judge them with the same
//! arithmetic, which does not; so the arithmetic lives here, with a test of its
//! own.

/// One sampled colour: its red, green, and blue channels, each `0..=255`.
pub type Rgb = [i32; 3];

/// The per-channel difference a reader can actually see, as `ForecastView`
/// measured for its row rules.
pub const VISIBLE_LEVELS: i32 = 12;

/// The largest per-channel difference between two colours.
#[must_use]
pub fn spread(first: Rgb, second: Rgb) -> i32 {
    (0..3)
        .map(|channel| (first[channel] - second[channel]).abs())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{VISIBLE_LEVELS, spread};

    #[test]
    fn the_spread_is_the_widest_gap_in_any_one_channel() {
        // Given: Nord's background, and a grey one visible step lighter.
        let background = [0x2e, 0x34, 0x40];
        let lighter = [0x2e + VISIBLE_LEVELS, 0x34, 0x40];

        // When / Then: identical colours do not differ, the widest channel
        // decides, and the direction of a difference does not matter.
        assert_eq!(spread(background, background), 0);
        assert_eq!(spread(background, lighter), VISIBLE_LEVELS);
        assert_eq!(spread([10, 200, 30], [40, 190, 25]), 30);
        assert_eq!(spread([0, 0, 255], [255, 0, 0]), 255);
        assert_eq!(spread([255, 0, 0], [0, 0, 255]), 255);
    }
}
