//! Capture-off measurement of the selected real game, paired by the Python driver.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use thrum_fixtures::mixed_field::day_cycle::DayCycle;
use thrum_render::capabilities::{DeviceProfile, DeviceRecipe, OpenedDevice};
use thrum_render::runtime::{FrameConfig, FrameWorld as _};
use v1_game::GameHost as _;

fn main() -> Result<(), String> {
    let mut low_sun = None;
    let mut noon = false;
    let mut profile = DeviceProfile::Native;
    let mut store = None;
    let mut library = false;
    let mut mist_day = false;
    let mut clouds_off_rate = 1.0;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--low-sun" => low_sun = Some(1.0),
            "--dawn" => low_sun = Some(-1.0),
            "--noon" => noon = true,
            "--mist-day" => mist_day = true,
            "--clouds-off-solar-rate" => {
                let value = args.next().ok_or("missing clouds-off solar rate")?;
                clouds_off_rate = value.parse::<f64>().map_err(|error| error.to_string())?;
                if !(1.0..=60.0).contains(&clouds_off_rate) {
                    return Err("clouds-off solar rate must be between 1 and 60".into());
                }
            }
            "--device-profile" => {
                profile = match args.next().as_deref() {
                    Some("portable") => DeviceProfile::Portable,
                    Some("native") => DeviceProfile::Native,
                    _ => return Err("--device-profile expects portable or native".into()),
                };
            }
            "--placed-store" => {
                store = Some(args.next().ok_or("--placed-store needs a path")?.into());
            }
            "--library" => library = true,
            _ => return Err(format!("unrecognised cloud cost option {arg}")),
        }
    }
    if u8::from(low_sun.is_some()) + u8::from(noon) + u8::from(mist_day) > 1 {
        return Err("choose one of the low Sun, noon or mist day views".into());
    }
    if library && (low_sun != Some(1.0) || store.is_none()) {
        return Err("--library needs --low-sun and --placed-store".into());
    }
    pollster::block_on(measure(
        low_sun,
        noon,
        profile,
        store,
        library,
        mist_day.then_some(clouds_off_rate),
    ))
}

