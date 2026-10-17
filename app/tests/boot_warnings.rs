//! The window has to boot without its own QML warning about itself.
//!
//! Qt reports a QML fault it can recover from as a warning and carries on, so
//! the window still renders and no assertion about it fails. `ConditionsView`
//! logged "Detected recursive rearrange" on every launch that way: the header
//! corner's width cap was half the view's own width, while the view was still
//! sizing itself from its contents, corner included, so each pass widened the
//! next and Qt gave up after two. Nothing read that warning but the journal,
//! and in CI not even that, since with no terminal on stderr Qt writes to
//! journald and the container runs none. So this records Qt's messages from
//! before the window exists and fails on any the app's own QML raised.
//!
//! The location is Utrecht, a short name, on purpose. How many passes the
//! header needed depended on its text: under `Amsterdam-West`, the fixture
//! the other harnesses boot with, the view settled within the two Qt allows,
//! and a shorter name took a third.

use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use allesbuien_core::location_settings::LocationSettings;
use allesbuien_harness::{
    HarnessBuilder,
    probe::{child, focused_window, property},
};
use allesbuien_testkit::{SETTLE_BUDGET, StubFeedApi, pump_until, write_weather};
use cxx_qt_lib::QString;

/// `QtWarningMsg` and `QtFatalMsg`: the warning, critical, and fatal levels
/// are the values from one to the other, and `QtInfoMsg`, four, comes after.
const WARNING_LEVELS: std::ops::RangeInclusive<i32> = 1..=3;

/// Where the app's QML lives in its resources, so a message that starts with
/// it came from one of the app's own files rather than from Qt's platform code.
const APP_QML: &str = "qrc:/qt/qml/nl/allesbuien/";

/// Every message Qt raised at warning level or worse, in order.
///
/// A `static` because Qt calls its handler through a bare function pointer
/// that carries no state. This file is the whole process, so nothing else
/// shares it.
static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// `QtMessageHandler`, with the context and the message as the pointers C++
/// passes its two references as.
type MessageHandler = extern "C" fn(kind: i32, context: *const c_void, message: *const QString);

unsafe extern "C" {
    /// `qInstallMessageHandler`, bound by its Itanium-mangled Qt 6 name, as
    /// `allesbuien_harness::probe` binds the functions it needs.
    #[link_name = "_Z22qInstallMessageHandlerPFv9QtMsgTypeRK18QMessageLogContextRK7QStringE"]
    fn install_message_handler(handler: Option<MessageHandler>) -> Option<MessageHandler>;
}

/// Records one message. C++ calls it, so it must not panic: a poisoned lock
/// drops the message rather than unwinding into Qt.
extern "C" fn record(kind: i32, _context: *const c_void, message: *const QString) {
    if !WARNING_LEVELS.contains(&kind) || message.is_null() {
        return;
    }
    // SAFETY: Qt passes a reference to a live `QString` for the length of the call.
    let text = unsafe { &*message }.to_string();
    if let Ok(mut warnings) = WARNINGS.lock() {
        warnings.push(text);
    }
}

#[test]
fn boots_without_a_warning_from_its_own_qml() {
    // Given: Qt's messages recorded from before the window exists, over a
    // desktop setting that names a short place.
    // SAFETY: called before the `QGuiApplication` exists, with a handler whose
    // signature is `QtMessageHandler`'s and which lives as long as the process.
    unsafe { install_message_handler(Some(record)) };
    let home = tempfile::tempdir().expect("temporary home");
    write_weather(
        home.path(),
        r#"{"name":"Utrecht","latitude":52.09,"longitude":5.12}"#,
    );

    // When: the window boots and its header is laid out at the window's size.
    let booted = HarnessBuilder::with_feed(Arc::new(StubFeedApi))
        .location(LocationSettings::watching(home.path().to_owned()))
        .boot();
    pump_until(
        "the header row is laid out in a focused window",
        SETTLE_BUDGET,
        || {
            booted.process_events();
            // SAFETY: on the GUI thread of the live booted app, whose window
            // and header row the probes return.
            unsafe {
                focused_window()
                    .and_then(|window| child(window, "headerRow"))
                    .and_then(|header| property(header, "width").value::<f64>())
                    .is_some_and(|width| width > 0.0)
            }
        },
    );

    // Then: none of the warnings came from the window's own QML.
    let warnings = WARNINGS.lock().expect("warnings lock").clone();
    let from_app_qml: Vec<&String> = warnings
        .iter()
        .filter(|warning| warning.starts_with(APP_QML))
        .collect();
    assert!(
        from_app_qml.is_empty(),
        "the window's QML warned while booting: {from_app_qml:#?}"
    );
}
