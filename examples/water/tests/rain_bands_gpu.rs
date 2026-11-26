//! Metal samples of both rain bands at exact texel centres.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn periodic_fields_match_committed_rate_history() {
    use std::sync::Arc;
    use thrum_render::runtime::DeviceSite;
    use thrum_sea::{Ambient, Profile as AmbientProfile, rain::{Bands, FIRST_LAYER, PASS_COUNT, SHARED_LAYERS}};
    use thrum_water_model::{OpenSky, RainBands, RainBandsProfile, RateHistory};

    let Some(gpu) = thrum_test_support::gpu("rain bands", Some(wgpu::Backend::Metal)) else { return; };
    let (device, queue, _) = support::device(&gpu);
    let site = DeviceSite::untracked(device.clone());
    let profile = RainBandsProfile { count: 256.0, ..RainBandsProfile::default() };
    let table = Arc::new(thrum_rain_model::RingProfile {
        count: profile.count, ..thrum_rain_model::RingProfile::default()
    }.table());
    let step = 1.0 / 60.0;
    let mut history = RateHistory::dry(step, -1, 106);
    for tick in 0..96 {
        history.commit(if tick < 60 { 50.0 / 3600.0 } else { 0.0 });
    }
    let model = RainBands::new(profile, Arc::clone(&table), history, Arc::new(OpenSky)).expect("rain model");
    let counts = [2.0, 10.0, 50.0].map(|rate| {
        let mut model = model.clone();
        let mut history = model.history().clone();
        history.rates.fill(rate / 3600.0);
        model.set_history(history).unwrap();
        (0..2).flat_map(|band| model.drops(band, 0.95))
            .map(|drop| drop.weight).sum::<f64>()
    });
    assert!(counts[0] > 0.0 && counts[1] > 4.0 * counts[0] && counts[2] > 1.4 * counts[1], "{counts:?}");
    println!("Weighted drop counts at 2, 10 and 50 mm/h: {counts:?}");
    let ambient = Ambient::with_rain(&site, AmbientProfile::default()).expect("shared cascade array");
    let texture = ambient.filtered_texture(0).expect("cascades");
    assert_eq!((texture.width(), texture.depth_or_array_layers(), texture.mip_level_count()), (512, SHARED_LAYERS, 10));
    let mut bands = Bands::in_cascades(&site, &model, texture).expect("rain planes");
    let texel = |index: u32| [(37 * index + 5) % 512, (149 * index + 31) % 512];
    let read = |bands: &mut Bands, time| {
        bands.record(&queue, &model, time);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for index in 0..PASS_COUNT {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            bands.encode(index, &mut pass);
        }
        let witnesses = [FIRST_LAYER, FIRST_LAYER + 2].map(|layer| thrum_test_support::readback::Readback::texture(
            &device, &mut encoder, bands.texture(), thrum_test_support::readback::Mip::layer(0, layer), ..
        ));
        let submitted = queue.submit([encoder.finish()]);
        let fields = witnesses.map(|read| read.wait(&device, submitted.clone()));
        (0..64).flat_map(|index| {
            let [x, y] = texel(index);
            let offset = (y * 512 + x) as usize * 8;
            let field = &fields[(index / 32) as usize];
            <[u8; 8]>::try_from(&field[offset..offset+8]).unwrap()
        }).collect::<Vec<_>>()
    };
    let expected = |drops: &[thrum_water_model::Drop], time: f64, band: usize, texel: [u32; 2]| {
        let side = profile.patches[band];
        let point = texel.map(|x| (f64::from(x) + 0.5) / 512.0 * side);
        let mut sample = [0.0; 4];
        for drop in drops.iter().filter(|drop| drop.weight > 0.0) {
            let offset: [f64; 2] = std::array::from_fn(|axis| {
                let d = point[axis] - drop.place[axis];
                d - (d / side).round() * side
            });
            let radius = offset[0].hypot(offset[1]);
            let [h, slope, dt] = table.ring(drop.class, radius, time - drop.time);
            for (sum, value) in sample.iter_mut().zip([h, slope * offset[0] / radius, slope * offset[1] / radius, dt]) {
                *sum += value * drop.weight;
            }
        }
        sample
    };
    let mut late_misses = 0;
    let tolerance = [1e-6, 9e-5, 9e-5, 9e-5];
    for time in [0.605, 57.0 * step, 1.4] {
        let raw = read(&mut bands, time);
        assert_eq!(raw, read(&mut bands, time), "equal display time");
        let scheduled = [0, 1].map(|band| model.drops(band, time));
        let previous = [0, 1].map(|band| model.drops(band, time - step / 2.0));
        for (i, bytes) in raw.as_chunks::<8>().0.iter().enumerate() {
            let band = i / 32;
            let at = texel(u32::try_from(i).unwrap());
            let model_sample = expected(&scheduled[band], time, band, at);
            let late = expected(&previous[band], time - step / 2.0, band, at);
            for j in 0..4 {
                let actual = f64::from(support::half_value(u16::from_le_bytes([bytes[2*j], bytes[2*j+1]])));
                assert!((actual - model_sample[j]).abs() <= tolerance[j],
                    "at {time}, band {band}, texel {at:?} channel {j}: GPU {actual} model {}", model_sample[j]);
                late_misses += usize::from((actual - late[j]).abs() > tolerance[j]);
            }
        }
        let counts = bands.drop_counts();
        assert!(counts.into_iter().all(|count| count > 0), "no rings at {time}");
        println!("Metal bands at {time:.3}s: {counts:?}; tolerances {tolerance:?}");
    }
    assert!(late_misses > 30, "{late_misses} half-tick differences");
    assert!(read(&mut bands, 1.4).iter().any(|byte| *byte != 0), "the present rate alone would clear the field");
    assert!(!model.active(2.0));
    assert!(read(&mut bands, 2.0).iter().all(|byte| *byte == 0));
}
}

