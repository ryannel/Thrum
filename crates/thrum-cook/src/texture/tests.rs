use super::*;
use std::error::Error;

#[test]
fn image_import_failure_keeps_the_real_decoder_error() {
    let error = cook_texture(b"not an image", false, Profile::Portable, Quality::Fast)
        .expect_err("invalid encoded image");
    assert!(matches!(error, CookError::ImageImport(_)));
    let source = error.source().expect("decoder source");
    assert!(matches!(
        source.downcast_ref::<image::ImageError>(),
        Some(image::ImageError::Unsupported(_))
    ));
}

#[test]
fn astc_adapter_keeps_the_upstream_code_that_does_not_implement_error() {
    let Err(code) = astcenc_rs::ConfigBuilder::new()
        .with_block_size(astcenc_rs::Extents::new(3, 3))
        .build()
    else {
        panic!("unsupported ASTC block size");
    };
    let error = CookError::TextureEncode(crate::TextureEncodeError::Astc(code));
    let source = error
        .source()
        .expect("typed encoder source")
        .downcast_ref::<crate::TextureEncodeError>()
        .expect("encoder adapter");
    assert_eq!(
        *source,
        crate::TextureEncodeError::Astc(astcenc_rs::Error::BadBlockSize)
    );
}
