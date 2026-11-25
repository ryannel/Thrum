//! A tiny second provider, driven through the real frame host and scene graph.
use super::*;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use crate::apple_tile::{self, CommandBuffer, Texture};
use crate::{
    apple_tile::{
        NativeBinding, NativeFrameValues, NativeGraph, NativeGraphInputs, NativePolicy,
        NativePreparation, NativeProvider, NativeState, PreparedProvider,
    },
    error::RenderError,
    graph::{
        Access, AccessKind, FrameGraph, GraphTextures, PassBody, PassScope, ResourceDescriptor,
        ResourceScope, TextureKind,
    },
    pass_key::PassKey,
    synthetic::{NativeOutcome, NativeSegment, NativeTwinSelection, TwinReason},
};
#[cfg(any(target_os = "macos", target_os = "ios"))]
use objc2_metal::{MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLOrigin, MTLSize};
use std::sync::atomic::AtomicUsize;

const PASS: &str = "native crop and private history";
const TWIN: &str = "portable crop and private history";
const INPUT: [u32; 2] = [4, 2];
const SURFACE: [u32; 2] = [64, 64];
const OUTPUT: [u32; 2] = [96, 64];
const OUTPUT_PIXELS: usize = OUTPUT[0] as usize * OUTPUT[1] as usize;
const ZERO: [u32; 2] = [0, 0];
const FIRST: [u32; 2] = [1, 0];
const SECOND: [u32; 2] = [2, 0];
const MARKER: [u32; 2] = [2, 0];
const FAR_MARKER: [u32; 2] = [OUTPUT[0] - 1, OUTPUT[1] - 1];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const USAGE: wgpu::TextureUsages = wgpu::TextureUsages::COPY_SRC
    .union(wgpu::TextureUsages::COPY_DST)
    .union(wgpu::TextureUsages::TEXTURE_BINDING);

struct Crop {
    source: wgpu::Texture,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
    history: [wgpu::Texture; 2],
    zero: wgpu::Texture,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    raw: Option<Raw>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    native_side: AtomicUsize,
    twin_side: AtomicUsize,
    ignore_reset: bool,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct Raw {
    source: Texture,
    output: Texture,
    history: [Texture; 2],
    zero: Texture,
}

struct Factory {
    state: Arc<NativeState>,
    crop: Arc<Crop>,
}

fn size([width, height]: [u32; 2]) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    words: &[u16],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(PASS),
        size: size(extent),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: USAGE,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(words),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(extent[0] * 8),
            rows_per_image: Some(extent[1]),
        },
        size(extent),
    );
    texture
}

impl Factory {
    fn new(capabilities: &DeviceCapabilities, queue: &wgpu::Queue, ignore_reset: bool) -> Self {
        let device = capabilities.site();
        // Unique texels make a stale rectangle and a stale history visible in
        // different parts of the output. The second row repeats the first.
        let row = [0x3c00; 4]
            .into_iter()
            .chain([0x4000; 4])
            .chain([0x4400; 4])
            .chain([0x4800; 4])
            .collect::<Vec<_>>();
        let source = texture(device, queue, INPUT, &row.repeat(2));
        let output = texture(device, queue, OUTPUT, &vec![0; OUTPUT_PIXELS * 4]);
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let history = std::array::from_fn(|_| texture(device, queue, [1, 1], &[0x3800; 4]));
        let zero = texture(device, queue, [1, 1], &[0; 4]);
        let prepared = NativePreparation::new(capabilities, NativePolicy::Recipe);
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let (prepared, raw) = {
            let mut prepared = prepared;
            let raw = prepared.prepare_metal(device, false, || {
                let retain = |texture| apple_tile::texture(texture).ok_or(RenderError::StaleTarget);
                Ok::<_, RenderError>((
                    Raw {
                        source: retain(&source)?,
                        output: retain(&output)?,
                        history: [retain(&history[0])?, retain(&history[1])?],
                        zero: retain(&zero)?,
                    },
                    Duration::ZERO,
                ))
            });
            (prepared, raw)
        };
        Self {
            state: prepared.state(PASS, TWIN, false),
            crop: Arc::new(Crop {
                source,
                output,
                output_view,
                history,
                zero,
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                raw,
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                native_side: AtomicUsize::new(0),
                twin_side: AtomicUsize::new(0),
                ignore_reset,
            }),
        }
    }
}