thrum_test_support::macos_gpu_test! {
fn rain_dispatches_in_the_yard_and_expires_on_the_same_surface() {
    use thrum_render::{contribution::QualityInvalidation, runtime::FrameWorld};
    use water_scene::island::{self, Wind};
    let Some(gpu) = thrum_test_support::gpu("rain surface", Some(wgpu::Backend::Metal)) else { return; };
    support::island::with_cooked_island("rain_dispatches_in_the_yard_and_expires_on_the_same_surface", || {
        let (game, report, _, _) = island::build_rain_yard(Wind::Calm, true).unwrap();
        let mut fixture = support::Fixture::from_game(&gpu, [160, 90], game, report);
        fixture.parameters(r#"{"rain.rate":0,"camera.island":6}"#);
        fixture.warm_cooked();
        let knob = fixture.host.quality_knobs().into_iter()
            .find(|knob| knob.name.ends_with("water.rain_cadence")).expect("rain cadence");
        assert_eq!(knob.value, 1);
        assert_eq!(knob.invalidation, QualityInvalidation::EffectHistory);
        let dry = fixture.frame(1.0);
        let ran = |frame: &thrum_render::runtime::FrameSample| frame.gpu_pass_ns.as_ref().unwrap()
            .passes().iter().any(|pass| pass.name.ends_with("rain bands"));
        assert!(!ran(&dry));
        let dry_lake = fixture.image();
        fixture.parameters(r#"{"rain.rate":50,"rain.time_scale":600}"#);
        let rain = fixture.frame(1.2);
        assert!(ran(&rain));
        assert_ne!(fixture.image(), dry_lake);
        let endpoint = fixture.host.world().source();
        let fixed = fixture.host.capture_pixels().expect("capture rain endpoint");
        assert_eq!(fixture.host.world().source(), endpoint);
        assert_eq!(fixture.host.capture_pixels().expect("repeat rain endpoint"), fixed);

        assert!(fixture.report.lock().unwrap().rain_drops.unwrap().iter().all(|&n| n > 0));
        fixture.host.set_quality_knob(&knob.name, 4).unwrap();
        let pipelines = fixture.host.pipeline_creation_count();
        let first = fixture.frame(1.22);
        let second = fixture.frame(1.225);
        assert!(ran(&first), "changing cadence invalidates the previous field");
        assert_eq!(fixture.host.pipeline_creation_count(), pipelines);
        assert!(!ran(&second), "lower cadence skips an advancing display frame");
        fixture.host.set_quality_knob(&knob.name, 1).unwrap();
        fixture.parameters(r#"{"rain.rate":0,"rain.time_scale":1}"#);
        assert!(ran(&fixture.frame(1.62)), "the shower still has living rings");
        let dry = fixture.frame(2.25);
        assert!(!ran(&dry));
        assert_eq!(fixture.report.lock().unwrap().rain_drops, Some([0, 0]));
        assert_eq!(fixture.image(), dry_lake, "drained rain draws the same calm lake");
    });
}
}

thrum_test_support::macos_gpu_test! {
fn surface_gain_uses_depth_and_the_soft_roof_edge() {
    use wgpu::util::DeviceExt;
    let Some(gpu) = thrum_test_support::gpu("rain gain", Some(wgpu::Backend::Metal)) else { return; };
    let (device, queue, _) = support::device(&gpu);
    let profile = thrum_rain_model::RingProfile::default();
    let table = profile.table();
    // One sheltered one-metre column. Its east edge crosses a 20 cm
    // exposure filter. The same compiled helper is called by the surface.
    let mut columns = vec![[1.0, 0.0, f32::NEG_INFINITY, 1.0]];
    columns.extend(table.depth_response.iter().map(|&[d, g]| {
        #[expect(clippy::cast_possible_truncation, reason = "test declares production f32 records")]
        let pair = [d as f32, g as f32, 0.0, 0.0];
        pair
    }));
    let height = u32::try_from(columns.len()).unwrap();
    let responses = height - 1;
    let mut settings = bytemuck::cast_slice(&[1.28_f32, 2.1, 0.0, 0.001, 0.0, 0.0, 1.0, 0.2]).to_vec();
    settings.extend_from_slice(bytemuck::cast_slice(&[1u32, 1, responses, 1]));
    settings.extend_from_slice(bytemuck::cast_slice(&[1.0_f32, 1.0, 0.0, 0.0]));
    let buffer = |label, contents, usage| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label), contents, usage,
    });
    let uniform = buffer("rain gain settings", &settings, wgpu::BufferUsages::UNIFORM);
    let extent = wgpu::Extent3d { width: 1, height, depth_or_array_layers: 1 };
    let canopy = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rain gain columns"), size: extent,
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        canopy.as_image_copy(), bytemuck::cast_slice(&columns),
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(16), rows_per_image: None },
        extent,
    );
    let columns = canopy.create_view(&wgpu::TextureViewDescriptor::default());
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rain gain samples"), size: 24, mapped_at_creation: false,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
    });
    let shader = format!(
        "struct SurfaceRow {{ level:f32, bed:f32, swash:f32 }}\n{}\n{}",
        thrum_water::RAIN_GAIN_WGSL.replace("@group(3)", "@group(0)"),
        r"
        @group(0) @binding(22) var<storage, read_write> answers: array<f32>;
        @compute @workgroup_size(1)
        fn main() {
            answers[0]=rain_gain(vec2(0.5),SurfaceRow(0.0,-0.10,0.0));
            answers[1]=rain_gain(vec2(1.0,0.5),SurfaceRow(0.0,-0.10,0.0));
            answers[2]=rain_gain(vec2(1.5,0.5),SurfaceRow(0.0,-0.002,0.0));
            answers[3]=rain_gain(vec2(1.5,0.5),SurfaceRow(0.0,-0.10,0.0));
            answers[4]=rain_gain(vec2(1.5,0.5),SurfaceRow(0.0,-1e30,0.0));
            answers[5]=rain_gain(vec2(1.5,0.5),SurfaceRow(2.0,1.9,0.0));
        }"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("rain gain witness"), source: wgpu::ShaderSource::Wgsl(shader.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("rain gain witness"), layout: None, module: &module, entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(), cache: None,
    });
    let layout = pipeline.get_bind_group_layout(0);
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("rain gain witness"), layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding:19, resource:uniform.as_entire_binding() },
            wgpu::BindGroupEntry { binding:20, resource:wgpu::BindingResource::TextureView(&columns) },
            wgpu::BindGroupEntry { binding:22, resource:result.as_entire_binding() },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0,&group,&[]);
        pass.dispatch_workgroups(1,1,1);
    }
    let read = thrum_test_support::readback::Readback::buffer(&device,&mut encoder,&result,..);
    let data = read.wait(&device,queue.submit([encoder.finish()]));
    let samples: &[f32] = bytemuck::cast_slice(&data);
    for (actual, expected) in samples.iter().zip([0.0,0.5,table.response(0.002),1.0,1.0,1.0]) {
        assert!((f64::from(*actual)-expected).abs() < 0.000_01, "GPU {actual}, model {expected}");
    }
    println!("Metal shelter, soft edge, 2mm, deep, off-bed sea and above-shelter gains: {samples:?}");
}
}

