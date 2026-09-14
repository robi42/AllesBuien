//! Reading the rendered QML object tree of the booted window.
//!
//! Several regressions this app has shipped were outcomes of layout and paint
//! that no controller property reports: a sentence printed twice, a reading
//! elided at paint time, a caption that went blank instead of away. Asserting
//! them means finding a named item in the live tree and reading a property off
//! it, and cxx-qt-lib binds neither `findChild` nor `QObject::property`.
//!
//! So this binds them directly, by their Itanium-mangled Qt 6 names. The types
//! below are opaque: Rust never reads their layout, only passes the pointers Qt
//! handed out back to Qt. Nothing here can be proven by a unit test, because a
//! library test process must not construct a `QGuiApplication`; every link name
//! is proven instead by the harnesses that call it, and a missing or renamed
//! symbol fails their link rather than passing silently.
//!
//! A list view's delegates are the one part of the tree `findChild` cannot see:
//! Qt 6 parents them to the view's content item as items, never as objects. So
//! [`delegate`] asks the view for one by row, through `QQuickItemView`'s own
//! `itemAtIndex`, and [`descendant`] searches below whatever item it returns.
//!
//! Every function is `unsafe` with one shared contract, stated once here: the
//! pointers passed in came from [`focused_window`], [`child`], [`delegate`], or
//! [`descendant`] on the harness's
//! booted application, that application is still alive, and the call is made
//! on the thread that booted it. A non-null pointer proves none of that.

use std::{
    ffi::{CString, c_char},
    ptr::NonNull,
};

use cxx_qt_lib::{QImage, QString, QVariant};

/// An opaque `QObject`, only ever handled through a pointer Qt returned.
#[repr(C)]
pub struct QObject {
    _private: [u8; 0],
}

/// An opaque `QMetaObject`; only `QObject::staticMetaObject` is ever named.
#[repr(C)]
struct QMetaObject {
    _private: [u8; 0],
}

/// An opaque `QQuickWindow`, only ever handled through a pointer Qt returned.
#[repr(C)]
pub struct QQuickWindow {
    _private: [u8; 0],
}

/// `Qt::FindChildrenRecursively`: search the whole tree, not only direct children.
const FIND_RECURSIVELY: i32 = 1;

unsafe extern "C" {
    #[link_name = "_ZN15QGuiApplication11focusWindowEv"]
    fn focus_window() -> *mut QQuickWindow;
    #[link_name = "_Z20qt_qFindChild_helperPK7QObjectRK7QStringRK11QMetaObject6QFlagsIN2Qt15FindChildOptionEE"]
    fn find_child_helper(
        parent: *const QObject,
        name: &QString,
        meta_object: &QMetaObject,
        options: i32,
    ) -> *mut QObject;
    // Returns a `QQuickItem *`. `QObject` is `QQuickItem`'s first base, so the
    // same address is the `QObject *` every other probe takes.
    #[link_name = "_ZNK14QQuickItemView11itemAtIndexEi"]
    fn item_view_item_at_index(view: *const QObject, index: i32) -> *mut QObject;
    #[link_name = "_ZNK7QObject8propertyEPKc"]
    fn object_property(object: *const QObject, name: *const c_char) -> QVariant;
    #[link_name = "_ZN12QQuickWindow10grabWindowEv"]
    fn grab_quick_window(window: *mut QQuickWindow) -> QImage;

    #[link_name = "_ZN7QObject16staticMetaObjectE"]
    static QOBJECT_STATIC_META_OBJECT: QMetaObject;
}

/// The window Qt currently gives focus to, once the root window has one.
///
/// # Safety
///
/// Call only on the thread that booted this process's `QGuiApplication`, while
/// that application is alive.
#[must_use]
pub unsafe fn focused_window() -> Option<NonNull<QQuickWindow>> {
    // SAFETY: the caller upholds the module contract; the result is checked for
    // null before anyone can use it.
    NonNull::new(unsafe { focus_window() })
}