impl NativeProvider for Factory {
    fn declare(
        &self,
        graph: &mut FrameGraph,
        _: NativeGraphInputs<'_>,
        _: [u32; 2],
    ) -> Result<NativeGraph, RenderError> {
        let descriptor = |[width, height]: [u32; 2]| ResourceDescriptor::Texture {
            dimension: TextureKind::D2,
            width,
            height,
            layers: 1,
            mips: 1,
            samples: 1,
            format: FORMAT,
            usage: USAGE,
        };
        let source = graph.resource(
            descriptor(INPUT),
            ResourceScope::Imported { initialized: true },
        )?;
        let output = graph.resource(
            descriptor(OUTPUT),
            ResourceScope::Persistent {
                owner: 0xA120,
                epoch: 0,
                initialized: true,
            },
        )?;
        let pass = graph.declare_native(
            PassKey::NativeProvider(PASS),
            PassBody {
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![
                    Access::new(source, 0..1, AccessKind::CopyRead),
                    Access::new(output.clone(), 0..1, AccessKind::CopyWrite),
                ],
                external_side_effect: false,
            },
        );
        Ok(NativeGraph {
            pass,
            output,
            history: None,
            extent: OUTPUT,
            bindings: Vec::new(),
        })
    }

    fn bind(&self, _: &GraphTextures, _: &NativeGraph) -> Result<Arc<NativeBinding>, RenderError> {
        let crop = Arc::clone(&self.crop);
        Ok(NativeBinding::new(&self.state, crop))
    }
}

struct Copy<'a, T> {
    from: &'a T,
    to: &'a T,
    source_origin: [u32; 2],
    destination_origin: [u32; 2],
    extent: [u32; 2],
}

fn copies<'a, T>(
    source: &'a T,
    output: &'a T,
    previous: &'a T,
    next: &'a T,
    values: NativeFrameValues,
) -> [Copy<'a, T>; 4] {
    assert_eq!(values.content_extent, [2, 2]);
    assert!(values.content_origin[0] + values.content_extent[0] <= INPUT[0]);
    assert_eq!(values.content_origin[1], 0);
    [
        Copy {
            from: source,
            to: output,
            source_origin: values.content_origin,
            destination_origin: ZERO,
            extent: values.content_extent,
        },
        // Prove readback reaches the final texel, outside both the render
        // width and the first row of the provider output.
        Copy {
            from: source,
            to: output,
            source_origin: [values.content_origin[0] + 1, 1],
            destination_origin: FAR_MARKER,
            extent: [1, 1],
        },
        Copy {
            from: previous,
            to: output,
            source_origin: ZERO,
            destination_origin: MARKER,
            extent: [1, 1],
        },
        Copy {
            from: source,
            to: next,
            source_origin: values.content_origin,
            destination_origin: ZERO,
            extent: [1, 1],
        },
    ]
}