thrum_test_support::macos_gpu_test! {
fn query_is_identical_at_both_rain_cadences() {
    use water_scene::island::{self, Wind};
    let Some(gpu) = thrum_test_support::gpu("rain query cadence", Some(wgpu::Backend::Metal)) else { return; };
    support::island::with_cooked_island("query_is_identical_at_both_rain_cadences", || {
        let sample = |cadence| {
            let (game, report, _, replies) = island::build_rain_yard(Wind::Calm, true).expect("rain yard");
            let mut fixture = support::Fixture::from_game(&gpu, [112, 64], game, report);
            fixture.parameters(r#"{"rain.rate":0,"camera.island":4}"#);
            fixture.warm_cooked();
            let knob = fixture.host.quality_knobs().into_iter()
                .find(|knob| knob.name.ends_with("water.rain_cadence")).expect("rain cadence");
            fixture.host.set_quality_knob(&knob.name, cadence).expect("cadence setting");
            fixture.parameters(r#"{"rain.rate":50,"rain.time_scale":600}"#);
            fixture.frame_without_timings(1.4);
            for step in 1..=12 {
                fixture.frame_without_timings(1.4 + f64::from(step) / 60.0);
                fixture.finish_tick_queries();
            }
            let reply = replies.lock().expect("yard replies");
            let reply = reply.as_ref().expect("delivered tick query");
            let water = reply.points[0].surface;
            assert!(water.wet && water.body == Some(3));
            assert_ne!(water.sample.slope, [0.0; 2], "the query took the ring field");
            (reply.posted, water)
        };
        assert_eq!(sample(1), sample(4));
    });
}
}

thrum_test_support::macos_gpu_test! {
fn the_moving_surface_filters_and_removes_live_rain() {
    use std::{sync::{Arc, Mutex}, time::Duration};
    use thrum_app::sky::{Camera, ContentStatus, Controls, SkyGame};
    use thrum_sea_model::{Cascade, Spectrum};
    use thrum_water::{Water, query::{BodyQuery, QueryLimits, QueryPoint}};
    use thrum_water_model::{Bed, Body, BodyKind, Chart, Map, OpenSky, RainBands, RainBandsProfile, RateHistory, Stage};
    let Some(gpu) = thrum_test_support::gpu("carried rain query and removal", Some(wgpu::Backend::Metal)) else { return; };
    let step = Duration::from_nanos(16_666_667);
    let use_tick = 2_u32;
    let time = f64::from(use_tick) * step.as_secs_f64();
    let size = [21, 13];
    let footprint: Vec<_> = (0..size[1]).flat_map(|z|
        (0..size[0]).map(move |x| x > 0 && x + 1 < size[0] && z > 0 && z + 1 < size[1])
    ).collect();
    let heights = footprint.iter().map(|&wet| if wet { 0.0 } else { 1.1 }).collect();
    let bed = Bed::new([0.0; 2], 0.5, size, heights).expect("channel bed");
    let chart = Chart {
        levels: vec![1.0; footprint.len()],
        currents: (0..footprint.len())
            .map(|i| match i % size[0] {
                0..10 => [0.5, 0.0], 10..17 => [1.5, 0.0], 17 => [0.01, 0.0], _ => [0.03, 0.0],
            }).collect(),
        footprint,
        ends: Vec::new(),
    };
    let mut map = Map::prepare(
        bed,
        vec![Body {
            name: "Varying current".into(), kind: BodyKind::Chart(chart),
            level: 0.0, stage: Stage { min: 0.0, max: 0.0, value: 0.0 }, fetch: 10.0, gains: [0.0; 4],
        }],
        thrum_water_model::Profile { margin_height: 0.3, swash_ratio: 0.0 },
    ).expect("current map");
    let profile = RainBandsProfile::default();
    let mut history = RateHistory::dry(step.as_secs_f64(), 200, 220);
    history.rates.fill(50.0 / 3600.0);
    let rain = RainBands::new(profile, Arc::new(thrum_rain_model::RingProfile::default().table()), history, Arc::new(OpenSky)).expect("rain model");
    let reference = (9.0 * 0.5 + 7.0 * 1.5 + 0.01 + 2.0 * 0.03) / 19.0;
    let stretch = reference / 0.5;
    let depart = |x| [0.5 + (x - 0.5) * stretch - reference * time, 2.0];
    let position = (60_u16..120).map(|i| [f32::from(i) / 30.0, 2.0]).find(|position| {
        let world = position.map(f64::from);
        let sample = rain.sample(depart(world[0]), time);
        let stop = time + step.as_secs_f64();
        let at_stop = [depart(world[0])[0] - reference * step.as_secs_f64(), 2.0];
        let carries_a_birth = (0..2).any(|band| {
            let period = profile.patches[band];
            rain.drops(band, stop).iter().any(|drop| {
                let [dx, dz] = std::array::from_fn(|i| {
                    let d = at_stop[i] - drop.place[i];
                    d - (d / period).round() * period
                });
                drop.time >= time && drop.weight > 0.0 && dx.hypot(dz) < 0.35
            })
        });
        carries_a_birth && sample.slope[0].abs() > 0.001
            && (sample.slope[0] - rain.sample(world, time).slope[0]).abs() > 0.0001
    }).expect("moving ring crosses a test point");
    let world = position.map(f64::from);
    let departure = depart(world[0]);
    let point = rain.sample(departure, time);
    let hull = rain.sample_footprint(departure, time, 3.0);
    // The last two nodes lie below the 0.05 m/s carriage speed floor.
    // Their phase derivative is reference / floor, but the blend still
    // changes with authored speed (0.01 to 0.03 m/s over half a metre).
    let blend_position = [8.75, 2.0];
    let travel_time = 4.0 / 0.5 + 0.5 * 3.0_f64.ln() + 3.0 / 1.5
        + 0.5 * 30.0_f64.ln() / 1.45 + 0.25 / 0.05;
    let first = rain.sample([0.5 + reference * (travel_time - time), 2.0], time);
    let second = rain.sample(blend_position.map(f64::from), time);
    let blend_term = 0.8 * (first.height - second.height);
    let slope_without_blend = 0.4 * first.slope[0] * reference / 0.05 + 0.6 * second.slope[0];
    assert!(blend_term.abs() > 1e-6);
    assert!(point.slope[0].abs() > 0.0001);
    assert!((rain.sample(world, time).slope[0] - point.slope[0]).abs() > 0.0001);
    assert_eq!(hull.slope, [0.0; 2]);
    map.set_rain(Some(rain.clone()));
    let check = |cadence| {
        let map = Arc::new(Mutex::new(map.clone()));
        let spectrum = Spectrum {
            long: Vec::new(), tail_slope_variance: 0.0,
            cascades: std::array::from_fn(|index| Cascade::new([160.0, 37.0, 9.0][index], &[]).unwrap()),
        };
        let mut profile = water_scene::profile(&spectrum);
        profile.query = Some(QueryLimits { points: 5, max_age: 3 });
        let water = Water::mapped(Arc::clone(&map), spectrum, profile).expect("moving surface");
        let mailbox = water.query_mailbox().expect("query");
        let report = water.report();
        let game = SkyGame::new(
            Controls::new(), Camera::look_at([0.5, 3.0, 2.0], [3.0, 1.0, 2.0], 60.0), |_, _| {},
        ).with_tick_duration(step).unwrap()
            .with_contribution(Box::new(water), || ContentStatus::Drawn(1));
        let mut fixture = support::Fixture::from_game(&gpu, [96, 64], game, Arc::clone(&report));
        fixture.frame_without_timings(0.0);
        let knob = fixture.host.quality_knobs().into_iter().find(|knob| knob.name.ends_with("water.rain_cadence")).unwrap();
        fixture.host.set_quality_knob(&knob.name, cadence).unwrap();
        let post = |tick, endpoint| mailbox.post(endpoint, vec![BodyQuery {
            id: 1, generation: 0, points: [position, position, position, position, blend_position].into_iter()
                .zip([0.0, 3.0, 0.128, 0.2784, 0.0]).map(|(position, radius)| QueryPoint {
                    position, radius, use_tick: u64::from(tick),
                }).collect(),
        }]);
        let queries = |fixture: &mut support::Fixture, tick| {
            post(tick, fixture.endpoint(u64::from(tick - 1)));
            let sample = fixture.frame(f64::from(tick - 1) * step.as_secs_f64());
            fixture.finish_tick_queries();
            let reply = mailbox.drain(fixture.endpoint(u64::from(tick)), step).pop().expect("delivered body");
            (sample, reply)
        };
        let ran = |sample: &thrum_render::runtime::FrameSample| sample.gpu_pass_ns.as_ref().unwrap()
            .passes().iter().any(|pass| pass.name.ends_with("rain bands"));
        let (wet, reply) = queries(&mut fixture, use_tick);
        assert!(ran(&wet));
        assert!(report.lock().unwrap().rain_drops.unwrap().iter().all(|&count| count > 0));
        let actual = [reply.points[0].surface.sample, reply.points[1].surface.sample];
        for (actual, expected) in actual.iter().zip([point, hull]) {
            assert!((actual.height - 1.0 - expected.height).abs() < 1e-6);
            assert!((actual.height_dt - (expected.height_dt - reference * expected.slope[0])).abs() < 1e-6);
            assert!((actual.velocity[1] - expected.height_dt).abs() < 1e-6);
            for (a, b) in actual.slope.iter().zip([expected.slope[0] * stretch, expected.slope[1]]) {
                assert!((a - b).abs() < 1e-6, "carried slope {a} expected {b}");
            }
        }
        let blend_reply = reply.points[4].surface.sample;
        assert!((blend_reply.slope[0] - slope_without_blend - blend_term).abs() < 1e-7,
            "blend slope {}, direct product rule {}, blend term {blend_term}",
            blend_reply.slope[0], slope_without_blend + blend_term);
        println!("world slope at varying blend {}, gradient term {blend_term:.8}", blend_reply.slope[0]);
        let stop_tick = use_tick + 1;
        let mut dry_history = RateHistory::dry(step.as_secs_f64(), i64::from(stop_tick) - 2, 220);
        dry_history.rates.fill(50.0 / 3600.0);
        let posting = {
            let mut map = map.lock().expect("moving map");
            map.set_rate_history(dry_history.clone()).expect("posting history");
            map.rain().expect("posting rain model").clone()
        };
        post(stop_tick, fixture.endpoint(u64::from(stop_tick - 1)));
        fixture.frame_without_timings(f64::from(stop_tick - 1) * step.as_secs_f64());
        fixture.finish_tick_queries();
        dry_history.commit(0.0);
        let stopped = {
            let mut map = map.lock().expect("moving map");
            map.set_rate_history(dry_history).expect("committed stop");
            map.rain().expect("committed rain model").clone()
        };
        let stop_time = f64::from(stop_tick) * step.as_secs_f64();
        let stop_departure = [departure[0] - reference * (stop_time - time), departure[1]];
        let stopped_reply = mailbox.drain(fixture.endpoint(u64::from(stop_tick)), step)
            .pop().expect("ready reply after the rate commit");
        assert!(!stopped_reply.held);
        assert_eq!(stopped_reply.posted, u64::from(stop_tick - 1));
        let use_field = stopped.query(stop_time);
        let posting_field = posting.query(stop_time);
        let mut old_history_misses = false;
        for answer in &stopped_reply.points[2..4] {
            let radius = f64::from(answer.point.radius);
            let expected = use_field.sample(stop_departure, radius);
            let old = posting_field.sample(stop_departure, radius);
            let actual = answer.surface.sample;
            assert!((actual.height - 1.0 - expected.height).abs() < 1e-8);
            assert!((actual.slope[0] - expected.slope[0] * stretch).abs() < 1e-6);
            assert!((actual.slope[1] - expected.slope[1]).abs() < 1e-6);
            assert!((actual.height_dt - (expected.height_dt - reference * expected.slope[0])).abs() < 1e-7);
            old_history_misses |= (old.height - expected.height).abs() > 1e-8
                || (old.slope[0] - expected.slope[0]).abs() > 1e-6
                || (old.slope[1] - expected.slope[1]).abs() > 1e-6
                || (old.height_dt - expected.height_dt).abs() > 1e-7;
            println!("radius {radius:.4}, use tick {stop_tick}, reply {actual:?}, prior history {old:?}");
        }
        assert!(old_history_misses, "the posting history must carry a drop that did not fall");
        map.lock().unwrap().set_rain(None);
        let (dry, removed) = queries(&mut fixture, use_tick + 2);
        assert!(!ran(&dry));
        assert_eq!(report.lock().unwrap().rain_drops, Some([0; 2]));
        for answer in removed.points {
            assert_eq!(answer.surface.sample.height, 1.0);
            assert_eq!(answer.surface.sample.slope, [0.0; 2]);
            assert_eq!(answer.surface.sample.height_dt, 0.0);
        }
        map.lock().unwrap().set_rain(Some(rain.clone()));
        let (restored, _) = queries(&mut fixture, use_tick + 3);
        assert!(ran(&restored), "restoring the model regenerates the field at any cadence");
        println!("Cadence {cadence}, departure {departure:?}, point slope {:?}, hull {:?}, point rate {:.7}; removing the model cleared the field and the tick reply.", actual[0].slope, actual[1].slope, actual[0].height_dt);
        actual
    };
    assert_eq!(check(1), check(4));
}
}

#[cfg(target_os = "macos")]
#[test]
fn the_disc_averages_the_drawn_ring_including_its_edge_fade() {
    use std::sync::Arc;
    use thrum_water_model::{OpenSky, RainBands, RainBandsProfile, RateHistory};
    let profile = RainBandsProfile {
        count: 256.0,
        ..RainBandsProfile::default()
    };
    let table = Arc::new(
        thrum_rain_model::RingProfile {
            count: profile.count,
            ..Default::default()
        }
        .table(),
    );
    let mut history = RateHistory::dry(0.001, 1999, 2000);
    history.rates.fill(50.0 / 3600.0);
    let mut rain = RainBands::new(profile, Arc::clone(&table), history, Arc::new(OpenSky)).unwrap();
    let time = 1.53;
    let point = [6.37, -0.02];
    let query = rain.query(time);
    let as_array = |sample: thrum_sea_model::Sample| {
        [
            sample.height,
            sample.slope[0],
            sample.slope[1],
            sample.height_dt,
        ]
    };
    let tolerance = [1.0e-7, 8.0e-5, 8.0e-5, 1.5e-4];
    let compare = |label, actual: [f64; 4], wanted: [f64; 4]| {
        println!("{label}: disc {actual:?}, direct {wanted:?}");
        for i in 0..4 {
            assert!(
                (actual[i] - wanted[i]).abs() < tolerance[i],
                "{label} channel {i}: {actual:?} expected {wanted:?}"
            );
        }
    };
    let differs = |a: [f64; 4], b: [f64; 4]| (0..4).any(|i| (a[i] - b[i]).abs() > tolerance[i]);
    let patch_mean = as_array(query.sample(point, 0.64));
    assert_eq!(patch_mean, as_array(query.sample([2.0, 3.0], 1.2)));
    assert_eq!(patch_mean[1..3], [0.0; 2]);
    for (radius, label) in [(0.04, "small"), (0.128, "float"), (0.278, "craft")] {
        let direct = direct_disc(&rain, point, time, radius);
        compare(label, as_array(query.sample(point, radius)), direct);
        if label == "small" {
            assert!(differs(direct, patch_mean));
            assert!(differs(direct, as_array(rain.sample(point, time))));
        }
    }

    for (point, radius, label) in [
        ([4.807, 3.249], 0.04, "small review station"),
        ([5.349, 3.443], 0.128, "float review station"),
        ([1.826, 2.182], 0.278, "craft review station"),
        (
            [5.352, 0.369],
            f64::from(0.2784_f32),
            "craft request radius",
        ),
    ] {
        compare(
            label,
            as_array(query.sample(point, radius)),
            direct_disc(&rain, point, time, radius),
        );
    }

    // Find a committed interval containing exactly one local drop, then make
    // every other interval dry. Observe that drop at age 0.8, one reach away.
    let (point, time) = (0..2)
        .flat_map(|band| rain.drops(band, 1.3))
        .filter(|drop| drop.class == table.classes.len() - 1 && drop.weight >= 1.0)
        .find_map(|drop| {
            let point = [drop.place[0] + table.reach(), drop.place[1]];
            let time = drop.time + 0.8;
            let nearby = (0..2)
                .flat_map(|band| rain.drops(band, time))
                .filter(|other| {
                    let period = profile.patches[other.band];
                    other.tick == drop.tick
                        && other.weight > 0.0
                        && periodic_offset(point, other.place, period)
                            .iter()
                            .map(|v| v * v)
                            .sum::<f64>()
                            < (table.reach() + 0.128).powi(2)
                })
                .count();
            (nearby == 1).then_some((point, time, drop.tick))
        })
        .map(|(point, time, tick)| {
            let mut history = RateHistory::dry(0.001, tick - 1, 1002);
            history.commit(50.0 / 3600.0);
            history.commit(0.0);
            rain.set_history(history).unwrap();
            (point, time)
        })
        .expect("isolated biggest-class drop");
    let mean = direct_disc(&rain, point, time, 0.128);
    compare(
        "faded edge",
        as_array(rain.sample_footprint(point, time, 0.128)),
        mean,
    );
    let patch = rain.sample_footprint(point, time, 0.64);
    assert!((mean[0] - patch.height).abs() > tolerance[0]);
}

#[cfg(target_os = "macos")]
fn periodic_offset(point: [f64; 2], place: [f64; 2], period: f64) -> [f64; 2] {
    std::array::from_fn(|axis| {
        let delta = point[axis] - place[axis];
        delta - period * (delta / period).round()
    })
}

#[cfg(target_os = "macos")]
fn direct_disc(
    rain: &thrum_water_model::RainBands,
    point: [f64; 2],
    time: f64,
    radius: f64,
) -> [f64; 4] {
    // Area-weighted polar midpoint rule about the station, reading only the
    // point table. Reject centres whose entire disc misses a ring.
    let table = rain.table();
    let drops: Vec<_> = (0..2)
        .flat_map(|band| rain.drops(band, time))
        .filter_map(|drop| {
            let offset = periodic_offset(point, drop.place, rain.profile().patches[drop.band]);
            let local = offset[0].hypot(offset[1]) < table.reach() + radius;
            (local && drop.weight > 0.0).then_some((drop, offset))
        })
        .collect();
    let mut sum = [0.0; 4];
    let radial = 256;
    let angles = 512;
    for j in 0..angles {
        let angle = std::f64::consts::TAU * (f64::from(j) + 0.5) / f64::from(angles);
        let (sin, cos) = angle.sin_cos();
        for i in 0..radial {
            let fraction = (f64::from(i) + 0.5) / f64::from(radial);
            let r = radius * fraction;
            for &(drop, offset) in &drops {
                let dx = offset[0] + r * cos;
                let dz = offset[1] + r * sin;
                let distance = (dx * dx + dz * dz).sqrt();
                let [height, slope, rate] = table.ring(drop.class, distance, time - drop.time);
                for (total, value) in
                    sum.iter_mut()
                        .zip([height, slope * dx / distance, slope * dz / distance, rate])
                {
                    *total += value * (drop.weight * 2.0 * fraction);
                }
            }
        }
    }
    sum.map(|total| total / f64::from(angles * radial))
}