#[expect(
    clippy::print_stdout,
    reason = "measurement executable emits its one machine-readable record"
)]
async fn measure(
    low_sun: Option<f32>,
    noon: bool,
    profile: DeviceProfile,
    store: Option<std::path::PathBuf>,
    library: bool,
    mist_day: Option<f64>,
) -> Result<(), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let OpenedDevice {
        adapter,
        device,
        queue,
        capabilities: caps,
        ..
    } = DeviceRecipe {
        profile,
        ..DeviceRecipe::default()
    }
    .open(&instance, None)
    .await
    .map_err(|error| error.to_string())?;
    let setup = v1_game::selected_setup()?;
    let config = FrameConfig {
        render_extent: [2560, 1440],
        surface: None,
    };
    let placed = store
        .map(|path| {
            let sky = v1_game::PlacedSky::new(path.into(), false)?;
            if library {
                sky.with_capture_placement("library-cost")
            } else {
                Ok(sky)
            }
        })
        .transpose()
        .map_err(|error| error.to_string())?;
    let has_shapes = placed.is_some();
    let mut host = if let Some(placed) = placed {
        setup
            .prepare_placed(&device, &queue, &caps, config, placed)
            .await
            .map_err(|error| error.to_string())?
    } else {
        setup.prepare(&device, &queue, &caps, config).await?
    };
    let content = v1_game::attach_cooked_scene(&mut host)?;
    host.set_timing_requested(true);
    let mut view_angles = if let Some(side) = low_sun.or(mist_day.map(|_| -1.0)) {
        let elevation = 10.0_f32.to_radians();
        let hour_angle = side * (elevation.sin() * std::f32::consts::SQRT_2).acos();
        let solar_time = 0.5 + hour_angle / std::f32::consts::TAU;
        let sun = DayCycle::sample(0, 0.0, hour_angle / std::f32::consts::TAU).toward_sun;
        let yaw = sun[0].atan2(-sun[2]);
        host.set_time_of_day(solar_time)?;
        host.set_camera_yaw(yaw)?;
        host.set_camera_pitch(elevation)?;
        Some(serde_json::json!({
            "start_solar_hours": solar_time * 24.0,
            "start_sun_elevation_degrees": sun[1].asin().to_degrees(),
            "yaw_degrees": yaw.to_degrees(),
            "pitch_degrees": elevation.to_degrees()
        }))
    } else if noon {
        host.set_time_of_day(0.5)?;
        host.set_camera_yaw(-51.0_f32.to_radians())?;
        host.set_camera_pitch(24.0_f32.to_radians())?;
        Some(serde_json::json!({
            "start_solar_hours": 12,
            "yaw_degrees": -51,
            "pitch_degrees": 24,
        }))
    } else {
        None
    };
    let Warmup {
        last_tick,
        mut refill,
        first,
        setup_pass_peaks,
    } = settle_scene(&mut host, &device, content.as_ref(), has_shapes, library)?;
    v1_game::apply_cloud_quality(&mut host).map_err(|e| e.to_string())?;
    let day = if let Some(clouds_off_rate) = mist_day {
        Some(prepare_mist_day(
            &mut host,
            setup,
            view_angles.as_mut().expect("mist day camera"),
            clouds_off_rate,
        )?)
    } else {
        None
    };
    let day_clouds = day.as_ref().map(|(clouds, _)| *clouds);
    let day_schedule = day.map(|(_, schedule)| schedule);
    let measurement_ticks = day_schedule
        .as_ref()
        .map_or(120, |ticks| ticks.last().copied().unwrap_or_default());
    let mut day_samples = Vec::new();
    let mut day_clocks = BTreeMap::new();
    let mut passes: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut whole = Vec::new();
    let mut minimum_volume_instances = usize::MAX;
    for relative_tick in 1..=measurement_ticks {
        // A quarter tick avoids resolving nanosecond rounding on a tick boundary.
        let phase = if mist_day.is_some() { 0.25 } else { 0.0 };
        let time = Duration::from_secs_f64((f64::from(last_tick + relative_tick) + phase) / 60.0);
        if day_schedule
            .as_ref()
            .is_some_and(|ticks| ticks.binary_search(&relative_tick).is_err())
        {
            host.advance_without_rendering(time)?;
            continue;
        }
        let report = host.drive(time, None)?;
        if !report.submitted {
            continue;
        }
        if let Some(clouds) = day_clouds {
            let (submission, solar, amount) = mist_day_submission(&mut host, clouds)?;
            day_clocks.insert(submission, (relative_tick, solar, amount));
        }
        if library {
            let shapes = host
                .cloud_placed_input()
                .and_then(thrum_clouds::WeatherInput::shapes_report)
                .ok_or("the library measurement lacks a Shapes report")?;
            if shapes.volume_instances != 13 || shapes.volume_refusals != 0 {
                return Err(format!("expected all thirteen volume forms: {shapes:?}"));
            }
            minimum_volume_instances = minimum_volume_instances.min(shapes.volume_instances);
        }
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            let clock = day_clocks.remove(&sample.submission_order);
            if mist_day.is_some() && clock.is_none() {
                continue;
            }
            let timings = sample.gpu_pass_ns.ok_or("GPU counters required")?;
            let frame_time = timings.passes().iter().map(|p| p.nanoseconds).sum::<f64>();
            whole.push(frame_time);
            if let Some((tick, solar, amount)) = clock {
                day_samples.push(serde_json::json!({
                    "sample_tick": tick,
                    "solar_hours": solar,
                    "mist_amount": amount,
                    "whole_frame_gpu_ns": frame_time,
                    "passes": timings.passes().iter().map(|pass| (pass.name.to_string(), pass.nanoseconds))
                        .collect::<BTreeMap<_, _>>(),
                }));
            }
            let mut sun_sum = 0.0;
            for pass in timings.passes() {
                passes
                    .entry(pass.name.to_string())
                    .or_default()
                    .push(pass.nanoseconds);
                if pass.name.contains("cloud Sun path") {
                    sun_sum += pass.nanoseconds;
                }
            }
            refill = refill.max(sun_sum);
        }
    }
    if whole.len() < 30 {
        return Err("insufficient completed GPU frames".into());
    }
    let mean = |v: &[f64]| {
        v.iter().sum::<f64>() / f64::from(u32::try_from(v.len()).expect("bounded frame count"))
    };
    let pass_means: BTreeMap<_, _> = passes
        .into_iter()
        .map(|(name, values)| (name, mean(&values)))
        .collect();
    let volume = host.view_volume_storage();
    println!(
        "{}",
        serde_json::json!({
            "adapter":format!("{:?}",adapter.get_info()), "extent":[2560,1440], "capture":false,
            "frames":whole.len(), "whole_frame_gpu_ns":mean(&whole), "pass_gpu_ns":pass_means,
            "maximum_sun_refill_ns":refill,
            "resources":resources(host.cloud_resource_bytes(), volume.as_ref()),
            "view_volume_grid":volume.map(|volume| volume.grid),
            "storage":host.storage_report(), "view":host.scene_camera().map(v1_game::SceneCamera::eye),
            "view_angles":view_angles,
            "minimum_volume_instances":library.then_some(minimum_volume_instances),
            "day_samples":mist_day.map(|_| day_samples),
            "day_clouds":day_clouds,
            "setup_pass_peaks_ns":setup_pass_peaks,
            "first_submission":first,"ray_statistics":ray_statistics(&device,&queue,host.cloud_sky_products().as_ref())?,
            "quality":host.quality_knobs().iter().map(|q|serde_json::json!({"name":q.name,"value":q.value})).collect::<Vec<_>>(),
        })
    );
    Ok(())
}

