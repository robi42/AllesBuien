use cxx_qt_lib::{QColor, QImage, QImageFormat, QQmlImageProviderBaseImageType, QSize, QString};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
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
    pub fn request_image(
        self: core::pin::Pin<&mut Self>,
        _id: &QString,
        _size: *mut QSize,
        _requested_size: &QSize,
    ) -> QImage {
        let mut image = QImage::from_width_height_and_format(16, 16, QImageFormat::Format_RGBA8888);
        image.fill(&QColor::from_rgb(65, 105, 225));
        image
    }
}
