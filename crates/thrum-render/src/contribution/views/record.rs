//! Single CPU/WGSL authority for a secondary view's reconstruction record.

/// Column-major matrices and world-space clipping for a view of this frame.
/// This is not a presented-frame contract and carries no motion or history.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SecondaryViewRecord {
    /// Raster transform, with the presenting frame's jitter at this view's extent.
    pub view_proj: [[f32; 4]; 4],
    /// Inverse raster transform, including its jitter.
    pub inverse_view_proj: [[f32; 4]; 4],
    /// Transform without jitter, for correspondence with world points.
    pub unjittered_view_proj: [[f32; 4]; 4],
    /// Inverse of the unjittered transform.
    pub inverse_unjittered_view_proj: [[f32; 4]; 4],
    /// Camera xyz in world metres; w is reserved.
    pub camera: [f32; 4],
    /// Viewport width/height, then physical attachment width/height, in pixels.
    pub extent: [f32; 4],
    /// Effective unit-normal plane after the 5 cm clamp; nonnegative side is kept.
    pub clip_plane: [f32; 4],
    /// Clip enabled, air from plane, least instance size in pixels, reserved.
    pub options: [f32; 4],
}

#[cfg(test)]
mod tests {
    use super::SecondaryViewRecord;

    #[test]
    fn record_matches_wgsl() {
        let module =
            wgpu::naga::front::wgsl::parse_str(include_str!("record.wgsl")).expect("record WGSL");
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("SecondaryViewRecord"))
            .expect("record type");
        let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("struct")
        };
        assert_eq!(*span as usize, size_of::<SecondaryViewRecord>());
        let offsets = [
            std::mem::offset_of!(SecondaryViewRecord, view_proj),
            std::mem::offset_of!(SecondaryViewRecord, inverse_view_proj),
            std::mem::offset_of!(SecondaryViewRecord, unjittered_view_proj),
            std::mem::offset_of!(SecondaryViewRecord, inverse_unjittered_view_proj),
            std::mem::offset_of!(SecondaryViewRecord, camera),
            std::mem::offset_of!(SecondaryViewRecord, extent),
            std::mem::offset_of!(SecondaryViewRecord, clip_plane),
            std::mem::offset_of!(SecondaryViewRecord, options),
        ];
        assert_eq!(members.len(), offsets.len());
        for (member, offset) in members.iter().zip(offsets) {
            assert_eq!(member.offset as usize, offset);
        }
    }
}
