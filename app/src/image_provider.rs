use std::sync::{Arc, OnceLock, RwLock};

use cxx_qt_lib::{QImage, QQmlImageProviderBaseImageType, QSize, QString};

pub(crate) type FrameBytes = Arc<[u8]>;

#[derive(Default)]
struct FrameStore {
    generation: u64,
    frames: Vec<FrameBytes>,
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
    /// Returns the generation-keyed radar frame requested by Qt.
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
        let id = id.to_string();
        let Some((generation, index)) = id.split_once('/') else {
            return QImage::default();
        };
        let (Ok(generation), Ok(index)) = (generation.parse::<u64>(), index.parse::<usize>())
        else {
            return QImage::default();
        };
        let Ok(store) = frame_store().read() else {
            return QImage::default();
        };
        if store.generation != generation {
            return QImage::default();
        }
        let image = store
            .frames
            .get(index)
            .and_then(|frame| QImage::from_data(frame, Some("PNG")))
            .unwrap_or_default();
        if !size.is_null() {
            // SAFETY: Category 8 (FFI boundary): Qt's requestImage contract supplies
            // a writable QSize output pointer when it is non-null for this call.
            unsafe { size.write(image.size()) };
        }
        image
    }
}
