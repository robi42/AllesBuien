//! Clicking into the booted window, the way a mouse does.
//!
//! Qt Quick Controls remember how a control got its focus, and the window draws
//! a focus ring for one reason and not the other, so proving that a click leaves
//! no ring takes a click Qt believes. Like [`crate::keys`], this enters the
//! event through the window system interface, with the entry point `QTest`'s own
//! `mouseClick` uses for a `QWindow`. cxx-qt-lib binds neither that nor
//! `QWindow::mapToGlobal`, so both are bound by their Itanium-mangled Qt 6
//! names.
//!
//! Every function is `unsafe` with the contract [`crate::probe`] states: the
//! window came from [`crate::probe::focused_window`] on the harness's booted
//! application, that application is still alive, and the call is made on the
//! thread that booted it.

use std::{
    ptr::NonNull,
    time::{SystemTime, UNIX_EPOCH},
};

use cxx_qt_lib::QPointF;

use crate::probe::QQuickWindow;

/// `QEvent::MouseButtonPress`.
const MOUSE_PRESS: i32 = 2;
/// `QEvent::MouseButtonRelease`.
const MOUSE_RELEASE: i32 = 3;
/// `Qt::LeftButton`, both as the button that changed and as a held-buttons set.
const LEFT_BUTTON: u32 = 1;
/// `Qt::NoButton`.
const NO_BUTTONS: u32 = 0;
/// `Qt::NoModifier`.
const NO_MODIFIERS: u32 = 0;

unsafe extern "C" {
    // A `QQuickWindow` is its `QWindow` at the same address. Both `QFlags` are
    // one `unsigned int` with trivial copy and destruction, and `Qt::MouseButton`
    // and `QEvent::Type` are `int`-sized enums, so each travels as exactly that.
    #[link_name = "_Z19qt_handleMouseEventP7QWindowRK7QPointFS3_6QFlagsIN2Qt11MouseButtonEES6_N6QEvent4TypeES4_INS5_16KeyboardModifierEEi"]
    fn handle_mouse_event(
        window: *mut QQuickWindow,
        local: &QPointF,
        global: &QPointF,
        buttons: u32,
        button: u32,
        event_type: i32,
        modifiers: u32,
        timestamp: i32,
    );
    // `QPointF` is two `qreal`s with trivial copy and destruction, so the
    // Itanium ABI returns it in registers exactly as a `repr(C)` pair of `f64`.
    #[link_name = "_ZNK7QWindow11mapToGlobalERK7QPointF"]
    fn window_map_to_global(window: *const QQuickWindow, point: &QPointF) -> QPointF;
}

/// Presses and releases the left button at `at`, in the window's own logical
/// coordinates, as [`crate::probe::map_to_scene`] reports them.
///
/// The events carry the wall clock's milliseconds, as a real mouse's do, so two
/// clicks on one spot inside the double-click interval are a double-click here
/// exactly as they would be by hand.
///
/// # Safety
///
/// `window` must come from [`crate::probe::focused_window`] on the still-live
/// booted application, and the call must be on its GUI thread.
pub unsafe fn click(window: NonNull<QQuickWindow>, at: &QPointF) {
    let timestamp = timestamp_ms(SystemTime::now());
    // SAFETY: the caller upholds the module contract, and both points outlive
    // the calls that borrow them.
    unsafe {
        let global = window_map_to_global(window.as_ptr(), at);
        for (event_type, buttons) in [(MOUSE_PRESS, LEFT_BUTTON), (MOUSE_RELEASE, NO_BUTTONS)] {
            handle_mouse_event(
                window.as_ptr(),
                at,
                &global,
                buttons,
                LEFT_BUTTON,
                event_type,
                NO_MODIFIERS,
                timestamp,
            );
        }
    }
}

/// `now` as the `int` milliseconds Qt's input events carry. Qt only ever
/// subtracts two of them, so wrapping at `i32::MAX` costs nothing but a
/// double-click straddling the wrap, once every 24.8 days.
fn timestamp_ms(now: SystemTime) -> i32 {
    let millis = now
        .duration_since(UNIX_EPOCH)
        .expect("the wall clock should read after 1970")
        .as_millis();
    i32::try_from(millis % (1_u128 << 31)).expect("a remainder of 2^31 fits an i32")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::timestamp_ms;

    #[test]
    fn a_timestamp_counts_milliseconds_and_wraps_into_an_int() {
        // Given
        let second = UNIX_EPOCH + Duration::from_millis(1_000);
        let past_the_wrap = UNIX_EPOCH + Duration::from_millis((1 << 31) + 7);

        // When / Then
        assert_eq!(timestamp_ms(second), 1_000);
        assert_eq!(timestamp_ms(past_the_wrap), 7);
    }
}
