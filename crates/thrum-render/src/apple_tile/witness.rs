//! Raw copy/select diagnostic commands.
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLComputeCommandEncoder,
    MTLComputePipelineState, MTLDevice, MTLLibrary, MTLOrigin, MTLSize,
};

use super::{CommandBuffer, Texture};

#[derive(Clone)]
pub(crate) struct Pipeline(Retained<ProtocolObject<dyn MTLComputePipelineState>>);

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
                &NSString::from_str(include_str!("toggle.metal")),
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
        reason = "The witness blits and dispatches on its retained and validated texture bindings"
    )]
    pub(crate) fn encode(&self, buffer: &CommandBuffer, frame: &Frame<'_>) {
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
    }
}
