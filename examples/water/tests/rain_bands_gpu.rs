//! Metal samples of both rain bands at exact texel centres.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn periodic_fields_match_committed_rate_history() {
    use std::sync::Arc;
    use thrum_render::runtime::DeviceSite;
    use thrum_sea::rain::{Bands, PASS_COUNT};
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
    let mut bands = Bands::new(&site, &model).expect("rain bands");
    let texel = |index: u32| [(37 * index + 5) % 512, (149 * index + 31) % 512];
    let read = |bands: &mut Bands, time| {
        bands.record(&queue, &model, time);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for index in 0..PASS_COUNT {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            bands.encode(index, &mut pass);
        }
        let witnesses = [0, 2].map(|layer| thrum_test_support::readback::Readback::texture(
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
    let expected = |model: &RainBands, time: f64, band: usize, texel: [u32; 2]| {
        let side = profile.patches[band];
        let point = texel.map(|x| (f64::from(x) + 0.5) / 512.0 * side);
        let mut sample = [0.0; 4];
        for drop in model.drops(band, time).into_iter().filter(|drop| drop.weight > 0.0) {
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
        for (i, bytes) in raw.as_chunks::<8>().0.iter().enumerate() {
            let band = i / 32;
            let at = texel(u32::try_from(i).unwrap());
            let model_sample = expected(&model, time, band, at);
            let late = expected(&model, time - step / 2.0, band, at);
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
            .find(|knob| knob.name.ends_with("/water.rain_cadence")).expect("rain cadence");
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
    let mut columns: Vec<[f32; 4]> = table.depth_response.iter().map(|&[d, g]| {
        #[expect(clippy::cast_possible_truncation, reason = "test declares production f32 records")]
        let pair = [d as f32, g as f32, 0.0, 0.0];
        pair
    }).collect();
    let responses = u32::try_from(columns.len()).unwrap();
    columns.push([1.0, 0.0, f32::NEG_INFINITY, 1.0]);
    let mut settings = bytemuck::cast_slice(&[1.28_f32, 2.1, 0.0, 0.001, 0.0, 0.0, 1.0, 0.2]).to_vec();
    settings.extend_from_slice(bytemuck::cast_slice(&[1u32, 1, responses, 1]));
    let buffer = |label, contents, usage| device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label), contents, usage,
    });
    let uniform = buffer("rain gain settings", &settings, wgpu::BufferUsages::UNIFORM);
    let columns = buffer("rain gain columns", bytemuck::cast_slice(&columns), wgpu::BufferUsages::STORAGE);
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
            wgpu::BindGroupEntry { binding:20, resource:columns.as_entire_binding() },
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