/// The item named `name` anywhere under `window`, if the tree holds one.
///
/// # Safety
///
/// `window` must come from [`focused_window`] on the still-live booted
/// application, and the call must be on its GUI thread.
#[must_use]
pub unsafe fn child(window: NonNull<QQuickWindow>, name: &str) -> Option<NonNull<QObject>> {
    // SAFETY: the caller upholds the module contract, and a `QQuickWindow` is a
    // `QObject`.
    unsafe { descendant(window.cast::<QObject>(), name) }
}

/// The item named `name` anywhere under `object`, if its subtree holds one.
///
/// # Safety
///
/// As [`property`].
#[must_use]
pub unsafe fn descendant(object: NonNull<QObject>, name: &str) -> Option<NonNull<QObject>> {
    // SAFETY: the caller upholds the module contract, and the result is checked
    // for null before it is returned.
    NonNull::new(unsafe {
        find_child_helper(
            object.as_ptr(),
            &QString::from(name),
            &QOBJECT_STATIC_META_OBJECT,
            FIND_RECURSIVELY,
        )
    })
}

/// The delegate `view` currently shows for `row`, if it has created one.
///
/// # Safety
///
/// As [`property`], and `view` must be a `ListView`, `GridView`, or another
/// `QQuickItemView`: the call is dispatched on that class, not looked up.
#[must_use]
pub unsafe fn delegate(view: NonNull<QObject>, row: i32) -> Option<NonNull<QObject>> {
    // SAFETY: the caller upholds the module contract and guarantees `view` is a
    // `QQuickItemView`; the result is checked for null before it is returned.
    NonNull::new(unsafe { item_view_item_at_index(view.as_ptr(), row) })
}

/// The value of `object`'s QML property `name`; invalid when it has none.
///
/// # Safety
///
/// `object` must come from [`child`] on the still-live booted application, and
/// the call must be on its GUI thread.
#[must_use]
pub unsafe fn property(object: NonNull<QObject>, name: &str) -> QVariant {
    let name = property_name(name);
    // SAFETY: the caller upholds the module contract, and `name` is a
    // NUL-terminated string that outlives the call.
    unsafe { object_property(object.as_ptr(), name.as_ptr()) }
}

/// The `text` of a label-like item.
///
/// # Safety
///
/// As [`property`].
///
/// # Panics
///
/// When the item has no string `text` property.
#[must_use]
pub unsafe fn text_of(object: NonNull<QObject>) -> String {
    // SAFETY: the caller upholds `property`'s contract.
    unsafe { property(object, "text") }
        .value::<QString>()
        .expect("a label-like item should carry text")
        .to_string()
}

/// A numeric property such as `width` or `implicitWidth`.
///
/// # Safety
///
/// As [`property`].
///
/// # Panics
///
/// When the item has no such property or it is not a number.
#[must_use]
pub unsafe fn number_of(object: NonNull<QObject>, name: &str) -> f64 {
    // SAFETY: the caller upholds `property`'s contract.
    unsafe { property(object, name) }
        .value::<f64>()
        .unwrap_or_else(|| panic!("{name} should be a readable number"))
}

/// Renders `window` into an image, as it would appear on screen.
///
/// # Safety
///
/// As [`child`].
#[must_use]
pub unsafe fn grab_window(window: NonNull<QQuickWindow>) -> QImage {
    // SAFETY: the caller upholds the module contract.
    unsafe { grab_quick_window(window.as_ptr()) }
}

/// A property name as the NUL-terminated string `QObject::property` reads.
fn property_name(name: &str) -> CString {
    CString::new(name).unwrap_or_else(|_| panic!("property name {name:?} contains a NUL byte"))
}

#[cfg(test)]
mod tests {
    use super::property_name;

    #[test]
    fn a_property_name_is_passed_on_nul_terminated_and_unchanged() {
        // Given / When
        let name = property_name("implicitWidth");

        // Then
        assert_eq!(name.as_bytes_with_nul(), b"implicitWidth\0");
    }

    #[test]
    #[should_panic(expected = "contains a NUL byte")]
    fn a_property_name_with_an_interior_nul_is_refused_by_name() {
        // Given / When / Then
        let _ = property_name("te\0xt");
    }
}
