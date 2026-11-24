use super::{GpuRect, GpuRow, GpuView};
use std::mem::{offset_of, size_of};
use wgpu::naga::{TypeInner, front::wgsl};

#[test]
fn sprite_abi_matches_wgsl_layout() {
    let module =
        wgsl::parse_str(&crate::shaders::Shader::Sprite.source()).expect("valid sprite WGSL");
    for (name, size, offsets) in [
        (
            "SpriteView",
            size_of::<GpuView>(),
            &[
                offset_of!(GpuView, clip_per_world),
                offset_of!(GpuView, centre_high),
                offset_of!(GpuView, centre_low),
                offset_of!(GpuView, alpha),
                offset_of!(GpuView, pixels_per_unit),
            ][..],
        ),
        (
            "SpriteRow",
            size_of::<GpuRow>(),
            &[
                offset_of!(GpuRow, current),
                offset_of!(GpuRow, previous),
                offset_of!(GpuRow, scales),
                offset_of!(GpuRow, rotation),
                offset_of!(GpuRow, rotation_delta),
                offset_of!(GpuRow, frame),
                offset_of!(GpuRow, sheet),
                offset_of!(GpuRow, tint),
                offset_of!(GpuRow, flash),
                offset_of!(GpuRow, additive),
                offset_of!(GpuRow, padding0),
                offset_of!(GpuRow, padding1),
                offset_of!(GpuRow, padding2),
            ][..],
        ),
        (
            "SpriteFrameRecord",
            size_of::<GpuRect>(),
            &[
                offset_of!(GpuRect, origin),
                offset_of!(GpuRect, size),
                offset_of!(GpuRect, page),
                offset_of!(GpuRect, kind),
                offset_of!(GpuRect, tile),
                offset_of!(GpuRect, padding),
            ][..],
        ),
    ] {
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .expect("sprite ABI struct");
        let TypeInner::Struct { members, span } = &ty.inner else {
            panic!("sprite ABI type must be a struct");
        };
        assert_eq!(*span as usize, size, "{name} size");
        assert_eq!(
            members
                .iter()
                .map(|member| member.offset as usize)
                .collect::<Vec<_>>(),
            offsets,
            "{name} member offsets"
        );
    }
}

#[test]
fn sprite_row_admission_reports_the_actual_refusal() {
    use super::{RenderError, row_storage};

    let row_bytes = size_of::<GpuRow>() as u64;
    assert_eq!(row_storage(0, 1, row_bytes).unwrap(), (1, row_bytes));

    let empty = row_storage(0, 0, row_bytes).unwrap_err();
    assert!(matches!(empty, RenderError::EmptySpriteRowCapacity));
    assert_eq!(empty.to_string(), "Sprite scene requires at least one row");

    let storage = row_storage(1, 1, row_bytes).unwrap_err();
    assert!(matches!(storage, RenderError::SpriteRowStorageLimit));
    assert_eq!(
        storage.to_string(),
        "Sprite rows exceed device storage capacity"
    );

    let count = row_storage(u32::MAX, 1, u64::MAX).unwrap_err();
    assert!(matches!(count, RenderError::SpriteRowCapacity));
    assert_eq!(
        count.to_string(),
        "Sprite row count exceeds the supported range"
    );
}

#[test]
fn covered_pixels_include_the_pixel_filter_margin() {
    use super::{coverage_at, pack};
    use crate::sprite::{
        EmbeddedSheet, SpriteRect, SpriteRow, SpriteSampling, SpriteScene, SpriteView,
    };

    let mut scene = SpriteScene::new(
        EmbeddedSheet {
            extent: [32; 2],
            rgba: vec![255; 32 * 32 * 4].into(),
            frames: [SpriteRect {
                origin: [0; 2],
                size: [32; 2],
            }]
            .into(),
            texels_per_unit: 1.0,
        },
        1,
    );
    let row = SpriteRow::new(0, [0.0; 2]);
    let gpu = pack(&scene, &[0], row, row.into());
    let view = SpriteView {
        centre: [0.0; 2],
        pixels_per_unit: 1.5,
    };
    assert_eq!(
        coverage_at(&scene, &gpu, 0, view, [128; 2], 1.0, true),
        Some(52.0 * 52.0)
    );
    scene.sheets[0].sampling = SpriteSampling::Smooth;
    assert_eq!(
        coverage_at(&scene, &gpu, 0, view, [128; 2], 1.0, true),
        Some(51.0 * 51.0)
    );
    scene.sheets[0].tiles.push(0);
    assert_eq!(
        coverage_at(&scene, &gpu, 0, view, [128; 2], 1.0, true),
        Some(48.0 * 48.0)
    );
}