impl PreparedProvider for Crop {
    fn record_twin(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        values: NativeFrameValues,
        reset: bool,
    ) {
        let side = self.twin_side.load(Ordering::Relaxed);
        let previous = if reset && !self.ignore_reset {
            &self.zero
        } else {
            &self.history[side]
        };
        let origin = |[x, y]: [u32; 2]| wgpu::Origin3d { x, y, z: 0 };
        for copy in copies(
            &self.source,
            &self.output,
            previous,
            &self.history[1 - side],
            values,
        ) {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    origin: origin(copy.source_origin),
                    ..copy.from.as_image_copy()
                },
                wgpu::TexelCopyTextureInfo {
                    origin: origin(copy.destination_origin),
                    ..copy.to.as_image_copy()
                },
                size(copy.extent),
            );
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[expect(
        unsafe_code,
        reason = "Fixed initialized disjoint test textures and checked rectangles"
    )]
    fn record_native(&self, buffer: &CommandBuffer, values: NativeFrameValues, reset: bool) {
        let raw = self.raw.as_ref().expect("prepared Metal handles");
        // Model a native provider that has already advanced its private side
        // when the platform refuses acquisition, even without writing history.
        let side = self.native_side.fetch_xor(1, Ordering::Relaxed);
        let previous = if reset && !self.ignore_reset {
            &raw.zero
        } else {
            &raw.history[side]
        };
        let blit = buffer.blitCommandEncoder().expect("provider blit encoder");
        let origin = |[x, y]: [u32; 2]| MTLOrigin {
            x: x as usize,
            y: y as usize,
            z: 0,
        };
        for copy in copies(
            &raw.source,
            &raw.output,
            previous,
            &raw.history[1 - side],
            values,
        ) {
            let [width, height] = copy.extent.map(|value| value as usize);
            // SAFETY: copies() validates the source crop. Destinations are
            // distinct retained RGBA16F textures with room for each copy.
            unsafe {
                blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                    copy.from, 0, 0, origin(copy.source_origin),
                    MTLSize { width, height, depth: 1 }, copy.to, 0, 0, origin(copy.destination_origin),
                );
            }
        }
        blit.endEncoding();
    }

    fn submitted(&self, _: bool, twin: bool) {
        if twin {
            self.twin_side.fetch_xor(1, Ordering::Relaxed);
        }
    }

    fn output(&self) -> &wgpu::Texture {
        &self.output
    }

    fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }
}

type TestGpu = thrum_test_support::Gpu;

async fn device(
    test_gpu: &TestGpu,
    generation: u64,
    recipe: &str,
) -> (DeviceCapabilities, wgpu::Queue) {
    let (capabilities, queue) = gpu(test_gpu, generation).await;
    (
        capabilities
            .with_recipe_override(recipe)
            .expect("fixture recipe"),
        queue,
    )
}

fn install(
    host: &mut Host,
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ignore_reset: bool,
) {
    let super::super::super::super::PreparedRenderer::ThreeD(prepared) = &mut host.prepared else {
        panic!("expected 3D host");
    };
    prepared.native = Some(Arc::new(Factory::new(capabilities, queue, ignore_reset)));
    host.enable_submitted_capture_from(1, CaptureLimits::default(), CaptureSource::NativeOutput)
        .unwrap();
}

async fn fixture(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    ignore_reset: bool,
) -> Host {
    let mut host = host(
        capabilities,
        queue,
        Policy {
            empty: true,
            ..Policy::default()
        },
    )
    .await;
    install(&mut host, capabilities, queue, ignore_reset);
    host
}

fn record(
    host: &mut Host,
    origin: [u32; 2],
) -> crate::runtime::EncodedFrame<'_, StaticWorld, Policy> {
    host.source.native_content_origin = Some(origin);
    let encoded = host
        .advance(Duration::ZERO)
        .unwrap()
        .encode_scene()
        .unwrap();
    assert!(encoded.can_acquire_surface());
    encoded
}

fn fail_submission(host: &mut Host, surface: &SurfaceOutput<'_>) {
    host.source.native_content_origin = Some(ZERO);
    let mut advanced = host.advance(Duration::ZERO).unwrap();
    let mut report = advanced.report.take().unwrap();
    let owner = advanced.host.take().unwrap();
    let frame = owner.extract_serial(Duration::ZERO, 0, 0).unwrap();
    let prepared = owner
        .prepare_extracted(0, &mut report, frame)
        .unwrap()
        .unwrap();
    let recorded = owner.encode_scene_only(0, prepared).unwrap();
    assert!(
        owner
            .finish_encoded_with(
                recorded,
                &mut report,
                Some(surface),
                std::time::Instant::now(),
                || Err(FrameError::InvalidState(
                    "forced second-provider submission refusal"
                )),
            )
            .is_err()
    );
}

