//! Qualitative intensity ramp for the radar overlay.
//!
//! Buienradar publishes **no** RGB-to-rainfall mapping for these sprites. Its
//! own legend page names two qualitative buckets and no numbers; Home Assistant
//! and `python-buienradar` both consume the imagery without ever decoding a
//! pixel. Printing `mm/h` or `dBZ` ticks would therefore assert a precision no
//! source backs, so [`BUCKETS`] stays qualitative on purpose — that constraint
//! is pinned by a test, not left to reviewer memory.
//!
//! [`RAMP`] is a 12-swatch sample of the vendor's own ~100-step gradient,
//! transcribed from opaque pixels of four live frames of the
//! `RadarMapRainWebmercatorNL` sprite. It runs light lavender through saturated
//! blue and violet into magenta and red, matching what the map actually draws.
//! Keeping the values here rather than in QML is what lets `Main.qml` stay free
//! of hardcoded colours.

/// Swatches sampled from the radar sprite ramp, lightest rainfall first.
pub const RAMP: [&str; 12] = [
    "#ecedfd", "#cbcfff", "#a8afff", "#858eff", "#5c6af8", "#3243e4", "#0b1bca", "#3a1aa2",
    "#641a80", "#961957", "#c31732", "#fc1602",
];

/// Captions under the ramp, left to right. Qualitative by design — see module docs.
pub const BUCKETS: [&str; 3] = ["Light", "Moderate", "Heavy"];

#[cfg(test)]
mod tests {
    use super::{BUCKETS, RAMP};

    fn channels(swatch: &str) -> [u8; 3] {
        let digits = swatch.strip_prefix('#').expect("swatch needs a # prefix");
        assert_eq!(digits.len(), 6, "swatch {swatch} is not six hex digits");
        [0, 2, 4].map(|start| {
            u8::from_str_radix(&digits[start..start + 2], 16)
                .unwrap_or_else(|_| panic!("swatch {swatch} is not hex"))
        })
    }

    fn luminance(swatch: &str) -> f64 {
        let [red, green, blue] = channels(swatch).map(f64::from);
        0.2126f64.mul_add(red, 0.7152f64.mul_add(green, 0.0722 * blue))
    }

    #[test]
    fn the_ramp_runs_from_the_lightest_swatch_to_the_reddest() {
        // Given: the sampled vendor ramp.
        // When
        let lightest = RAMP
            .iter()
            .copied()
            .max_by(|left, right| luminance(left).total_cmp(&luminance(right)));
        let reddest = RAMP
            .iter()
            .copied()
            .max_by_key(|swatch| channels(swatch)[0]);

        // Then: light rainfall reads first and the heavy red core reads last.
        assert_eq!(lightest, Some(RAMP[0]));
        assert_eq!(reddest, Some(RAMP[RAMP.len() - 1]));
    }

    #[test]
    fn every_swatch_is_a_distinct_six_digit_hex_colour() {
        // Given / When
        let mut seen: Vec<[u8; 3]> = RAMP.iter().map(|swatch| channels(swatch)).collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();

        // Then
        assert_eq!(seen.len(), total);
    }

    #[test]
    fn no_bucket_caption_claims_a_numeric_scale() {
        // Given: settled research — the vendor documents no RGB-to-rate mapping.
        // When / Then
        for caption in BUCKETS {
            assert!(
                !caption.chars().any(|letter| letter.is_ascii_digit()),
                "caption {caption} carries a number"
            );
            let lowered = caption.to_ascii_lowercase();
            for unit in ["mm", "dbz", "/h", "inch"] {
                assert!(!lowered.contains(unit), "caption {caption} claims {unit}");
            }
        }
    }
}
