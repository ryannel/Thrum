//! Silent native reference and timing driver called by scripts/audio.py.

use std::{
    error::Error,
    hint::black_box,
    io::{self, Write},
    sync::Arc,
    time::Instant,
};
use thrum_audio::{
    Config, Handle, Listener, VoiceSettings,
    diagnostics::{Digest, FullPool, MovingEmitters, ReferenceRun},
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("missing mode")?;
    let mut output = io::BufWriter::new(io::stdout().lock());
    match mode.as_str() {
        "reference" | "reference-bits" => {
            let frames = args.next().ok_or("missing pull size")?.parse()?;
            reference(frames, mode == "reference-bits", &mut output)?;
        }
        "costs" => costs(&mut output)?,
        "sparse" => sparse(&mut output)?,
        _ => return Err("expected reference, reference-bits, costs or sparse".into()),
    }
    output.flush()?;
    Ok(())
}

fn sparse(output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    const PULLS: u32 = 3750;
    const FRAMES_PER_PULL: usize = 128;
    writeln!(output, "pool_slots,sounding_voices,trial,seconds")?;
    for count in [0, 1] {
        for trial in 1..=5 {
            for capacity in [64, 512] {
                let (mut handle, mut mix) = Handle::new(Config {
                    one_shots: 3 * capacity / 4,
                    emitters: capacity / 4,
                    ..Config::default()
                })?;
                mix.start_pulling();
                let mut frame = [0.0; FRAMES_PER_PULL * 2];
                if count == 1 {
                    let sound = handle.add_sound(Arc::from([8192_i16]), 1)?;
                    handle.start_emitter(sound, VoiceSettings::default())?;
                    handle.update();
                }
                mix.pull(&mut frame);
                assert_eq!(handle.record().voices, count);
                let start = Instant::now();
                for _ in 0..PULLS {
                    mix.pull(black_box(&mut frame));
                }
                let seconds = start.elapsed().as_secs_f64();
                assert_eq!(handle.record().voices, count);
                assert_eq!(
                    frame[0].to_bits(),
                    if count == 1 { 0.25_f32 } else { 0.0 }.to_bits()
                );
                writeln!(output, "{capacity},{count},{trial},{seconds}")?;
            }
        }
    }
    Ok(())
}

fn reference(frames: usize, bits: bool, result: &mut impl Write) -> io::Result<()> {
    if !(1..=4096).contains(&frames) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pull size must be in 1 to 4096",
        ));
    }
    let mut output = vec![0.0; 8192];
    let mut reference = ReferenceRun::default();
    let mut digest = Digest::default();
    while reference.remaining() != 0 {
        let count = reference.pull(&mut output[..frames * 2]);
        let samples = &output[..count * 2];
        if bits {
            for sample in samples {
                writeln!(result, "{:08x}", sample.to_bits())?;
            }
        } else {
            digest.write(samples);
        }
    }
    if !bits {
        writeln!(result, "{:016x}", digest.finish())?;
    }
    Ok(())
}

fn costs(output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let mut pool = FullPool::new()?;
    let start = Instant::now();
    let witness = pool.render();
    let elapsed = start.elapsed().as_secs_f64();

    let mut scene = MovingEmitters::new()?;
    let start = Instant::now();
    let eligible = black_box(scene.update());
    let update = start.elapsed().as_secs_f64();

    let config = Config::default();
    writeln!(
        output,
        concat!(
            "{{\"render_seconds\":{},\"witness\":{},\"update_seconds\":{},\"eligible\":{},",
            "\"one_shots\":{},\"emitters\":{},\"commands\":{},\"outcomes\":{},\"sounds\":{},",
            "\"ramp_frames\":{},\"glide_step\":{},\"knee\":{},\"near\":{},\"emitter_margin\":{}}}"
        ),
        elapsed,
        witness,
        update,
        eligible,
        config.one_shots,
        config.emitters,
        config.commands,
        config.outcomes,
        config.sounds,
        config.ramp_frames,
        config.glide_step,
        config.knee,
        Listener::default().near,
        config.emitter_margin
    )?;
    Ok(())
}