fn submit(
    host: &mut Host,
    surface: &SurfaceOutput<'_>,
    origin: [u32; 2],
) -> (Vec<u8>, NativeSegment) {
    let encoded = record(host, origin);
    let frame = encoded.frame_extent().unwrap();
    assert_eq!(frame.attachments, [SURFACE, SURFACE, SURFACE, OUTPUT]);
    assert_eq!(frame.source, OUTPUT);
    assert_eq!(frame.destination, SURFACE);
    assert_eq!(frame.viewport, [0, 11, 64, 42]);
    assert!(encoded.submit(Some(surface)).unwrap().surface_presented);
    let segment = host
        .slots
        .iter()
        .find_map(|slot| match &slot.state {
            crate::runtime::SlotState::Rendering(sample) => sample.native_segment.clone(),
            _ => None,
        })
        .expect("candidate reached the host mailbox");
    assert_eq!(segment.outcome(), NativeOutcome::Pending);
    assert_eq!(segment.submission, host.last_submission().unwrap());
    assert_eq!(segment.generation, host.generation);
    assert_eq!(segment.pass_name(), PASS);
    let (extent, pixels) = image_with_extent(host);
    assert_eq!(extent, OUTPUT);
    (pixels, segment)
}

fn texel(image: &[u8], x: usize) -> [u8; 8] {
    image[x * 8..(x + 1) * 8].try_into().unwrap()
}