type Host = thrum_render::runtime::FrameHost<v1_game::GameWorld, v1_game::GameScene>;

fn prepare_mist_day(
    host: &mut Host,
    setup: v1_game::GameSetup,
    view_angles: &mut serde_json::Value,
    clouds_off_rate: f64,
) -> Result<(bool, Vec<u32>), String> {
    use v1_game::{CloudSelection, GroundWeather};
    let clouds = match setup.clouds {
        CloudSelection::Ground(GroundWeather::DawnMist) => true,
        CloudSelection::Off => false,
        _ => return Err("--mist-day needs the dawn-mist or clouds-off selection".into()),
    };
    let rate = if clouds {
        setup.cloud_weather.time_scale
    } else {
        clouds_off_rate
    };
    let schedule = day_schedule(rate);
    host.set_time_of_day(16.5 / 24.0)?;
    if !clouds {
        let origin = host.world().tick() + 1;
        let duration = *schedule.last().expect("mist day schedule");
        host.set_cost_solar_sweep(
            [origin + 1, origin + 1 + u64::from(duration)],
            [16.5, mist_day_end(duration, rate)],
        );
    }
    view_angles["start_solar_hours"] = 16.5.into();
    view_angles["start_sun_elevation_degrees"] = DayCycle::sample(
        0,
        0.0,
        DayCycle::offset_for_time(0, 16.5 / 24.0).expect("mist day start"),
    )
    .toward_sun[1]
        .asin()
        .to_degrees()
        .into();
    Ok((clouds, schedule))
}

fn mist_day_submission(host: &mut Host, clouds: bool) -> Result<(u64, f32, f64), String> {
    let clock = host
        .last_submitted_clock()
        .ok_or("mist day submission has no clock")?;
    let solar = host.rendered_time_of_day(clock.presentation_alpha) * 24.0;
    let amount = if clouds {
        host.world_mut()
            .publication()
            .map_err(|error| error.to_string())?
            .page
            .weather()
            .map(|weather| weather.ground().amount(0, true))
            .ok_or("mist day has no committed weather")?
    } else {
        0.0
    };
    Ok((clock.submission_order, solar, amount))
}

struct Warmup {
    last_tick: u32,
    refill: f64,
    first: Option<serde_json::Value>,
    setup_pass_peaks: BTreeMap<String, f64>,
}

