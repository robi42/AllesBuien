//! Delivers already-fetched PNG bytes to QML without composing them in Rust.
//!
//! One provider class answers two registered names because the ids cannot
//! collide: a radar frame is addressed `image://radar/<generation>/<index>` and
//! always carries a `/`, while a condition icon is addressed
//! `image://condition/<CODE>` and is always a bare letter stem. [`ImageId`]
//! makes that the only routing rule, so neither the C++ trampoline nor the
//! constructor has to learn which name it was registered under.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use cxx_qt_lib::{QImage, QQmlImageProviderBaseImageType, QSize, QString};

pub(crate) type FrameBytes = Arc<[u8]>;

#[derive(Default)]
struct FrameStore {
    generation: u64,
    frames: Vec<FrameBytes>,
    icons: HashMap<String, FrameBytes>,
}

/// What a QML `image://` id resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ImageId {
    RadarFrame { generation: u64, index: usize },
    ConditionIcon(String),
}

impl ImageId {
    fn parse(id: &str) -> Option<Self> {
        match id.split_once('/') {
            Some((generation, index)) => Some(Self::RadarFrame {
                generation: generation.parse().ok()?,
                index: index.parse().ok()?,
            }),
            None => crate::condition_icon::cache_key(id).map(Self::ConditionIcon),
        }
    }
}

static FRAME_STORE: OnceLock<RwLock<FrameStore>> = OnceLock::new();

fn frame_store() -> &'static RwLock<FrameStore> {
    FRAME_STORE.get_or_init(|| RwLock::new(FrameStore::default()))
}

pub(crate) fn begin_generation(generation: u64) {
    if let Ok(mut store) = frame_store().write() {
        store.generation = generation;
        store.frames.clear();
    }
}

pub(crate) fn replace_frames(generation: u64, frames: Vec<FrameBytes>) {
    if let Ok(mut store) = frame_store().write()
        && store.generation == generation
    {
        store.frames = frames;
    }
}

/// Publishes one condition icon under its cache key.
pub(crate) fn store_icon(key: &str, bytes: FrameBytes) {
    if let Ok(mut store) = frame_store().write() {
        store.icons.insert(key.to_owned(), bytes);
    }
}

/// Reports whether a code is already published, so warming can skip it.
#[must_use]
pub(crate) fn has_icon(key: &str) -> bool {
    frame_store()
        .read()
        .is_ok_and(|store| store.icons.contains_key(key))
}

/// What QML may bind now for an icon URL.
///
/// A provider URL is withheld as `""` until the store can answer it, so the
/// later re-publish is a change QML reacts to instead of a same-string setter
/// that emits nothing. Any other URL, a CDN fallback or empty, passes through.
#[must_use]
pub(crate) fn published_url(url: &str) -> String {
    match crate::condition_icon::provider_key(url) {
        Some(key) if !has_icon(key) => String::new(),
        _ => url.to_owned(),
    }
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qsize.h");
        type QSize = cxx_qt_lib::QSize;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlimageproviderbase.h");
        type QQmlImageProviderBase = cxx_qt_lib::QQmlImageProviderBase;
        #[namespace = "rust::cxxqtlib1"]
        type QQmlImageProviderBaseImageType = cxx_qt_lib::QQmlImageProviderBaseImageType;

        include!(<QtQuick/QQuickImageProvider>);
        type QQuickImageProvider;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[base = QQuickImageProvider]
        type ImageProvider = super::ImageProviderRust;

        #[cxx_override]
        #[cxx_name = "requestImage"]
        unsafe fn request_image(
            self: Pin<&mut ImageProvider>,
            id: &QString,
            size: *mut QSize,
            requested_size: &QSize,
        ) -> QImage;
    }

    unsafe extern "C++" {
        include!("image_provider.h");
        #[rust_name = "new_image_provider"]
        #[must_use]
        fn imageProviderNew(
            image_type: QQmlImageProviderBaseImageType,
        ) -> *mut QQmlImageProviderBase;
    }

    impl
        cxx_qt::Constructor<
            (QQmlImageProviderBaseImageType,),
            BaseArguments = (QQmlImageProviderBaseImageType,),
        > for ImageProvider
    {
    }
}

pub struct ImageProviderRust;

impl cxx_qt::Constructor<(QQmlImageProviderBaseImageType,)> for ffi::ImageProvider {
    type BaseArguments = (QQmlImageProviderBaseImageType,);
    type InitializeArguments = ();
    type NewArguments = ();

