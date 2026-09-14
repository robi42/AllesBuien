//! A pending condition icon shows a skeleton that can actually be seen.
//!
//! The skeleton was once drawn in the colour of the card it sits on, so every
//! binding test passed while nothing appeared on screen. This harness holds the
//! first refresh open forever and reads what the window shows meanwhile: the
//! hero's skeleton is visible, differs in colour from its card, and breathes.

use std::{future::pending, ptr::NonNull, sync::Arc};

use allesbuien_core::weather::{FeedApi, FeedFuture};
use allesbuien_harness::{
    HarnessBuilder,
    probe::{QObject, child, descendant, focused_window, number_of, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, pump_until};
use cxx_qt_lib::QColor;

/// A feed whose answer never arrives, so the window stays in its loading state.
struct SilentFeedApi;

impl FeedApi for SilentFeedApi {
    fn fetch(&self, _latitude: f64, _longitude: f64) -> FeedFuture<'_> {
        Box::pin(pending())
    }
}

/// The hero icon's skeleton and the card beneath it, once both are rendered.
fn skeleton_and_card() -> Option<(NonNull<QObject>, NonNull<QObject>)> {
    // SAFETY: called only on the GUI thread while the booted app is alive.
    unsafe {
        let window = focused_window()?;
        let skeleton = descendant(child(window, "heroIcon")?, "skeleton")?;
        Some((skeleton, child(window, "sideCard")?))
    }
}

fn color_of(object: NonNull<QObject>) -> QColor {
    // SAFETY: `object` was just returned by the probes on the GUI thread.
    unsafe { property(object, "color") }
        .value::<QColor>()
        .expect("a rectangle's color should be a QColor")
}

#[test]
fn a_pending_hero_icon_shows_a_visible_pulsing_skeleton() {
    // Given: a window whose first weather refresh never answers.
    let booted = HarnessBuilder::with_feed(Arc::new(SilentFeedApi)).boot();

    // When: the window is up.
    pump_until("the hero skeleton is rendered", SETTLE_BUDGET, || {
        booted.process_events();
        skeleton_and_card().is_some()
    });
    let (skeleton, card) = skeleton_and_card().expect("the wait returned with both items");

    // Then: the slot is marked as loading and its skeleton is shown in a colour
    // its card does not already have.
    assert!(
        *booted
            .weather_controller()
            .expect("controller should exist")
            .icons_loading()
    );
    // SAFETY: `skeleton` comes from the probes on the live app's GUI thread.
    let visible = unsafe { property(skeleton, "visible") }.value::<bool>();
    assert_eq!(visible, Some(true));
    assert_ne!(color_of(skeleton), color_of(card));

    // Then: the skeleton breathes rather than standing still.
    // SAFETY: as above.
    let first_opacity = unsafe { number_of(skeleton, "opacity") };
    pump_until("the skeleton's opacity moves", SETTLE_BUDGET, || {
        booted.process_events();
        // SAFETY: as above; the hero icon outlives every refresh.
        (unsafe { number_of(skeleton, "opacity") } - first_opacity).abs() > 0.01
    });
}
