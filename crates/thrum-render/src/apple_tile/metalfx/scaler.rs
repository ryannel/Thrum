//! Minimal binding for the classic MetalFX scaler from the Apple SDK.
use super::super::{CommandBuffer, NativeFrameValues, Texture};
use super::MetalFxProbe;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Bool},
    sel,
};
use objc2_metal::MTLPixelFormat;

// The iOS Simulator SDK omits MetalFX. Keep the prepared TAAU path linkable.
#[cfg(not(target_abi = "sim"))]
#[expect(
    unsafe_code,
    reason = "Link the Apple framework without declaring C functions"
)]
#[link(name = "MetalFX", kind = "framework")]
unsafe extern "C" {}

pub(super) struct Scaler {
    inner: Retained<AnyObject>,
    #[cfg(feature = "creation-counters")]
    probe: MetalFxProbe,
    #[cfg(feature = "creation-counters")]
    last_reset: Option<bool>,
}

// SAFETY: the scaler is exclusively configured and encoded under its extent
// mutex. Only Metal commands touch its GPU history, on the coordinator queue.
#[expect(
    unsafe_code,
    reason = "MetalFX ownership crosses preparation and coordinator threads"
)]
// SAFETY: the extent mutex serializes every mutable operation on the scaler.
unsafe impl Send for Scaler {}

