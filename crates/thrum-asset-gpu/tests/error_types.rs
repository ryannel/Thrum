//! Public error inventory and wrapped-source contract.

use std::error::Error;
use thrum_asset_gpu::{MaterialReadError, PlacementError, rows::CookedRowError};

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<MaterialReadError>();
    assert_error::<PlacementError>();
    assert_error::<CookedRowError>();
    #[cfg(feature = "draw-probe")]
    assert_error::<thrum_asset_gpu::draw_probe::DrawProbeError>();
}

#[test]
fn wrapped_loader_errors_preserve_sources() {
    let error = MaterialReadError::Io {
        path: "hook.wgsl".into(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    };
    assert!(error.source().expect("file source").is::<std::io::Error>());
    let error = CookedRowError::Product(thrum_asset::AssetError::HashMismatch);
    assert!(
        error
            .source()
            .expect("product source")
            .is::<thrum_asset::AssetError>()
    );
}

#[cfg(feature = "draw-probe")]
#[test]
fn probe_failures_preserve_product_errors_and_classify_invalid_coverage() {
    use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedSource, MeshPlacement, draw_probe::*};
    let error = DrawProbeError::Product(thrum_asset::AssetError::HashMismatch);
    assert!(
        error
            .source()
            .expect("product source")
            .is::<thrum_asset::AssetError>()
    );
    let assets = CookedAssets::new(CookedConfig::new(
        CookedSource::Store("missing-store".into()),
        vec![MeshPlacement::new("mesh", "texture")],
    ))
    .expect("valid placement");
    assert!(matches!(
        assets.with_draw_probe(Probe::Reference(Vec::new())),
        Err(DrawProbeError::PlacementCoverage)
    ));
}
