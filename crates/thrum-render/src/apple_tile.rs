//! Whole raw Metal recording only. No surface, queue or completion authority.
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLComputeCommandEncoder,
    MTLComputePipelineState, MTLDevice, MTLLibrary, MTLOrigin, MTLSize, MTLTexture,
};

#[derive(Clone)]
pub(crate) struct Texture(Retained<ProtocolObject<dyn MTLTexture>>);

// SAFETY: constructed only from wgpu-hal's Metal Texture (itself Send + Sync).
// This wrapper exposes no CPU contents or mutable descriptor. Native recording
// remains on the exclusive coordinator; other threads only retain/drop it at
// completion. The wgpu generation owns initialization, queue ordering and lifetime.
#[expect(
    unsafe_code,
    reason = "A retained view of wgpu-hal's Send/Sync GPU-owned Metal texture, with coordinator-only recording"
)]
// SAFETY: only the wgpu-hal-owned, immutable GPU resource retain crosses threads.
unsafe impl Send for Texture {}
// SAFETY: the same GPU-only access invariant applies to shared references; all
// native writes are ordered commands, never unsynchronized CPU texture mutation.
#[expect(
    unsafe_code,
    reason = "Completion may release immutable native texture retains on a wgpu callback thread"
)]
// SAFETY: all texture mutation is coordinator-ordered GPU work; callbacks only drop.
unsafe impl Sync for Texture {}

impl std::ops::Deref for Texture {
    type Target = ProtocolObject<dyn MTLTexture>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Clone)]
pub(crate) struct Pipeline(Retained<ProtocolObject<dyn MTLComputePipelineState>>);

#[expect(
    unsafe_code,
    reason = "Preparation-only retained Metal handles; the non-reentrant HAL guard is released before any other wgpu operation"
)]
pub(crate) fn texture(texture: &wgpu::Texture) -> Option<Texture> {
    // SAFETY: the generation retains the wgpu owner through completion and never
    // destroys or aliases this allocation. Only a balanced native retain escapes.
    let guard = unsafe { texture.as_hal::<wgpu::hal::api::Metal>() }?;
    Some(Texture(guard.raw_handle().into()))
}

pub(crate) struct Frame<'a> {
    pub(crate) source: &'a Texture,
    pub(crate) previous: &'a Texture,
    pub(crate) output: &'a Texture,
    pub(crate) next: &'a Texture,
    pub(crate) extent: [u32; 2],
    pub(crate) select_history: bool,
    pub(crate) no_commands: bool,
    pub(crate) no_tile: bool,
}

impl Pipeline {
    #[expect(
        unsafe_code,
        reason = "Native pipeline preparation through a retained generation device guard"
    )]
    pub(crate) fn prepare(device: &wgpu::Device) -> Result<Self, String> {
        // SAFETY: this guard is used only for synchronous native preparation;
        // no other wgpu call, native ownership transfer or recording occurs.
        let guard = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.ok_or("not Metal")?;
        let library = guard
            .raw_device()
            .newLibraryWithSource_options_error(
                &NSString::from_str(include_str!("apple_tile/toggle.metal")),
                None,
            )
            .map_err(|error| format!("native library: {error}"))?;
        let function = library
            .newFunctionWithName(&NSString::from_str("toggle"))
            .ok_or("missing native toggle kernel")?;
        guard
            .raw_device()
            .newComputePipelineStateWithFunction_error(&function)
            .map(Self)
            .map_err(|error| format!("native pipeline: {error}"))
    }

    #[expect(
        unsafe_code,
        reason = "Dedicated whole raw encoder; initialized retained textures with checked identical extents and no surface or queue access"
    )]
    pub(crate) fn encode(&self, encoder: &mut wgpu::CommandEncoder, frame: &Frame<'_>) {
        let record = |raw: Option<&mut wgpu::hal::metal::CommandEncoder>| {
            let buffer = raw
                .expect("registry admitted Metal")
                .raw_command_buffer()
                .expect("wgpu opened raw buffer");
            if frame.no_commands {
                return;
            }
            let blit = buffer.blitCommandEncoder().expect("Metal blit encoder");
            let [width, height] = frame.extent.map(|v| v as usize);
            let origin = MTLOrigin { x: 0, y: 0, z: 0 };
            let crop = MTLOrigin {
                x: width / 4,
                y: height / 4,
                z: 0,
            };
            for destination in [frame.output, frame.next] {
                // SAFETY: source/destination are distinct retained base-level
                // RGBA16F textures of the checked same nonempty extent.
                unsafe {
                    blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                    frame.source, 0, 0, origin, MTLSize { width, height, depth: 1 }, destination, 0, 0, origin);
                }
                if frame.select_history {
                    // SAFETY: the quarter-extent crop lies inside both textures;
                    // previous and next are distinct preinitialized allocations.
                    unsafe {
                        blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        frame.previous, 0, 0, crop, MTLSize { width: (width / 4).max(1), height: (height / 4).max(1), depth: 1 }, destination, 0, 0, crop);
                    }
                }
            }
            blit.endEncoding();
            if frame.no_tile {
                return;
            }
            let compute = buffer
                .computeCommandEncoder()
                .expect("Metal compute encoder");
            compute.setComputePipelineState(&self.0);
            for (index, texture) in [frame.previous, frame.output, frame.next]
                .into_iter()
                .enumerate()
            {
                // SAFETY: retained RGBA16F textures match the prepared kernel's
                // read/write argument indices. Previous never aliases a writer.
                unsafe {
                    compute.setTexture_atIndex(Some(texture), index);
                }
            }
            let one = MTLSize {
                width: 1,
                height: 1,
                depth: 1,
            };
            compute.dispatchThreads_threadsPerThreadgroup(one, one);
            compute.endEncoding();
        };
        // SAFETY: this encoder has no wgpu recording. The callback ends only its
        // native encoders; wgpu alone ends/submits the command buffer.
        unsafe {
            encoder.as_hal_mut::<wgpu::hal::api::Metal, _, _>(record);
        }
    }
}
