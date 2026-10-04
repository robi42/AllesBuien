//! Driving keyboard input into the booted window, the way a keyboard does.
//!
//! The radar's playback keys are window-context `Shortcut`s that have to win
//! over whichever control holds the focus, and the only proof of that is to let
//! Qt decide in the order it decides for a real key: the shortcut map first,
//! then the focused item. A key event handed to the window with
//! `QCoreApplication::sendEvent` skips the first half. It goes straight to the
//! focused item, so a focused button would click and a test would be measuring
//! a path no keyboard takes.
//!
//! `qt_handleKeyEvent` is the entry point `QTest`'s own `keyClick` uses for a
//! `QWindow`. It enters the event through the window system interface as a
//! spontaneous one and delivers it before returning, so whatever the key does
//! has happened by the time these functions do. cxx-qt-lib does not bind it,
//! so, like [`crate::probe`], this binds it by its Itanium-mangled Qt 6 name.
//!
//! Every function is `unsafe` with the contract [`crate::probe`] states: the
//! window came from [`crate::probe::focused_window`] on the harness's booted
//! application, that application is still alive, and the call is made on the
//! thread that booted it.

use std::ptr::NonNull;

use cxx_qt_lib::QString;

use crate::probe::QQuickWindow;

/// `QEvent::KeyPress`.
const KEY_PRESS: i32 = 6;
/// `QEvent::KeyRelease`.
const KEY_RELEASE: i32 = 7;
/// `Qt::ShiftModifier`.
const SHIFT: u32 = 0x0200_0000;
/// `Qt::KeypadModifier`.
const KEYPAD: u32 = 0x2000_0000;

unsafe extern "C" {
    // A `QQuickWindow` is its `QWindow` at the same address. `QEvent::Type` is
    // an `int`, and `QFlags<Qt::KeyboardModifier>` is one `unsigned int` with
    // trivial copy and destruction, so the Itanium ABI passes it by value as
    // exactly that.
    #[link_name = "_Z17qt_handleKeyEventP7QWindowN6QEvent4TypeEi6QFlagsIN2Qt16KeyboardModifierEERK7QStringbt"]
    fn handle_key_event(
        window: *mut QQuickWindow,
        event_type: i32,
        key: i32,
        modifiers: u32,
        text: &QString,
        autorepeat: bool,
        count: u16,
    );
}

/// A key the radar's keyboard scheme names, as a keyboard reports it.
#[derive(Clone, Copy, Debug)]
pub enum Key {
    Space,
    Return,
    /// `Key_Enter` without the keypad modifier, the spelling a remapped key or
    /// another platform sends.
    Enter,
    /// The keypad's Enter: `Key_Enter` carrying `Qt::KeypadModifier`, which is
    /// what a keypad actually sends, and which a `Shortcut` for `Enter` only
    /// matches because Qt retries the lookup without that modifier.
    KeypadEnter,
    Tab,
    /// Shift+Tab, which arrives as `Key_Backtab` with Shift still held.
    Backtab,
    Home,
    End,
    Left,
    Right,
}

impl Key {
    /// The `Qt::Key` code, the modifiers held with it, and the text it types.
    const fn event(self) -> (i32, u32, &'static str) {
        match self {
            Self::Space => (0x20, 0, " "),
            Self::Return => (0x0100_0004, 0, "\r"),
            Self::Enter => (0x0100_0005, 0, "\r"),
            Self::KeypadEnter => (0x0100_0005, KEYPAD, "\r"),
            Self::Tab => (0x0100_0001, 0, "\t"),
            Self::Backtab => (0x0100_0002, SHIFT, ""),
            Self::Home => (0x0100_0010, 0, ""),
            Self::End => (0x0100_0011, 0, ""),
            Self::Left => (0x0100_0012, 0, ""),
            Self::Right => (0x0100_0014, 0, ""),
        }
    }
}

/// Presses and releases `key` once.
///
/// # Safety
///
/// `window` must come from [`crate::probe::focused_window`] on the still-live
/// booted application, and the call must be on its GUI thread.
pub unsafe fn key_click(window: NonNull<QQuickWindow>, key: Key) {
    // SAFETY: the caller upholds the module contract.
    unsafe { key_hold(window, key, 0) }
}

/// Holds `key` down long enough for `repeats` auto-repeats, then lets it go.
///
/// The repeats are spelled the way Qt's Wayland and xcb backends deliver a held
/// key: after the first press, each one is a release and a press that are both
/// flagged as auto-repeat, and only the final release is not.
///
/// # Safety
///
/// As [`key_click`].
pub unsafe fn key_hold(window: NonNull<QQuickWindow>, key: Key, repeats: u16) {
    // SAFETY: the caller upholds the module contract.
    unsafe {
        send(window, KEY_PRESS, key, false);
        for _ in 0..repeats {
            send(window, KEY_RELEASE, key, true);
            send(window, KEY_PRESS, key, true);
        }
        send(window, KEY_RELEASE, key, false);
    }
}

/// Enters one key event into `window` through the window system interface.
unsafe fn send(window: NonNull<QQuickWindow>, event_type: i32, key: Key, autorepeat: bool) {
    let (code, modifiers, text) = key.event();
    // SAFETY: the caller upholds the module contract, and `text` outlives the
    // call that borrows it.
    unsafe {
        handle_key_event(
            window.as_ptr(),
            event_type,
            code,
            modifiers,
            &QString::from(text),
            autorepeat,
            1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{KEYPAD, Key, SHIFT};

    #[test]
    fn the_keypad_enter_is_the_enter_key_with_the_keypad_modifier_held() {
        // Given / When
        let (keypad_code, keypad_modifiers, _) = Key::KeypadEnter.event();
        let (enter_code, enter_modifiers, _) = Key::Enter.event();

        // Then
        assert_eq!(keypad_code, enter_code);
        assert_eq!(enter_modifiers, 0);
        assert_eq!(keypad_modifiers, KEYPAD);
    }

    #[test]
    fn shift_tab_arrives_as_its_own_key_with_shift_still_held() {
        // Given / When
        let (backtab_code, backtab_modifiers, _) = Key::Backtab.event();
        let (tab_code, _, _) = Key::Tab.event();

        // Then
        assert_ne!(backtab_code, tab_code);
        assert_eq!(backtab_modifiers, SHIFT);
    }
}
