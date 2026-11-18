use super::{GpuRect, GpuRow, GpuView, SHADER};
use std::mem::{offset_of, size_of};
use wgpu::naga::{TypeInner, front::wgsl};

#[test]
fn sprite_abi_matches_wgsl_layout() {
    let module = wgsl::parse_str(SHADER).expect("valid sprite WGSL");
    for (name, size, offsets) in [
        (
            "View",
            size_of::<GpuView>(),
            &[
                offset_of!(GpuView, clip_per_world),
                offset_of!(GpuView, padding),
            ][..],
        ),
        (
            "Row",
            size_of::<GpuRow>(),
            &[
                offset_of!(GpuRow, pivot),
                offset_of!(GpuRow, right),
                offset_of!(GpuRow, down),
                offset_of!(GpuRow, frame),
                offset_of!(GpuRow, padding),
                offset_of!(GpuRow, tint),
            ][..],
        ),
        (
            "Frame",
            size_of::<GpuRect>(),
            &[offset_of!(GpuRect, origin), offset_of!(GpuRect, size)][..],
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