    fn route_arguments(
        arguments: (QQmlImageProviderBaseImageType,),
    ) -> (
        Self::NewArguments,
        Self::BaseArguments,
        Self::InitializeArguments,
    ) {
        ((), arguments, ())
    }

    fn new((): Self::NewArguments) -> ImageProviderRust {
        ImageProviderRust
    }
}

impl ffi::ImageProvider {
    /// Returns the radar frame or condition icon the QML id addresses.
    ///
    /// # Safety
    ///
    /// When `size` is non-null, Qt must provide a valid, writable `QSize`
    /// output pointer for the duration of this call.
    pub unsafe fn request_image(
        self: core::pin::Pin<&mut Self>,
        id: &QString,
        size: *mut QSize,
        _requested_size: &QSize,
    ) -> QImage {
        let image = ImageId::parse(&id.to_string())
            .and_then(published_bytes)
            .and_then(|bytes| QImage::from_data(&bytes, Some("PNG")))
            .unwrap_or_default();
        if !size.is_null() {
            // SAFETY: Qt's requestImage contract supplies a writable QSize output
            // pointer when it is non-null for this call.
            unsafe { size.write(image.size()) };
        }
        image
    }
}

/// Looks up the bytes already published for one id, if any.
fn published_bytes(id: ImageId) -> Option<FrameBytes> {
    let store = frame_store().read().ok()?;
    match id {
        ImageId::RadarFrame { generation, index } if store.generation == generation => {
            store.frames.get(index).cloned()
        }
        ImageId::RadarFrame { .. } => None,
        ImageId::ConditionIcon(key) => store.icons.get(&key).cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FrameBytes, ImageId, begin_generation, has_icon, published_bytes, published_url, store_icon,
    };

    #[test]
    fn a_two_part_numeric_id_addresses_a_radar_frame() {
        // Given / When / Then
        assert_eq!(
            ImageId::parse("7/3"),
            Some(ImageId::RadarFrame {
                generation: 7,
                index: 3
            })
        );
    }

    #[test]
    fn a_bare_letter_stem_addresses_a_condition_icon() {
        // Given / When / Then
        assert_eq!(
            ImageId::parse("qq"),
            Some(ImageId::ConditionIcon(String::from("QQ")))
        );
        assert_eq!(
            ImageId::parse("F"),
            Some(ImageId::ConditionIcon(String::from("F")))
        );
    }

    #[test]
    fn an_id_that_addresses_neither_resolves_to_nothing() {
        // Given / When / Then
        assert_eq!(ImageId::parse(""), None);
        assert_eq!(ImageId::parse("7/not-a-number"), None);
        assert_eq!(ImageId::parse("../../etc/passwd"), None);
    }

    #[test]
    fn a_published_icon_is_served_while_an_unpublished_one_is_not() {
        // Given
        store_icon("ZZ", FrameBytes::from(b"icon".to_vec()));

        // When / Then
        assert!(has_icon("ZZ"));
        assert!(!has_icon("ZY"));
        assert_eq!(
            published_bytes(ImageId::ConditionIcon(String::from("ZZ"))).as_deref(),
            Some(b"icon".as_slice())
        );
        assert!(published_bytes(ImageId::ConditionIcon(String::from("ZY"))).is_none());
    }

    #[test]
    fn a_provider_url_is_withheld_until_its_icon_is_published() {
        // Given: a key no other test in this process stores.
        let url = "image://condition/RU";
        let withheld = published_url(url);

        // When
        store_icon("RU", FrameBytes::from(b"icon".to_vec()));

        // Then
        assert_eq!(withheld, "");
        assert_eq!(published_url(url), url);
        assert_eq!(
            published_url("https://cdn.buienradar.nl/x/30x30/f.png"),
            "https://cdn.buienradar.nl/x/30x30/f.png"
        );
        assert_eq!(published_url(""), "");
    }

    #[test]
    fn a_stale_generation_is_never_served() {
        // Given
        begin_generation(41);
        super::replace_frames(41, vec![FrameBytes::from(b"frame".to_vec())]);

        // When / Then
        assert_eq!(
            published_bytes(ImageId::RadarFrame {
                generation: 41,
                index: 0
            })
            .as_deref(),
            Some(b"frame".as_slice())
        );
        assert!(
            published_bytes(ImageId::RadarFrame {
                generation: 40,
                index: 0
            })
            .is_none()
        );
    }
}