impl Scaler {
    #[expect(
        unsafe_code,
        reason = "Checked Objective-C selectors and wgpu Metal device access"
    )]
    #[expect(
        clippy::multiple_unsafe_ops_per_block,
        reason = "One descriptor configuration uses one SDK lifetime and ABI contract"
    )]
    pub(super) fn prepare(
        device: &wgpu::Device,
        [width, height]: [u32; 2],
        probe: MetalFxProbe,
    ) -> Result<Self, ()> {
        #[cfg(not(feature = "creation-counters"))]
        let _ = probe;
        if cfg!(target_abi = "sim") {
            return Err(());
        }
        let class = AnyClass::get(c"MTLFXTemporalScalerDescriptor").ok_or(())?;
        // SAFETY: the guard is used only to access the device, with no wgpu
        // operation or exported borrowed raw pointer during scaler preparation.
        let raw = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.ok_or(())?;
        let metal = raw.raw_device();
        // SAFETY: the types and ownership follow the SDK's descriptor interface.
        unsafe {
            let supported: Bool = msg_send![class, supportsDevice: &**metal];
            if !supported.as_bool() {
                return Err(());
            }
            let descriptor: Retained<AnyObject> = msg_send![class, new];
            let reactive: Bool =
                msg_send![&*descriptor, respondsToSelector: sel!(setReactiveMaskTextureEnabled:)];
            if !reactive.as_bool() {
                return Err(());
            }
            let width = width as usize;
            let height = height as usize;
            let _: () = msg_send![&*descriptor, setInputWidth: width];
            let _: () = msg_send![&*descriptor, setInputHeight: height];
            let _: () = msg_send![&*descriptor, setOutputWidth: width];
            let _: () = msg_send![&*descriptor, setOutputHeight: height];
            let _: () = msg_send![&*descriptor, setColorTextureFormat: MTLPixelFormat::RGBA16Float];
            let _: () =
                msg_send![&*descriptor, setOutputTextureFormat: MTLPixelFormat::RGBA16Float];
            let _: () =
                msg_send![&*descriptor, setDepthTextureFormat: MTLPixelFormat::Depth32Float];
            let _: () = msg_send![&*descriptor, setMotionTextureFormat: MTLPixelFormat::RG16Float];
            let _: () =
                msg_send![&*descriptor, setReactiveMaskTextureFormat: MTLPixelFormat::R8Unorm];
            let _: () = msg_send![&*descriptor, setReactiveMaskTextureEnabled: Bool::YES];
            let _: () = msg_send![&*descriptor, setRequiresSynchronousInitialization: Bool::YES];
            let _: () = msg_send![&*descriptor, setInputContentPropertiesEnabled: Bool::YES];
            let _: () = msg_send![&*descriptor, setInputContentMinScale: 1.0_f32];
            let _: () = msg_send![&*descriptor, setInputContentMaxScale: 2.0_f32];
            let scaler: Option<Retained<AnyObject>> =
                msg_send![&*descriptor, newTemporalScalerWithDevice: &**metal];
            scaler
                .map(|inner| Self {
                    inner,
                    #[cfg(feature = "creation-counters")]
                    probe,
                    #[cfg(feature = "creation-counters")]
                    last_reset: None,
                })
                .ok_or(())
        }
    }

    #[cfg(all(target_os = "macos", feature = "creation-counters"))]
    pub(super) fn last_reset(&self) -> Option<bool> {
        self.last_reset
    }

    #[expect(
        unsafe_code,
        reason = "Raw MetalFX selectors encode only on the supplied buffer"
    )]
    #[expect(
        clippy::multiple_unsafe_ops_per_block,
        reason = "One scaler configuration and encode use the same retained frames and buffer"
    )]
    pub(super) fn encode(
        &mut self,
        buffer: &CommandBuffer,
        [colour, depth, motion, reactive, exposure]: &[Texture; 5],
        output: &Texture,
        values: NativeFrameValues,
        reset: bool,
    ) {
        let [width, height] = values.content_extent.map(|value| value as usize);
        let [x, y] = values.raster_jitter;
        let (motion_scale, jitter_x, jitter_y) = (1.0_f32, x, y);
        #[cfg(feature = "creation-counters")]
        let (motion_scale, jitter_x, jitter_y, reset) = {
            let motion_scale = if self.probe == MetalFxProbe::ReverseMotion {
                -motion_scale
            } else {
                motion_scale
            };
            let jitter_x = if self.probe == MetalFxProbe::ReverseJitterX {
                -jitter_x
            } else {
                jitter_x
            };
            let jitter_y = if self.probe == MetalFxProbe::ReverseJitterY {
                -jitter_y
            } else {
                jitter_y
            };
            let skip_reset = match self.probe {
                MetalFxProbe::SkipReset => self.last_reset.is_some(),
                MetalFxProbe::SkipFirstReset => self.last_reset.is_none(),
                _ => false,
            };
            let reset = reset && !skip_reset;
            (motion_scale, jitter_x, jitter_y, reset)
        };
        let scaler = &*self.inner;
        // SAFETY: all objects have their declared Metal protocol and remain
        // alive through completion. The segment alone submits the buffer.
        unsafe {
            let _: () = msg_send![scaler, setColorTexture: &**colour];
            let _: () = msg_send![scaler, setDepthTexture: &**depth];
            let _: () = msg_send![scaler, setMotionTexture: &**motion];
            let _: () = msg_send![scaler, setReactiveMaskTexture: &**reactive];
            let _: () = msg_send![scaler, setExposureTexture: &**exposure];
            let _: () = msg_send![scaler, setOutputTexture: &**output];
            let _: () = msg_send![scaler, setInputContentWidth: width];
            let _: () = msg_send![scaler, setInputContentHeight: height];
            let _: () = msg_send![scaler, setMotionVectorScaleX: motion_scale];
            let _: () = msg_send![scaler, setMotionVectorScaleY: motion_scale];
            let _: () = msg_send![scaler, setJitterOffsetX: jitter_x];
            let _: () = msg_send![scaler, setJitterOffsetY: jitter_y];
            let _: () = msg_send![scaler, setPreExposure: 1.0_f32];
            let _: () = msg_send![scaler, setDepthReversed: Bool::YES];
            let reset = Bool::new(reset);
            let _: () = msg_send![scaler, setReset: reset];
            let _: () = msg_send![scaler, encodeToCommandBuffer: buffer];
            #[cfg(feature = "creation-counters")]
            {
                self.last_reset = Some(reset.as_bool());
            }
        }
    }
}
