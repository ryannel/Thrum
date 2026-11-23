//! Capture-off timings for the authored pool and a 100,513-instance field.
use std::{
    error::Error,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};
use thrum_app::{
    Game, Gpu,
    sky::{ContentStatus, StillWorld, headless_device},
};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost, FrameSample, FrameWorld, SceneSource, SurfaceOutput},
    synthetic::OUTPUT_FORMAT,
};

use mirror_scene::{MIRROR_POOL, Pool, fixture};

const EXTENT: [u32; 2] = [2560, 1440];
const FIELD: u32 = 100_513;
const ROUNDS: usize = 8;

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let (device, queue, caps) = pollster::block_on(headless_device())?;
    let surface = SurfaceProfile {
        format: OUTPUT_FORMAT,
        extent: EXTENT,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mirror cost output"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    let requests = Arc::new(AtomicU32::new(1));
    let start = Instant::now();
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(
        out,
        "adapter\t{:?}\trecipe\t{:?}",
        caps.adapter(),
        caps.recipe().map_err(|error| format!("{error:?}"))?
    )?;
    let mut game = mirror_scene::build_with_pool(
        Pool::new(MIRROR_POOL).with_cost_requests(Arc::clone(&requests)),
    )?;
    let mut host = pollster::block_on(game.prepare(
        Gpu {
            device: &device,
            queue: &queue,
            capabilities: &caps,
            surface,
        },
        Box::new(|| {}),
    ))?
    .host;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        drop(frame(&mut host, &device, &output, &start)?);
        match game.poll_status() {
            Some(ContentStatus::Drawn(_)) => break,
            Some(ContentStatus::Failed(error)) => return Err(error.into()),
            _ => {}
        }
        if Instant::now() > deadline {
            return Err("mirror cooked content did not draw".into());
        }
    }
    measure(
        "mirror", &mut host, &device, &output, &requests, &start, &mut out,
    )?;
    drop(host);
    let (scene, source) = fixture::scene(FIELD, EXTENT);
    let mut host = pollster::block_on(FrameHost::new(
        &caps,
        &queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(surface),
        },
        StillWorld::default(),
        source,
        scene,
        1.0,
    ))?;
    host.set_scene_contribution(Box::new(
        Pool::new(fixture::POOL).with_cost_requests(Arc::clone(&requests)),
    ))?;
    measure(
        "field", &mut host, &device, &output, &requests, &start, &mut out,
    )?;
    out.flush()?;
    Ok(())
}

fn frame<W: FrameWorld, S: SceneSource<W::Page>>(
    host: &mut FrameHost<W, S>,
    device: &wgpu::Device,
    output: &SurfaceOutput<'_>,
    start: &Instant,
) -> Result<FrameSample, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !host.drive(start.elapsed(), Some(output))?.submitted {
        if Instant::now() > deadline {
            return Err("timing frame missed every opportunity".into());
        }
        std::thread::yield_now();
    }
    loop {
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(15)),
        })?;
        if let Some(sample) = host.poll_completed()? {
            return Ok(sample);
        }
        if Instant::now() > deadline {
            return Err("timing frame did not retire".into());
        }
    }
}

fn measure<W: FrameWorld, S: SceneSource<W::Page>>(
    name: &str,
    host: &mut FrameHost<W, S>,
    device: &wgpu::Device,
    output: &SurfaceOutput<'_>,
    requests: &AtomicU32,
    start: &Instant,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let knobs = host.quality_knobs();
    assert_eq!(knobs.len(), 5);
    let mut arms = vec![
        ("off".to_owned(), 0, None),
        ("one".into(), 1, None),
        ("two".into(), 2, None),
    ];
    for (index, knob) in knobs.iter().enumerate() {
        for (end, value) in [("min", *knob.range.start()), ("max", *knob.range.end())] {
            let (_, control) = knob.name.rsplit_once('/').unwrap_or(("", &knob.name));
            arms.push((format!("{control} {end}"), 1, Some((index, value))));
        }
    }
    // Rotate order in each block, warm every arm, and keep the two slots alive
    // throughout so idle and live states differ only in the work they request.
    for round in 0..=ROUNDS {
        for (arm, count, setting) in arms.iter().cycle().skip(round).take(arms.len()) {
            requests.store(*count, Ordering::Relaxed);
            for (index, knob) in knobs.iter().enumerate() {
                let value = setting
                    .filter(|(changed, _)| *changed == index)
                    .map_or(knob.value, |(_, value)| value);
                host.set_quality_knob(&knob.name, value)?;
            }
            let sample = frame(host, device, output, start)?;
            let gpu = sample.gpu_pass_ns.ok_or("Metal timestamps unavailable")?;
            let active = gpu
                .passes()
                .iter()
                .filter(|pass| pass.name.starts_with("secondary view opaque"))
                .count();
            if active != *count as usize {
                return Err(format!(
                    "{name}/{arm}: measured {active} view draws, requested {count}"
                )
                .into());
            }
            if round > 0 {
                for pass in gpu.passes() {
                    writeln!(
                        out,
                        "{name}\t{arm}\t{round}\t{}\t{}\t{:.0}",
                        sample.submission_order, pass.name, pass.nanoseconds
                    )?;
                }
            }
        }
    }
    Ok(())
}
