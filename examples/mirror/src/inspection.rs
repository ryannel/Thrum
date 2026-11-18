//! Mac inspection through the normal presenter and T2 submitted-image tap.
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    time::{Duration, Instant},
};
use thrum_app::{
    Game,
    offscreen::{
        Offscreen,
        clock::{OpportunityClock, Timebase},
    },
    sky::{SkyGame, headless_device},
};
use thrum_render::runtime::{CaptureLimits, SubmittedImage};

const EXTENT: [u32; 2] = [2560, 1440];
const WAIT: Duration = Duration::from_secs(15);
type Scene = Offscreen<SkyGame>;

pub(super) fn run(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(output)?;
    let (device, queue, caps) = pollster::block_on(headless_device())?;
    fs::write(
        output.join("adapter.txt"),
        format!(
            "{:?}\nRecipe: {:?}\nExtent: {EXTENT:?}\nTemporal sampling: off\n",
            caps.adapter(),
            caps.recipe()
                .map_err(|error| format!("inspection recipe: {error:?}"))?
        ),
    )?;
    let mut game = mirror_scene::build()?;
    game.reload();
    let mut scene = Offscreen::new(game, &device, &caps, &queue, EXTENT)?;
    let warm_until = Instant::now() + Duration::from_secs(3);
    while Instant::now() < warm_until {
        scene.step()?;
        wait(&device)?;
        while scene.completed()?.is_some() {}
    }
    scene.enable_capture_with_limits(
        1,
        CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 2,
            byte_allowance: 64 * 1024 * 1024,
        },
    )?;
    fixed(&mut scene, &device, output)?;
    for name in ["pan", "tilt", "orbit", "sun", "mover"] {
        realtime(&mut scene, &device, &output.join(name), name)?;
    }
    if scene.omissions() != 0 {
        return Err(format!("{} submitted capture omissions", scene.omissions()).into());
    }
    Ok(())
}

fn wait(device: &wgpu::Device) -> Result<(), String> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(WAIT),
        })
        .map_err(|error| format!("wait for Metal: {error}"))?;
    Ok(())
}

fn frame(scene: &mut Scene, device: &wgpu::Device) -> Result<SubmittedImage, String> {
    wait(device)?;
    let deadline = Instant::now() + WAIT;
    loop {
        while scene.completed()?.is_some() {}
        if let Some(image) = scene.image()? {
            return Ok(image);
        }
        if Instant::now() >= deadline {
            return Err("submitted capture timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn fixed(
    scene: &mut Scene,
    device: &wgpu::Device,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = output.join("fixed");
    fs::create_dir(&output)?;
    let mut timestamps = ledger(&output)?;
    for (i, (name, height, tilt, sun, azimuth)) in [
        ("low-noon", 1.5, 9.0, 51, 130),
        ("low-tilt", 1.5, 31.0, 51, 130),
        ("high-noon", 10.0, 16.0, 51, 130),
        ("high-tilt", 10.0, 42.0, 51, 130),
        ("dawn", 1.5, 9.0, 4, 78),
        ("dusk", 1.5, 9.0, -5, 262),
        ("night", 1.5, 9.0, -10, 262),
    ]
    .into_iter()
    .enumerate()
    {
        scene.parameters(&format!(r#"{{"camera.height":{height},"camera.tilt":{tilt},"sun.elevation":{sun},"sun.azimuth":{azimuth}}}"#))?;
        let deadline = Instant::now() + WAIT;
        while !scene.step()?.submitted {
            if Instant::now() >= deadline {
                return Err("fixed frame was not submitted".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let image = frame(scene, device)?;
        save(
            &image,
            &output,
            name,
            &mut timestamps,
            u32::try_from(i)?,
            0,
            0,
        )?;
        scene.recycle(image);
    }
    Ok(())
}

fn realtime(
    scene: &mut Scene,
    device: &wgpu::Device,
    output: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(output)?;
    let mut timestamps = ledger(output)?;
    let mut clock = OpportunityClock::new(Instant::now(), 30, Timebase::Elapsed)?;
    let mut count = 0;
    loop {
        let next = clock.next_opportunity();
        let seconds = next.scenario_time.as_secs_f64();
        let t = ((seconds - 0.35) / 2.0).clamp(0.0, 1.0);
        let t = t * t * (3.0 - 2.0 * t);
        let (pan, tilt, orbit, sun, azimuth) = match name {
            "pan" => (25.0 - 50.0 * t, 9.0, 0.0, 51.0, 130.0),
            "tilt" => (0.0, 9.0 + 22.0 * t, 0.0, 51.0, 130.0),
            "orbit" => (0.0, 9.0, -25.0 + 50.0 * t, 51.0, 130.0),
            "sun" => {
                let (sun, azimuth) = solar_hour(t);
                (0.0, 9.0, 0.0, sun, azimuth)
            }
            _ => (0.0, 9.0, 0.0, 5.0, 262.0),
        };
        scene.parameters(&format!(r#"{{"camera.height":1.5,"camera.pan":{pan},"camera.tilt":{tilt},"camera.orbit":{orbit},"sun.elevation":{sun},"sun.azimuth":{azimuth}}}"#))?;
        if scene.step_for(&next)?.submitted {
            let image = frame(scene, device)?;
            save(
                &image,
                output,
                &format!("{count:04}"),
                &mut timestamps,
                next.index,
                next.scenario_time.as_nanos(),
                next.skipped_before,
            )?;
            scene.recycle(image);
            count += 1;
        }
        if seconds >= 2.85 {
            break;
        }
    }
    if count < 12 {
        return Err(format!("{name} submitted only {count} frames").into());
    }
    Ok(())
}

fn solar_hour(phase: f64) -> (f64, f64) {
    // One equinoctial afternoon hour at latitude 45° north.
    let (west, equatorial) = (65.0 + 15.0 * phase).to_radians().sin_cos();
    let south = std::f64::consts::FRAC_1_SQRT_2 * equatorial;
    (
        south.asin().to_degrees(),
        (-west).atan2(-south).to_degrees().rem_euclid(360.0),
    )
}

fn ledger(output: &Path) -> std::io::Result<BufWriter<File>> {
    let mut out = BufWriter::new(File::create(output.join("frames.csv"))?);
    writeln!(
        out,
        "file,opportunity,scenario_ns,skipped,submission,tick,phase,submitted_elapsed_ns,copy_gpu_ns"
    )?;
    Ok(out)
}

fn save(
    image: &SubmittedImage,
    output: &Path,
    name: &str,
    ledger: &mut BufWriter<File>,
    index: u32,
    time: u128,
    skipped: u32,
) -> std::io::Result<()> {
    let file = format!("{name}.rgba");
    fs::write(output.join(&file), &image.rgba)?;
    writeln!(
        ledger,
        "{file},{index},{time},{skipped},{},{},{},{},{}",
        image.sample.submission_order,
        image.sample.tick,
        image.sample.display_phase,
        image.submitted_elapsed.as_nanos(),
        image.copy_gpu_ns.unwrap_or_default()
    )
}
