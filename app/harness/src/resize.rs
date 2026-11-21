//! Resizing the booted window, as a person dragging its edge leaves it.
//!
//! The window's size is bound in QML to the size the theme bridge says it
//! opens at, and a binding that is evaluated again resizes the window to it.
//! Proving that a setting the window follows leaves a size the person chose
//! alone takes a window whose size is not the one its binding last gave it.
//! This calls `QWindow::resize` from outside QML, which changes the size and
//! leaves the binding in place, as a size the window system hands the window
//! does. cxx-qt-lib does not bind it, so it is bound by its Itanium-mangled
//! Qt 6 name.
//!
//! The function is `unsafe` with the contract [`crate::probe`] states: the
//! window came from [`crate::probe::focused_window`] on the harness's booted
//! application, that application is still alive, and the call is made on the
//! thread that booted it.

use std::ptr::NonNull;

use crate::probe::QQuickWindow;

unsafe extern "C" {
    // A `QQuickWindow` is its `QWindow` at the same address, and `resize` is
    // not virtual, so the call is the plain member function's.
    #[link_name = "_ZN7QWindow6resizeEii"]
    fn window_resize(window: *mut QQuickWindow, width: i32, height: i32);
}

/// Resizes `window` to `width` × `height`, in logical pixels.
///
/// # Safety
///
/// As [`crate::probe::child`].
pub unsafe fn resize(window: NonNull<QQuickWindow>, width: i32, height: i32) {
    // SAFETY: the caller upholds the module contract.
    unsafe { window_resize(window.as_ptr(), width, height) };
}
