//! Rainfall intensity scale for the radar overlay.
//!
//! Buienradar does not document an RGB-to-rainfall table for the sprite images,
//! but it does publish the scale those images are drawn against: the legend on
//! its own map, titled "Neerslag (mm/u)", names five buckets — 0-2, 2-5, 5-10,
//! 10-100 and 100+ — and pairs each with a colour. Those colours run light
//! lavender, blue, deep navy, red, magenta, which is the order the sprite's own
//! ~100-step gradient runs in when its opaque pixels are sorted. The vendor's
//! published buckets are therefore what the overlay means, and reproducing them
//! is more honest than inventing qualitative words for a scale the vendor
//! already quantifies.
//!
//! Each swatch is transcribed from opaque pixels of live
//! `RadarMapRainWebmercatorNL` frames rather than from the legend artwork, so
//! the key matches what the map actually paints. Keeping the values here rather
//! than in QML is what lets the QML stay free of hardcoded colours.
//!
//! The buckets are not linear — they step 2, 3, 5, 90, then open-ended — so
//! they are rendered as equal-width blocks, matching the vendor's own legend,
//! rather than as a proportional gradient the scale would not support.

/// A published rainfall bucket and the sprite colour representing it.
pub struct Bucket {
    /// Vendor caption, e.g. `"10-100"`. Bare range; [`UNIT`] is stated once.
    pub label: &'static str,
    /// Six-digit hex sampled from the sprite ramp.
    pub swatch: &'static str,
}

/// The unit every [`BUCKETS`] label is expressed in, stated once for the key.
pub const UNIT: &str = "mm/u";

/// Buienradar's published scale, lightest rainfall first.
pub const BUCKETS: [Bucket; 5] = [
    Bucket {
        label: "0-2",
        swatch: "#cbcfff",
    },
    Bucket {
        label: "2-5",
        swatch: "#7f88ff",
    },
    Bucket {
        label: "5-10",
        swatch: "#0c21d4",
    },
    Bucket {
        label: "10-100",
        swatch: "#fc1602",
    },
    Bucket {
        label: "100+",
        swatch: "#961957",
    },
];

/// Swatches alone, lightest first, for drawing the key as a ramp.
#[must_use]
pub fn ramp() -> Vec<&'static str> {
    BUCKETS.iter().map(|bucket| bucket.swatch).collect()
}

/// Captions alone, lightest first, aligned index-for-index with [`ramp`].
#[must_use]
pub fn labels() -> Vec<&'static str> {
    BUCKETS.iter().map(|bucket| bucket.label).collect()
}

#[cfg(test)]
mod tests {
    use super::{BUCKETS, UNIT, labels, ramp};

    fn channels(swatch: &str) -> [u8; 3] {
        let digits = swatch.strip_prefix('#').expect("swatch needs a # prefix");
        assert_eq!(digits.len(), 6, "swatch {swatch} is not six hex digits");
        [0, 2, 4].map(|start| {
            u8::from_str_radix(&digits[start..start + 2], 16)
                .unwrap_or_else(|_| panic!("swatch {swatch} is not hex"))
        })
    }

    #[test]
    fn reproduces_the_vendor_published_buckets_in_order() {
        // Given / When / Then: exactly the scale printed on Buienradar's legend.
        assert_eq!(labels(), ["0-2", "2-5", "5-10", "10-100", "100+"]);
        assert_eq!(UNIT, "mm/u");
    }

    #[test]
    fn every_swatch_is_a_distinct_six_digit_hex_colour() {
        // Given / When
        let mut seen: Vec<[u8; 3]> = BUCKETS
            .iter()
            .map(|bucket| channels(bucket.swatch))
            .collect();
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();

        // Then
        assert_eq!(seen.len(), total);
    }

    #[test]
    fn darkens_through_the_blues_so_the_key_reads_as_rising_rainfall() {
        // Given: the first three buckets are the blue run of the sprite ramp.
        let luma = |swatch: &str| {
            let [red, green, blue] = channels(swatch);
            u32::from(red) + u32::from(green) + u32::from(blue)
        };

        // When / Then
        assert!(luma(BUCKETS[0].swatch) > luma(BUCKETS[1].swatch));
        assert!(luma(BUCKETS[1].swatch) > luma(BUCKETS[2].swatch));
    }

    #[test]
    fn ramp_and_labels_stay_index_aligned() {
        // Given / When / Then
        assert_eq!(ramp().len(), labels().len());
        assert_eq!(ramp()[3], BUCKETS[3].swatch);
        assert_eq!(labels()[3], BUCKETS[3].label);
    }

    #[test]
    fn labels_omit_the_unit_because_the_key_states_it_once() {
        // Given / When / Then: avoids "10-100 mm/u" repeated five times.
        for bucket in &BUCKETS {
            assert!(
                !bucket.label.to_ascii_lowercase().contains("mm"),
                "label {} repeats the unit",
                bucket.label
            );
        }
    }
}