#[test]
fn native_provider_crop_changes_inside_one_generation_and_owns_timing() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "native_provider_crop_changes_inside_one_generation_and_owns_timing",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (native, queue) = device(&test_gpu, 71, "apple-tile").await;
        let portable = native.clone().with_recipe_override("portable").unwrap();
        let drawable = output(native.device(), SURFACE);
        let surface = SurfaceOutput {
            view: &drawable,
            extent: SURFACE,
        };
        let mut metal = fixture(&native, &queue, false).await;
        let mut twin = fixture(&portable, &queue, false).await;
        assert_eq!(metal.native_provider_report().unwrap().twin, None);
        assert_eq!(
            twin.native_provider_report().unwrap().twin,
            Some(NativeTwinSelection {
                name: TWIN,
                reason: TwinReason::PortableRecipe
            })
        );
        assert!(metal.native_witness_report().is_none());
        metal.set_timing_requested(true);
        twin.set_timing_requested(true);

        let (a, a_segment) = submit(&mut metal, &surface, FIRST);
        let (b, b_segment) = submit(&mut metal, &surface, SECOND);
        let (a_twin, _) = submit(&mut twin, &surface, FIRST);
        let (b_twin, twin_segment) = submit(&mut twin, &surface, SECOND);
        assert_eq!(a, a_twin);
        assert_eq!(b, b_twin);
        assert_eq!(b.len(), OUTPUT_PIXELS * 8);
        assert_eq!(texel(&a, OUTPUT_PIXELS - 1), [0, 0x44].repeat(4).as_slice());
        assert_eq!(texel(&b, OUTPUT_PIXELS - 1), [0, 0x48].repeat(4).as_slice());
        assert_eq!(texel(&a, 1), [0, 0x44].repeat(4).as_slice());
        assert_eq!(
            texel(&b, 1),
            [0, 0x48].repeat(4).as_slice(),
            "a texel only in the second rectangle"
        );
        assert_eq!(a_segment.generation, b_segment.generation);
        assert_eq!(a_segment.submission + 1, b_segment.submission);
        assert!(matches!(a_segment.outcome(), NativeOutcome::Timed([start, end]) if end > start));
        assert!(matches!(b_segment.outcome(), NativeOutcome::Timed([start, end]) if end > start));
        assert_eq!(twin_segment.outcome(), NativeOutcome::Unmeasured);
        metal.set_timing_requested(false);
        assert_eq!(
            submit(&mut metal, &surface, SECOND).1.outcome(),
            NativeOutcome::Unmeasured
        );
        println!(
            "two full {}-byte outputs match exactly on both sides; second-only texel {:02x?}; generation {}, submissions {} and {}, pass {:?}, {:?}",
            b.len(),
            texel(&b, 1),
            b_segment.generation,
            a_segment.submission,
            b_segment.submission,
            b_segment.pass_name(),
            b_segment.outcome()
        );

        // Changing the tap stage must now use the display grid too. Compare
        // both 96-wide SDR images and require a toned marker on the last row.
        let mut sdr = Vec::new();
        for host in [&mut metal, &mut twin] {
            host.disable_submitted_capture();
            let limits = CaptureLimits {
                staging_slots: 1,
                cpu_buffers: 1,
                byte_allowance: 70_000,
            };
            host.enable_submitted_capture_from(2, limits, CaptureSource::SceneLinear)
                .expect("scene-linear uses the 64×64 scene grid");
            host.disable_submitted_capture();

            // This allowance fits 64×64 SDR, but not the provider's 96×64 SDR.
            let allocations = crate::synthetic::target_allocation::starts();
            let tight = CaptureLimits {
                byte_allowance: 40_000,
                ..limits
            };
            assert!(matches!(
                host.enable_submitted_capture_from(3, tight, CaptureSource::Sdr),
                Err(FrameError::InvalidState(
                    "capture pixel-byte allowance exhausted"
                ))
            ));
            assert_eq!(crate::synthetic::target_allocation::starts(), allocations);
            assert_eq!(
                host.slots[0].target.capture_source(),
                CaptureSource::SceneLinear
            );
            host.enable_submitted_capture_from(3, limits, CaptureSource::Sdr)
                .unwrap();
            assert!(crate::synthetic::target_allocation::starts() > allocations);
            assert!(
                record(host, SECOND)
                    .submit(Some(&surface))
                    .unwrap()
                    .surface_presented
            );
            let (extent, pixels) = image_with_extent(host);
            assert_eq!(extent, OUTPUT);
            assert_eq!(pixels.len(), OUTPUT_PIXELS * 4);
            assert!(pixels[(OUTPUT_PIXELS - 1) * 4] > 0);
            sdr.push(pixels);
        }
        assert_eq!(sdr[0], sdr[1]);
    });
}

