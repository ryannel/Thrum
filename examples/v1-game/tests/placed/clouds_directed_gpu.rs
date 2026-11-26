//! Check game-directed placement against the submitted GPU map.
#![cfg(feature = "proving")]

#[cfg(target_os = "macos")]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn directed_weather_map_matches_the_games_query() {
        run();
    }
}

#[cfg(target_os = "macos")]
fn run() {
    use v1_game::GameHost as _;
    let Some(gpu) = thrum_test_support::open(
        "game directed clouds",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(true);
    // Confirm a changed stack can still serve the same game's initial shape state.
    assert_eq!(fixture.replace_stack(), 2);
    let mut host = pollster::block_on(fixture.host(&gpu));
    let mut frames = fixture::Frames::new(&gpu);
    let report = frames.placed_submitted(&mut host, |report| {
        report.volume_instances == 1 && report.volume_refusals == 1
    });
    assert_eq!(report.directed_refusals, 1);
    assert_eq!(report.invalidations, 1);
    let (texture, origin, state) = host
        .cloud_placed_input()
        .expect("Shapes")
        .placed_map()
        .expect("submitted map");
    assert_eq!(
        state
            .shapes()
            .iter()
            .map(|shape| shape.id)
            .collect::<Vec<_>>(),
        [2, 1]
    );
    assert!(
        state
            .shapes()
            .iter()
            .all(|shape| shape.target.to_bits() == 1.0_f64.to_bits())
    );
    let raw = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        &texture,
        thrum_test_support::readback::Mip::layer(0, 0),
        ..,
    );
    let got = raw.as_chunks::<16>().0.iter().map(|row| {
        let words = row.as_chunks::<4>().0;
        std::array::from_fn::<_, 4, _>(|i| f32::from_le_bytes(words[i]))
    });
    let width = texture.width() as usize;
    let profile = v1_game::clouds::shade_config();
    let field = thrum_cloud_model::Field::new(profile.seed, profile.model, profile.types)
        .expect("game weather field");
    let mut volumes = [0, 0];
    let mut clear = 0;
    let mut inside = 0;
    for (at, [slot, weight, tau, _]) in got.enumerate() {
        let x = origin[0]
            + (f64::from(u32::try_from(at % width).expect("texel")) + 0.5) * fixture::TEXEL;
        let z = origin[1]
            + (f64::from(u32::try_from(at / width).expect("texel")) + 0.5) * fixture::TEXEL;
        let expected = field.query([x, 0.0, z], &state).optical_depth;
        if expected == 0.0 {
            assert_eq!([slot.to_bits(), tau.to_bits()], [0; 2], "{x}, {z}");
            clear += 1;
        } else {
            assert!(slot > 0.0, "{x}, {z}");
            assert!(
                (f64::from(tau) - expected).abs() < 0.001,
                "{x}, {z}: {tau} vs {expected}"
            );
            if expected > 1.0 {
                assert!(
                    (2.0 * f64::from(tau) - expected).abs() > 0.001,
                    "double density must fail"
                );
                inside += 1;
            }
            volumes[usize::from(weight > 0.0)] += 1;
        }
    }
    assert!(
        clear > 100 && inside > 4 && volumes.iter().all(|&count| count > 4),
        "clear={clear}, deep={inside}, column/volume={volumes:?}"
    );
}

thrum_test_support::macos_gpu_test! {
    fn clear_pixels_match_the_same_view_without_clouds() {
        clear_pixels();
    }
}

#[cfg(target_os = "macos")]
fn clear_pixels() {
    use v1_game::{GameHost as _, GameSetup, SceneCamera};
    let Some(gpu) = thrum_test_support::open(
        "directed clear pixels",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let mut fixture = fixture::Fixture::new(true);
    assert_eq!(fixture.replace_stack(), 2);
    let mut placed = pollster::block_on(fixture.host(&gpu));
    let mut absent = pollster::block_on(GameSetup::default().prepare(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        fixture::frame_config(),
    ))
    .expect("cloud-free game");
    let camera = SceneCamera::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 1700.0, -1700.0],
        0.75,
        1.0,
        200_000.0,
    )
    .expect("sky-facing camera");
    absent.set_scene_camera(camera).expect("matched camera");
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut placed, |report| {
        report.volume_instances == 1 && !report.history_invalid
    });
    for host in [&mut placed, &mut absent] {
        host.set_time_of_day(0.5).expect("matched noon");
    }
    let actual = placed.capture_pixels().expect("placed image");
    let control = absent.capture_pixels().expect("cloud-free image");
    frames.retire(&mut placed);
    frames.retire(&mut absent);

    let sky = placed.cloud_sky_products().expect("actual trace");
    let texture = &sky.fresh_transport;
    let raw = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        texture,
        thrum_test_support::readback::Mip::level(0),
        ..,
    );
    let width = texture.width() as usize;
    let height = texture.height() as usize;
    let mut clear = 0;
    let mut changed = 0;
    for (index, (a, b)) in actual
        .as_chunks::<4>()
        .0
        .iter()
        .zip(control.as_chunks::<4>().0)
        .enumerate()
    {
        let x = (index % fixture::EXTENT[0] as usize) / 2;
        let y = (index / fixture::EXTENT[0] as usize) / 2;
        let sky_only = (y.saturating_sub(1)..=(y + 1).min(height - 1)).all(|py| {
            (x.saturating_sub(1)..=(x + 1).min(width - 1)).all(|px| {
                // Binary16 transmittance is exactly one wherever the march found no density.
                let alpha = (py * width + px) * 8 + 6;
                raw[alpha..alpha + 2] == 0x3c00_u16.to_le_bytes()
            })
        });
        if sky_only {
            assert_eq!(a, b, "clear source and neighbouring samples at ({x}, {y})");
            clear += 1;
        } else if a != b {
            changed += 1;
        }
    }
    assert!(
        clear > 100 && changed > 10,
        "clear={clear}, changed={changed}"
    );
}

