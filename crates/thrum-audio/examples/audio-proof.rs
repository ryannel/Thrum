//! Silent native reference and timing driver called by scripts/audio.py.

use std::{
    error::Error,
    hint::black_box,
    io::{self, Write},
    time::Instant,
};
use thrum_audio::{
    Config, Listener,
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
        _ => return Err("expected reference, reference-bits or costs".into()),
    }
    output.flush()?;
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