fn settle_scene(
    host: &mut Host,
    device: &wgpu::Device,
    content: Option<&std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
    has_shapes: bool,
    library: bool,
) -> Result<Warmup, String> {
    // Settle ordinary asynchronous residency before the timed interval.
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut last_tick = 0;
    let mut refill = 0.0_f64;
    let mut first = None;
    let mut setup_pass_peaks = BTreeMap::<String, f64>::new();
    let settle_ticks = if has_shapes && !library { 300 } else { 30 };
    for tick in 0..360_u32 {
        last_tick = tick;
        host.drive(Duration::from_secs_f64(f64::from(tick) / 60.0), None)?;
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| e.to_string())?;
        while let Some(sample) = host.poll_completed()? {
            let timings = sample.gpu_pass_ns.ok_or("GPU counters required")?;
            if library {
                for pass in timings.passes() {
                    let peak = setup_pass_peaks.entry(pass.name.to_string()).or_default();
                    *peak = peak.max(pass.nanoseconds);
                }
            }
            let resident = !has_shapes
                || host
                    .cloud_placed_input()
                    .and_then(thrum_clouds::WeatherInput::shapes_report)
                    .is_some_and(|report| report.volume_instances > 0);
            if first.is_none() && resident {
                first = Some(
                    serde_json::json!({"whole_frame_gpu_ns":timings.passes().iter().map(|p|p.nanoseconds).sum::<f64>(),"pass_gpu_ns":timings.passes().iter().map(|p|(p.name.to_string(),p.nanoseconds)).collect::<BTreeMap<_,_>>()}),
                );
            }
            refill = refill.max(
                timings
                    .passes()
                    .iter()
                    .filter(|p| p.name.contains("cloud Sun path"))
                    .map(|p| p.nanoseconds)
                    .sum(),
            );
        }
        if content.is_none_or(|s| s.lock().is_ok_and(|s| s.drawn.is_some()))
            && first.is_some()
            && tick > settle_ticks
        {
            break;
        }
        if Instant::now() > deadline {
            return Err("cooked scene failed to settle".into());
        }
    }
    Ok(Warmup {
        last_tick,
        refill,
        first,
        setup_pass_peaks,
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the measurement samples are bounded over one fixture day"
)]
fn day_schedule(rate: f64) -> Vec<u32> {
    // Follow the ordinary ticks between short timing windows, from
    // late afternoon through dusk, midnight, dawn, sunrise and mid-morning.
    [0.0, 2.5, 5.0, 7.0, 8.25, 11.0, 12.75, 14.5, 17.0]
        .into_iter()
        .flat_map(|hours| {
            let tick = 1 + (hours * 750.0 / rate).round() as u32;
            tick..tick + 8
        })
        .collect()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the proof's end hour fits the presentation's solar phase"
)]
fn mist_day_end(ticks: u32, rate: f64) -> f32 {
    (16.5 + f64::from(ticks) * rate / 750.0) as f32
}

fn resources(
    mut cloud: Vec<(String, u64)>,
    volume: Option<&thrum_render::atmosphere::ViewVolumeStorage>,
) -> Vec<(String, u64)> {
    if let Some(volume) = volume {
        // The provider writes a texture owned by the view volume. Include it
        // once, with the other allocations reported by its owner.
        if volume
            .resources
            .iter()
            .any(|&(name, _)| name == "view volume medium coefficients")
        {
            cloud.retain(|(name, _)| name != "cloud medium coefficient texture");
        }
        cloud.extend(
            volume
                .resources
                .iter()
                .map(|&(name, bytes)| (name.to_owned(), bytes)),
        );
    }
    cloud
}

fn ray_statistics(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sky: Option<&thrum_clouds::SkyProducts>,
) -> Result<serde_json::Value, String> {
    let Some(sky) = sky else {
        return Ok(serde_json::Value::Null);
    };
    let texture = &sky.fresh_front;
    let stride = (texture.width() * 8).div_ceil(256) * 256;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud cost sample counts"),
        size: u64::from(stride) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(texture.height()),
            },
        },
        wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        },
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&output, wgpu::MapMode::Read, .., move |r| {
        let _ = send.send(r);
    });
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    receive
        .recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = output.get_mapped_range(..).map_err(|e| e.to_string())?;
    let refresh = sky.trace_stride;
    let phase = sky.submitted_frames.wrapping_sub(1) % (refresh * refresh);
    let mut samples = Vec::new();
    let [width, height] = sky.active_extent;
    let requested = sky.refresh_stride;
    let requested_phase = sky.submitted_frames.wrapping_sub(1) % (requested * requested);
    let mut out_of_turn = 0;
    for y in 0..height {
        for x in 0..width {
            if x % refresh != phase % refresh || y % refresh != phase / refresh {
                continue;
            }
            if x % requested != requested_phase % requested
                || y % requested != requested_phase / requested
            {
                out_of_turn += 1;
            }
            let at = (y * stride + x * 8 + 4) as usize;
            let word = u32::from(u16::from_le_bytes([mapped[at], mapped[at + 1]]));
            let sample = if word == 0 {
                0.0
            } else {
                f32::from_bits((((word >> 10) + 112) << 23) | ((word & 1023) << 13))
            };
            samples.push(sample);
        }
    }
    samples.sort_by(f32::total_cmp);
    let rays = u32::try_from(samples.len()).map_err(|e| e.to_string())?;
    if rays == 0 {
        return Err("no traced rays in selected phase".into());
    }
    Ok(
        serde_json::json!({"scope":"last completed capture-off frame, selected fresh texels, readback after timed interval","rays_traced":rays,"rays_beyond_refresh_phase":out_of_turn,"mean_samples":samples.iter().map(|&n|f64::from(n)).sum::<f64>()/f64::from(rays),"p95_samples":samples[(samples.len()*95/100).min(samples.len()-1)],"refresh_stride":refresh,"refresh_fraction":1.0/f64::from(refresh*refresh)}),
    )
}
