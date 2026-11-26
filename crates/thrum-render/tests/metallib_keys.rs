//! Matching keys load explicit passthrough modules. Each stale key forces a WGSL draw.

thrum_test_support::macos_gpu_test! {
    fn mismatched_metallib_keys_fall_back_before_drawing() {
        metal::check();
    }
}

#[cfg(target_os = "macos")]
mod metal {
    use std::{fmt, fs, io::Write as _, path::Path};

    use thrum_render::{
        capabilities::{
            DeviceCapabilities, HardwareFeature, HardwareRequest,
            metallib::{Preparation, stage_key},
        },
        runtime::{DeviceSite, PipelineInput},
    };
    use thrum_test_support::{Scratch, target};
    use wgpu::hal::metal::{capture_stage_translations, encode_stage_records, split_stage_records};

    const SOURCE: &str = r"
        @vertex fn vertex_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
            let points = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
            return vec4(points[i], 0.0, 1.0);
        }
        @fragment fn fragment_main() -> @location(0) vec4<f32> {
            return vec4(0.0, 1.0, 0.0, 1.0);
        }";

    fn group(site: &DeviceSite) -> PipelineInput<wgpu::BindGroupLayout> {
        site.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[],
        })
    }

    fn layout(
        site: &DeviceSite,
        group: &wgpu::BindGroupLayout,
    ) -> PipelineInput<wgpu::PipelineLayout> {
        site.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(group)],
            immediate_size: 0,
        })
    }

    fn shader(site: &DeviceSite, source: &str) -> PipelineInput<wgpu::ShaderModule> {
        site.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }

    fn pipeline(site: &DeviceSite, source: &str, zero: bool) -> wgpu::RenderPipeline {
        let group = group(site);
        let layout = layout(site, &group);
        let shader = shader(site, source);
        build_pipeline(site, &layout, &shader, zero)
    }

    fn build_pipeline(
        site: &DeviceSite,
        layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        zero: bool,
    ) -> wgpu::RenderPipeline {
        let compilation = wgpu::PipelineCompilationOptions {
            zero_initialize_workgroup_memory: zero,
            ..Default::default()
        };
        site.render(&wgpu::RenderPipelineDescriptor {
            label: Some("keyed stage draw"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vertex_main"),
                compilation_options: compilation.clone(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fragment_main"),
                compilation_options: compilation,
                targets: &[Some(target::FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        })
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

    fn stamp(store: &Path, name: &str, key: &str, payload: &[u8]) {
        let mut blob = format!("{key}\n{}\n", blake3::hash(payload).to_hex()).into_bytes();
        blob.extend(payload);
        fs::write(store.join(format!("{name}.blob")), blob).expect("write stage blob");
    }

    fn translated_draw(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        source: &str,
        zero: bool,
        stages: usize,
    ) {
        let site = capabilities.site();
        let (pipeline, translated) = capture_stage_translations(|| pipeline(site, source, zero));
        assert_eq!(translated.len(), stages);
        draw(site, queue, &pipeline);
    }

    fn report(args: fmt::Arguments<'_>) {
        writeln!(std::io::stderr().lock(), "{args}").expect("write Metal lifetime evidence");
    }

    fn live_inputs(instance: &wgpu::Instance) -> [usize; 3] {
        let hub = instance
            .generate_report()
            .expect("wgpu resource report")
            .hub;
        [
            hub.shader_modules.num_kept_from_user,
            hub.bind_group_layouts.num_kept_from_user,
            hub.pipeline_layouts.num_kept_from_user,
        ]
    }

    fn discard_inputs(instance: &wgpu::Instance, site: &DeviceSite, baseline: [usize; 3]) {
        for round in 1..=3 {
            let discarded: Vec<_> = (0..10)
                .map(|_| {
                    let group = group(site);
                    let layout = layout(site, &group);
                    let shader = shader(site, SOURCE);
                    (group, layout, shader)
                })
                .collect();
            assert_eq!(live_inputs(instance), baseline.map(|n| n + 10));
            drop(discarded);
            let live = live_inputs(instance);
            report(format_args!(
                "round {round}: live shader modules, bind group layouts, pipeline layouts = {live:?}"
            ));
            assert_eq!(live, baseline);
        }
    }

    fn check_reuse(instance: &wgpu::Instance, site: &DeviceSite, queue: &wgpu::Queue) {
        let baseline = live_inputs(instance);
        let group = group(site);
        let layout_handle = layout(site, &group);
        let shader = shader(site, SOURCE);
        // The clone must still borrow the same registered handle after the original drops.
        let shared_shader = shader.clone();
        let shared_layout = layout_handle.clone();
        drop((shader, layout_handle));
        let retained = baseline.map(|n| n + 1);
        assert_eq!(live_inputs(instance), retained);
        discard_inputs(instance, site, retained);

        for layout in [shared_layout, layout(site, &group)] {
            let (pipeline, translated) =
                capture_stage_translations(|| build_pipeline(site, &layout, &shared_shader, true));
            assert!(
                translated.is_empty(),
                "live inputs must still load both cooked stages"
            );
            draw(site, queue, &pipeline);
        }
        drop((group, shared_shader));
        assert_eq!(live_inputs(instance), baseline);
        report(format_args!(
            "all inputs released; both pipelines built from reused handles drew with zero translations"
        ));
    }

    #[expect(
        unsafe_code,
        reason = "the fixture admits only its own keyed Metal cook"
    )]
    fn use_store(capabilities: &mut DeviceCapabilities, store: &Path) {
        // SAFETY: Matching products were cooked using the fixture's source and explicit layout.
        // Negative controls have mismatching keys or an empty record table and are rejected before use.
        capabilities.set_metal_preparation(Some(unsafe {
            Preparation::trusted_store(store.to_owned(), false)
        }));
    }

    fn file_options(path: &Path) -> (blake3::Hash, String, usize) {
        let options: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("options.json")).expect("stage options"))
                .expect("parse stage options");
        let pipeline = options["pipeline_key"].as_str().expect("pipeline key");
        let translator = options["translator"].as_str().expect("translator");
        let language = options["language_version"].as_u64().expect("language");
        (
            blake3::Hash::from_hex(pipeline).expect("hex pipeline key"),
            translator.to_owned(),
            usize::try_from(language).expect("Metal language value"),
        )
    }

    pub(super) fn check() {
        let Some(gpu) =
            thrum_test_support::gpu("mismatched_metallib_keys", Some(wgpu::Backend::Metal))
        else {
            return;
        };
        let profile = thrum_test_support::Profile::Native;
        let restrictions = thrum_test_support::Restriction::default();
        let request = HardwareRequest {
            feature: HardwareFeature::MetalShaderPassthrough,
            required: true,
        };
        let (_, queue, mut capabilities) = gpu
            .device_with_hardware(profile, &restrictions, 1, &[request])
            .expect("admit Metal passthrough");
        let harvest = Scratch::new("metallib-key-harvest");
        capabilities.set_metal_preparation(Some(Preparation::harvest(harvest.path().to_owned())));
        draw(
            capabilities.site(),
            &queue,
            &pipeline(capabilities.site(), SOURCE, true),
        );
        let entries: Vec<_> = fs::read_dir(harvest.path())
            .expect("harvested fixture")
            .map(|entry| entry.expect("harvested stage").path())
            .collect();
        assert_eq!(entries.len(), 2);

        let matching = Scratch::new("metallib-key-matching");
        let wrong_translator = Scratch::new("metallib-key-translator");
        let wrong_language = Scratch::new("metallib-key-language");
        let no_record = Scratch::new("metallib-key-record");
        for entry in entries {
            let key = entry
                .file_name()
                .and_then(|key| key.to_str())
                .expect("stage key");
            let (pipeline, translator, language) = file_options(&entry);
            assert_eq!(stage_key(&pipeline, &translator, language), key);
            let payload_file = entry.join("cooked.blob");
            thrum_cook::metallib::cook_stage(&entry, &payload_file).expect("offline cook");
            let payload = fs::read(payload_file).expect("read cooked blob");
            stamp(matching.path(), key, key, &payload);
            for (store, stale) in [
                (
                    &wrong_translator,
                    stage_key(&pipeline, "stale translator", language),
                ),
                (
                    &wrong_language,
                    stage_key(&pipeline, &translator, language ^ 1),
                ),
            ] {
                assert_ne!(stale, key);
                stamp(store.path(), key, &stale, &payload);
            }
            let (_, library) = split_stage_records(&payload).expect("split cooked blob");
            stamp(
                no_record.path(),
                key,
                key,
                &encode_stage_records(&[], library),
            );
        }

        capabilities.set_metal_preparation(None);
        report(format_args!("Metal preparation disabled"));
        let baseline = live_inputs(&gpu.instance);
        discard_inputs(&gpu.instance, capabilities.site(), baseline);
        use_store(&mut capabilities, matching.path());
        report(format_args!(
            "Metal preparation enabled with a matching store"
        ));
        check_reuse(&gpu.instance, capabilities.site(), &queue);
        translated_draw(&capabilities, &queue, SOURCE, true, 0);
        translated_draw(
            &capabilities,
            &queue,
            &format!("{SOURCE}\n// stale source"),
            true,
            2,
        );
        translated_draw(&capabilities, &queue, SOURCE, false, 2);
        for store in [&wrong_translator, &wrong_language, &no_record] {
            use_store(&mut capabilities, store.path());
            translated_draw(&capabilities, &queue, SOURCE, true, 2);
        }

        let (_, queue, mut unadmitted) = gpu
            .device_with_hardware(profile, &restrictions, 2, &[])
            .expect("ordinary Metal device");
        use_store(&mut unadmitted, matching.path());
        translated_draw(&unadmitted, &queue, SOURCE, true, 2);
    }
}
