//! A matching blob takes the native loader. Each stale key forces a real WGSL draw.

thrum_test_support::macos_gpu_test! {
    fn mismatched_metallib_keys_fall_back_before_drawing() {
        metal::check();
    }
}

#[cfg(target_os = "macos")]
mod metallib_cook;

#[cfg(target_os = "macos")]
mod metal {
    use std::sync::{Arc, Mutex, PoisonError};

    use thrum_render::capabilities::metallib::{TRANSLATOR, library_payload, stage_key};
    use thrum_test_support::{Scratch, target};
    use wgpu::hal::metal::{
        StageInputs, StageLibrary, StageResolver, capture_stage_translations, with_stage_resolver,
    };

    const SOURCE: &str = r"
        @vertex fn vertex_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
            let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
            return vec4(points[i], 0.0, 1.0);
        }
        @fragment fn fragment_main() -> @location(0) vec4<f32> {
            return vec4(0.0, 1.0, 0.0, 1.0);
        }";

    #[derive(Clone, Copy)]
    enum Change {
        Source,
        Translator,
        Options,
        Language,
    }

    impl Change {
        fn key(self, inputs: &StageInputs<'_>) -> String {
            let mut options = inputs.options.clone();
            let mut facts = StageInputs { ..*inputs };
            let mut translator = TRANSLATOR;
            match self {
                Self::Source => facts.wgsl = Some("different WGSL"),
                Self::Translator => translator = "wgpu 30.0.1; Naga 30.0.0",
                Self::Options => {
                    options.zero_initialize_workgroup_memory =
                        !options.zero_initialize_workgroup_memory;
                }
                Self::Language => facts.language_version ^= 1,
            }
            facts.options = &options;
            stage_key(&facts, translator).expect("native stage key")
        }
    }

    #[derive(Default)]
    struct Libraries {
        keys: Mutex<Vec<(String, Option<String>)>>,
        loaded: Mutex<Vec<bool>>,
        blobs: Vec<Vec<u8>>,
        change: Option<Change>,
    }

    impl StageResolver for Libraries {
        fn resolve(&self, inputs: &StageInputs<'_>) -> StageLibrary {
            let key = stage_key(inputs, TRANSLATOR).expect("debug source provided by wgpu");
            let wrong = self.change.map(|change| change.key(inputs));
            if let Some(wrong) = &wrong {
                assert_ne!(&key, wrong);
            }
            self.keys
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((key.clone(), wrong));
            let bytes = self
                .blobs
                .iter()
                .find_map(|blob| library_payload(blob, &key))
                .map(<[u8]>::to_vec);
            StageLibrary { key, bytes }
        }

        fn loaded(&self, _key: &str, success: bool) {
            self.loaded
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(success);
        }
    }

    #[expect(
        unsafe_code,
        reason = "the fixture only loads offline output compiled for the same validated stage"
    )]
    fn pipeline(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        libraries: &Arc<Libraries>,
    ) -> wgpu::RenderPipeline {
        let libraries: Arc<dyn StageResolver> = Arc::<Libraries>::clone(libraries);
        // SAFETY: The positive control keys the cooked blob to the exact wgpu stage.
        // A negative control changes the embedded key, so its bytes cannot reach Metal.
        unsafe {
            with_stage_resolver(libraries, || {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("keyed stage draw"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vertex_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some("fragment_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(target::FORMAT.into())],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
        }
    }

    fn draw(device: &wgpu::Device, queue: &wgpu::Queue, pipeline: &wgpu::RenderPipeline) {
        let target = target::new(device, target::Depth::Without);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.colour.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        assert!(
            target
                .read(device, queue)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [0, 255, 0, 255])
        );
    }

    fn stamp(key: &str, payload: &[u8]) -> Vec<u8> {
        let mut blob = format!("{key}\n{}\n", blake3::hash(payload).to_hex()).into_bytes();
        blob.extend(payload);
        blob
    }

    pub(super) fn check() {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        descriptor.flags |= wgpu::InstanceFlags::DEBUG;
        let instance = wgpu::Instance::new(descriptor);
        let Some(gpu) = thrum_test_support::gpu_on("mismatched_metallib_keys", &instance) else {
            return;
        };
        let (device, queue) = gpu.device(
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("keyed stage draw"),
            source: wgpu::ShaderSource::Wgsl(SOURCE.into()),
        });
        let (reference, harvest) = capture_stage_translations(|| {
            pipeline(&device, &shader, &Arc::new(Libraries::default()))
        });
        draw(&device, &queue, &reference);
        assert_eq!(harvest.len(), 2);
        let native: Vec<_> = harvest
            .iter()
            .map(|stage| {
                super::metallib_cook::cook(stage, &Scratch::new("metallib-key"))
                    .expect("cook native stage")
            })
            .collect();

        let exact = Arc::new(Libraries {
            blobs: harvest
                .iter()
                .zip(&native)
                .map(|(stage, blob)| stamp(stage.key.as_deref().expect("harvested key"), blob))
                .collect(),
            ..Default::default()
        });
        let (cooked, translations) =
            capture_stage_translations(|| pipeline(&device, &shader, &exact));
        assert!(translations.is_empty());
        assert_eq!(*exact.loaded.lock().expect("load receipts"), [true, true]);
        draw(&device, &queue, &cooked);

        for change in [
            Change::Source,
            Change::Translator,
            Change::Options,
            Change::Language,
        ] {
            let candidate = Arc::new(Libraries {
                change: Some(change),
                ..Default::default()
            });
            let probe = pipeline(&device, &shader, &candidate);
            drop(probe);
            let stale = Arc::new(Libraries {
                blobs: candidate
                    .keys
                    .lock()
                    .expect("observed keys")
                    .iter()
                    .zip(&native)
                    .map(|((_, stale), blob)| stamp(stale.as_deref().expect("changed key"), blob))
                    .collect(),
                ..Default::default()
            });
            let (fallback, translations) =
                capture_stage_translations(|| pipeline(&device, &shader, &stale));
            assert_eq!(
                translations.len(),
                2,
                "both stages fell back before drawing"
            );
            assert!(stale.loaded.lock().expect("load receipts").is_empty());
            draw(&device, &queue, &fallback);
        }
    }
}
