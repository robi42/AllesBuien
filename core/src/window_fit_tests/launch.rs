//! How the opening size follows the screen while the window launches, when
//! the launch ends, and what moves the opening size after it: nothing.

use super::{DESIGN, end_launch, fit, monitor, size};
use crate::window_fit::WindowFitter;

#[test]
fn while_launching_each_screen_re_fits_the_opening_size() {
    // Given: a launch at 20 px that Qt first reports on its primary screen.
    let mut fitter = WindowFitter::new(DESIGN, 20);
    fitter.track_screen(monitor(2560, 1440));
    assert_eq!(fitter.fit().opening, size(1633, 1267));

    // When: the window lands on another monitor, then a wrong report arrives
    // and is corrected, all before the launch ends.
    fitter.track_screen(monitor(1280, 800));
    assert_eq!(fitter.fit().opening, size(1104, 757));
    fitter.track_screen(monitor(1280, 1440));
    fitter.track_screen(monitor(1280, 800));

    // Then: the opening size is the landing monitor's.
    assert!(fitter.launching());
    assert_eq!(fitter.fit().opening, size(1104, 757));
}

#[test]
fn the_launch_ends_at_the_second_poll_after_the_first_frame() {
    // Given
    let mut fitter = WindowFitter::new(DESIGN, 20);

    // When: polls before any frame count for nothing.
    for _ in 0..5 {
        assert!(!fitter.poll());
    }
    assert!(fitter.launching());

    // Then: a frame, then a first poll, then a second, which ends it.
    fitter.track_frame();
    assert!(!fitter.poll());
    assert!(fitter.launching());
    fitter.track_frame();
    assert!(fitter.poll());
    assert!(!fitter.launching());
    assert!(!fitter.poll(), "the launch ends once");
    fitter.track_frame();
    assert!(!fitter.launching());
}

#[test]
fn after_the_launch_no_screen_moves_the_opening_size() {
    // Given: a window that landed on a 1920×1080 monitor.
    let mut fitter = WindowFitter::new(DESIGN, 20);
    fitter.track_screen(monitor(1920, 1080));
    end_launch(&mut fitter);

    // When: it is moved to a smaller monitor.
    assert!(fitter.track_screen(monitor(1280, 800)));

    // Then: the zoom and the minimum follow; the opening size does not.
    assert_eq!(fitter.fit(), fit(13_517, size(960, 757), size(1512, 1037)));
}

#[test]
fn no_later_text_size_moves_an_opening_size_the_minimum_fits_in() {
    // Given
    let mut fitter = WindowFitter::new(DESIGN, 20);
    fitter.track_screen(monitor(1920, 1080));

    // When / Then: during the launch.
    fitter.set_base_size(9);
    assert_eq!(fitter.fit().zoom, 9_000);
    assert_eq!(fitter.fit().opening, size(1512, 1037));

    // When / Then: and after it.
    end_launch(&mut fitter);
    fitter.set_base_size(16);
    assert_eq!(fitter.fit().zoom, 16_000);
    assert_eq!(fitter.fit().opening, size(1512, 1037));
}

#[test]
fn during_the_launch_the_opening_size_never_falls_below_the_minimum() {
    // Given: a launch at 14 px, on Qt's primary screen, a 1280×800 panel.
    let mut fitter = WindowFitter::new(DESIGN, 14);
    fitter.track_screen(monitor(1280, 800));
    assert_eq!(fitter.fit().opening, size(1123, 770));

    // When: a poll reads 20 px, then the window lands on a 1920×1080 monitor,
    // all before the launch ends.
    fitter.set_base_size(20);
    fitter.track_screen(monitor(1920, 1080));

    // Then: the opening size fitted at 14 px there, 1143×887, is raised to the
    // minimum at 20, below which Qt would otherwise resize the window.
    assert!(fitter.launching());
    assert_eq!(
        fitter.fit(),
        fit(18_517, size(1315, 1037), size(1315, 1037))
    );

    // And: the end of the launch fixes it there, whatever the text size does.
    end_launch(&mut fitter);
    fitter.set_base_size(14);
    assert_eq!(fitter.fit().opening, size(1315, 1037));
}
