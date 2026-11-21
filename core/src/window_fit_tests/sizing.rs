//! The rules that turn a text size and a screen into the zoom, the minimum
//! size, and the opening size.

use super::{DESIGN, fit, fit_on, monitor, size};
use crate::window_fit::{Screen, WindowFitter, ZOOM_UNIT};

#[test]
fn twelve_px_is_the_window_as_designed_on_any_screen() {
    // Given / When / Then
    let designed = fit(ZOOM_UNIT, DESIGN.minimum, DESIGN.opening);
    for screen in [None, Some(monitor(2560, 1440)), Some(monitor(800, 800))] {
        assert_eq!(fit_on(12, screen), designed, "{screen:?}");
    }
}

#[test]
fn the_table_of_the_plan_holds() {
    // Given / When / Then
    assert_eq!(
        fit_on(14, Some(monitor(2560, 1440))),
        fit(14_000, size(994, 784), size(1143, 887))
    );
    assert_eq!(
        fit_on(14, Some(monitor(1280, 800))),
        fit(13_750, size(977, 770), size(1123, 770))
    );
    assert_eq!(
        fit_on(20, Some(monitor(1920, 1080))),
        fit(18_517, size(1315, 1037), size(1512, 1037))
    );
    let narrow = Screen {
        size: size(1920, 1080),
        available: size(1000, 1080),
    };
    assert_eq!(
        fit_on(20, Some(narrow)),
        fit(14_084, size(1000, 789), size(1000, 892))
    );
    assert_eq!(
        fit_on(9, Some(monitor(1920, 1080))),
        fit(9_000, size(639, 504), size(735, 570))
    );
    assert_eq!(fit_on(9, None), fit(9_000, size(639, 504), size(735, 570)));
    assert_eq!(
        fit_on(20, Some(monitor(800, 800))),
        fit(ZOOM_UNIT, DESIGN.minimum, DESIGN.opening)
    );
}

#[test]
fn the_bar_grows_with_the_text_size() {
    // Given: screens whose height is what caps the zoom, so the minimum fills
    // the room's height exactly.
    // When / Then: the bar is round(26 × base-size / 12): 30 px at 14, 43 at 20.
    assert_eq!(
        fit_on(14, Some(monitor(1280, 800))).minimum.height,
        800 - 30
    );
    assert_eq!(
        fit_on(20, Some(monitor(4000, 1080))).minimum.height,
        1080 - 43
    );
}

#[test]
fn before_any_screen_is_reported_nothing_caps_the_zoom() {
    // Given / When / Then
    assert_eq!(
        fit_on(20, None),
        fit(20_000, size(1420, 1120), size(1633, 1267))
    );
}

#[test]
fn a_screen_too_small_for_the_design_never_zooms_it_below_one() {
    // Given / When / Then
    assert_eq!(fit_on(20, Some(monitor(800, 800))).zoom, ZOOM_UNIT);
    assert_eq!(
        fit_on(12, Some(monitor(640, 480))),
        fit(ZOOM_UNIT, DESIGN.minimum, DESIGN.opening)
    );
}

#[test]
fn a_huge_base_size_saturates_instead_of_wrapping() {
    // Given / When
    let uncapped = fit_on(u32::MAX, None);
    let on_screen = fit_on(u32::MAX, Some(monitor(1920, 1080)));

    // Then: nothing wrapped. On a screen, a bar that large leaves no room,
    // so the zoom stays at 1.
    assert_eq!(uncapped.zoom, u32::MAX);
    assert!(uncapped.minimum.width > 852 && uncapped.opening.height > 760);
    assert_eq!(on_screen, fit(ZOOM_UNIT, DESIGN.minimum, DESIGN.opening));
}

#[test]
fn a_smaller_available_area_tightens_the_room_on_either_axis() {
    // Given
    let low = Screen {
        size: size(1920, 1080),
        available: size(1920, 800),
    };

    // When / Then: 800 is less than the 1037 the bar leaves.
    assert_eq!(fit_on(20, Some(low)).minimum.height, 800);
    assert_eq!(fit_on(20, Some(low)).opening, size(1167, 800));
}

#[test]
fn a_larger_available_area_changes_nothing() {
    // Given: the span of two monitors, which QML's available area reports.
    let spanned = Screen {
        size: size(1920, 1080),
        available: size(4480, 1440),
    };

    // When / Then
    assert_eq!(
        fit_on(20, Some(spanned)),
        fit_on(20, Some(monitor(1920, 1080)))
    );
}

#[test]
fn wherever_the_room_holds_the_design_the_minimum_fits_it() {
    // Given: every base size from 1 to 40 on a grid of screens.
    let mut checked = 0_u32;
    for base_size in 1..=40_u32 {
        let bar = (26 * base_size + 6) / 12;
        for width in (600..=4000_u32).step_by(37) {
            for height in (400..=2400_u32).step_by(29) {
                let room = size(width, height.saturating_sub(bar));
                if room.width < 852 || room.height < 672 {
                    continue;
                }

                // When
                let fit = fit_on(base_size, Some(monitor(width, height)));

                // Then
                let at = format!("{base_size} px on {width}×{height}");
                assert!(fit.minimum.width <= room.width, "{at}: {fit:?}");
                assert!(fit.minimum.height <= room.height, "{at}: {fit:?}");
                assert!(fit.opening.width >= fit.minimum.width, "{at}: {fit:?}");
                assert!(fit.opening.height >= fit.minimum.height, "{at}: {fit:?}");
                checked += 1;
            }
        }
    }
    assert!(checked > 100_000, "the grid checked only {checked} cases");
}

#[test]
fn a_report_with_any_number_not_positive_is_no_screen() {
    // Given / When / Then
    assert_eq!(
        Screen::from_report(1920, 1080, 3840, 1440),
        Some(Screen {
            size: size(1920, 1080),
            available: size(3840, 1440),
        })
    );
    for report in [
        (0, 1080, 1920, 1080),
        (1920, -1, 1920, 1080),
        (1920, 1080, 0, 1080),
        (1920, 1080, 1920, i32::MIN),
    ] {
        assert_eq!(
            Screen::from_report(report.0, report.1, report.2, report.3),
            None,
            "{report:?}"
        );
    }
}

#[test]
fn the_setters_report_only_a_changed_fit() {
    // Given
    let mut fitter = WindowFitter::new(DESIGN, 20);

    // When / Then
    assert!(fitter.track_screen(monitor(1920, 1080)));
    assert!(!fitter.track_screen(monitor(1920, 1080)));
    assert!(fitter.set_base_size(14));
    assert!(!fitter.set_base_size(14));
    assert!(fitter.set_base_size(12));
}