thrum_test_support::macos_gpu_test! {
    fn low_deck_keeps_placed_extinction_in_the_trace_and_overlay() {
        low_deck_placed();
    }
}

#[cfg(target_os = "macos")]
fn low_deck_placed() {
    use thrum_cloud_model::{Density, Field};
    use thrum_render::contribution::{
        ContributionFrame, ContributionPasses, SceneContribution, SceneSurface,
    };
    use v1_game::{CloudSelection, GameHost as _, GroundWeather, SceneCamera};

    struct OverlayProbe(std::sync::Arc<std::sync::Mutex<Option<wgpu::Texture>>>);
    impl SceneContribution for OverlayProbe {
        fn passes(&self) -> ContributionPasses {
            ContributionPasses::default()
        }

        fn rebuild(
            &mut self,
            surface: &SceneSurface<'_>,
        ) -> Result<(), thrum_render::runtime::FrameError> {
            *self.0.lock().unwrap() = surface.sky_overlay_target();
            Ok(())
        }

        fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
    }

    let Some(gpu) = thrum_test_support::open(
        "low deck with placed clouds",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let fixture = fixture::Fixture::with_coverage(true, &[[20.0, -1100.0]], [0.0; 2], 0.78);
    let selection = CloudSelection::Ground(GroundWeather::LowDeck);
    let mut host = pollster::block_on(fixture.host_with_clouds(&gpu, selection));
    let overlay = std::sync::Arc::default();
    host.set_game_contribution(Box::new(OverlayProbe(std::sync::Arc::clone(&overlay))))
        .expect("lend the live overlay");
    let overlay = overlay.lock().unwrap().take().expect("world-light overlay");
    let cadence = host
        .quality_knobs()
        .into_iter()
        .find(|knob| knob.name.ends_with("cloud sky overlay cadence"))
        .expect("overlay cadence");
    host.set_game_quality_knob(&cadence.name, 1)
        .expect("refresh the whole overlay");
    let camera = SceneCamera::perspective(
        [20.0, 310.0, 0.0],
        [20.0, 310.0, -1100.0],
        0.30,
        1.0,
        4000.0,
    )
    .expect("look above the deck toward the placed slab");
    host.set_scene_camera(camera)
        .expect("camera above the deck");
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut host, |report| {
        report.volume_instances == 1 && !report.history_invalid
    });
    let (_, _, state) = host
        .cloud_placed_input()
        .unwrap()
        .placed_map()
        .expect("placed map");
    let mut profile = v1_game::clouds::shade_config();
    // Reconstruct the authored V1 low-deck band for the headless point query.
    let kind = thrum_cloud_model::Type {
        base: 125.0,
        depth: [108.0, 119.0],
        liquid_water: 0.000_22,
        droplet_radius: 5e-6,
        profile: [0.23, 0.22],
        ..profile.types[0]
    };
    profile.types.fill(kind);
    let field = Field::new(profile.seed, profile.model, profile.types).expect("low deck field");
    assert!(
        field.density(
            [80.0, 340.0, -1100.0],
            &state,
            Density::Uneroded,
            |_| None,
            0.0
        ) > 0.0,
        "query missed the slab above the procedural layer"
    );

    let sky = host.cloud_sky_products().expect("submitted trace");
    let raw = thrum_test_support::readback::texture(
        &gpu.device,
        &gpu.queue,
        &sky.fresh_transport,
        thrum_test_support::readback::Mip::layer(0, 0),
        ..,
    );
    let width = sky.fresh_transport.width() as usize;
    assert!(
        raw.as_chunks::<8>().0[..width * 8]
            .iter()
            .any(|pixel| u16::from_le_bytes([pixel[6], pixel[7]]) < 0x3b00),
        "the upper part of the frame must see placed extinction"
    );
    assert!(
        overlay_transmittance(&gpu, &overlay) < 0.999,
        "the sky overlay must see placed extinction above the deck"
    );
}

#[cfg(target_os = "macos")]
fn overlay_transmittance(gpu: &thrum_test_support::Opened, source: &wgpu::Texture) -> f32 {
    let device = &gpu.device;
    let read = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("placed cloud overlay witness"),
        source: wgpu::ShaderSource::Wgsl(
            r"
            @group(0) @binding(0) var overlay: texture_2d_array<f32>;
            @group(0) @binding(1) var<storage, read_write> transmittance: f32;
            @compute @workgroup_size(1) fn main() {
                let size = vec2<i32>(textureDimensions(overlay));
                // The upper half of the -Z face cannot cross the lower deck.
                var closest = 1.0;
                for (var y = 0; y < size.y / 2; y++) {
                    for (var x = 0; x < size.x; x++) {
                        closest = min(closest, textureLoad(overlay, vec2(x, y), 5, 0).a);
                    }
                }
                transmittance = closest;
            }"
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("read submitted cloud overlay"),
        layout: None,
        module: &read,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let view = source.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let count = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("overlay minimum transmittance"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("cloud overlay witness inputs"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: count.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    gpu.queue.submit([encoder.finish()]);
    let result = thrum_test_support::readback::buffer(device, &gpu.queue, &count, ..);
    let minimum = f32::from_le_bytes(result.try_into().unwrap());
    println!("upper -Z overlay minimum transmittance {minimum}");
    minimum
}