#[test]
fn native_provider_native_discard_and_rejection_reset_private_history() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "native_provider_native_discard_and_rejection_reset_private_history",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (native, queue) = device(&test_gpu, 72, "apple-tile").await;
        let drawable = output(native.device(), SURFACE);
        let surface = SurfaceOutput {
            view: &drawable,
            extent: SURFACE,
        };
        let mut metal = fixture(&native, &queue, false).await;
        let mut control = fixture(&native, &queue, true).await;
        let reset = submit(&mut metal, &surface, SECOND).0;
        assert_ne!(
            submit(&mut control, &surface, SECOND).0,
            reset,
            "seeded private history"
        );
        for reason in ["cut", "acquisition refusal", "failed submission"] {
            for host in [&mut metal, &mut control] {
                submit(host, &surface, FIRST);
                match reason {
                    "cut" => host.invalidate_history(),
                    "acquisition refusal" => {
                        let dropped = record(host, ZERO).submit(None).unwrap();
                        assert!(dropped.dropped_view);
                        let report = host.native_provider_report().unwrap();
                        assert_eq!(report.discarded, 1);
                        assert_eq!(report.discards_kept_history, 0);
                    }
                    "failed submission" => fail_submission(host, &surface),
                    _ => unreachable!(),
                }
            }
            assert_eq!(submit(&mut metal, &surface, SECOND).0, reset, "{reason}");
            assert_ne!(
                submit(&mut control, &surface, SECOND).0,
                reset,
                "{reason}: ignored reset"
            );
        }
        assert_eq!(metal.native_provider_report().unwrap().discarded, 2);
        let previous = metal.last_submission().unwrap();
        assert!(metal.reject_native_provider());
        let (twin, segment) = submit(&mut metal, &surface, SECOND);
        assert_eq!(twin, reset);
        assert_eq!(segment.submission, previous + 1);
        assert_eq!(segment.outcome(), NativeOutcome::Unmeasured);
        let report = metal.native_provider_report().unwrap();
        assert_eq!(report.pass, PASS);
        assert_eq!(
            report.twin,
            Some(NativeTwinSelection {
                name: TWIN,
                reason: TwinReason::Rejected
            })
        );

        let (replacement, queue2) = device(&test_gpu.fresh_adapter(), 73, "apple-tile").await;
        let drawable = output(replacement.device(), SURFACE);
        let surface = SurfaceOutput {
            view: &drawable,
            extent: SURFACE,
        };
        metal.disable_submitted_capture();
        let inputs = metal.device_prepare_inputs().unwrap();
        let mut candidate = inputs
            .prepare(&replacement, &queue2, Some(super::super::surface()))
            .await
            .unwrap();
        metal.publish_device(&mut candidate).unwrap();
        install(&mut metal, &replacement, &queue2, false);
        let mut replacement_control = fixture(&replacement, &queue2, true).await;
        let (after, segment) = submit(&mut metal, &surface, SECOND);
        assert_eq!(segment.generation, 73);
        assert_eq!(after, reset);
        assert_ne!(submit(&mut replacement_control, &surface, SECOND).0, reset);
        assert_eq!(metal.native_provider_report().unwrap().resets, 1);
        println!(
            "native reset bytes match after cut, refusal, failed submission and generation change; each ignore-reset control differs; rejection report {report:?}"
        );
    });
}

#[test]
fn native_provider_twin_history_only_advances_on_submission() {
    pollster::block_on(async {
        let Some(test_gpu) = thrum_test_support::gpu(
            "native_provider_twin_history_only_advances_on_submission",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (portable, queue) = device(&test_gpu, 74, "portable").await;
        let drawable = output(portable.device(), SURFACE);
        let surface = SurfaceOutput {
            view: &drawable,
            extent: SURFACE,
        };
        let mut direct = fixture(&portable, &queue, false).await;
        let mut dropped = fixture(&portable, &queue, false).await;
        let mut reset_host = fixture(&portable, &queue, false).await;
        let reset = submit(&mut reset_host, &surface, SECOND).0;
        submit(&mut direct, &surface, FIRST);
        submit(&mut dropped, &surface, FIRST);
        let dropped_view = record(&mut dropped, ZERO).submit(None).unwrap();
        assert!(dropped_view.dropped_view);
        let report = dropped.native_provider_report().unwrap();
        assert_eq!(
            (
                report.encoded,
                report.submitted,
                report.discarded,
                report.discards_kept_history
            ),
            (2, 1, 1, 1)
        );
        let c = submit(&mut dropped, &surface, SECOND).0;
        assert_eq!(c, submit(&mut direct, &surface, SECOND).0);
        assert_ne!(c, reset);
        dropped.invalidate_history();
        assert_eq!(submit(&mut dropped, &surface, SECOND).0, reset, "cut");
        fail_submission(&mut dropped, &surface);
        assert_eq!(
            submit(&mut dropped, &surface, SECOND).0,
            reset,
            "failed submission"
        );
        assert!(record(&mut dropped, FIRST).cancel_background().dropped_view);
        assert_eq!(
            submit(&mut dropped, &surface, SECOND).0,
            reset,
            "background"
        );
        let report = dropped.native_provider_report().unwrap();
        assert_eq!((report.discarded, report.discards_kept_history), (3, 1));
        println!(
            "A, dropped B, C matches A, C and differs from reset; cut, failed submission and background match reset; {report:?}"
        );
    });
}
