//! Native paced full-frame lossless writer feasibility probe.
#[cfg(target_os = "macos")]
mod native {
    //! Full-record paced writer probe; stdin is exact 1920x1080 RGBA, stdout a spool.
    use std::{
        io::{self, Read, Write},
        os::fd::AsFd,
        time::{Duration, Instant},
    };
    use thrum_app::offscreen::writer::{ImageWriter, SpoolEncoding};
    pub(super) fn main() -> Result<(), Box<dyn std::error::Error>> {
        const BYTES: usize = 1920 * 1080 * 4;
        let output = std::fs::File::from(io::stdout().as_fd().try_clone_to_owned()?);
        let writer = ImageWriter::<Vec<u8>>::start(
            output,
            32,
            8 * 1024 * 1024 * 1024,
            SpoolEncoding::ZstdDelta,
        )
        .map_err(io::Error::other)?;
        let mut pool: Vec<_> = (0..32).map(|_| vec![0; BYTES]).collect();
        let mut discard = vec![0; BYTES];
        let mut misses = Vec::new();
        let mut peak = 0;
        let origin = Instant::now();
        let mut input = io::stdin().lock();
        for id in 0..480_u32 {
            while let Some(image) = writer.recycled() {
                pool.push(image);
            }
            let due = Duration::from_secs_f64(f64::from(id) / 120.0);
            if let Some(wait) = due.checked_sub(origin.elapsed()) {
                std::thread::sleep(wait);
            }
            if let Some(mut image) = pool.pop() {
                input.read_exact(&mut image)?;
                if let Some(image) = writer.submit(image) {
                    pool.push(image);
                    misses.push(id);
                }
            } else {
                input.read_exact(&mut discard)?;
                misses.push(id);
            }
            peak = peak.max(32 - pool.len());
        }
        let end = origin.elapsed();
        let report = writer
            .finish(Duration::from_secs(20))
            .map_err(io::Error::other)?;
        io::stderr().write_all(format!("{{\"frames\":{},\"raw_bytes\":{},\"encoded_bytes\":{},\"write_ns\":{},\"io_ns\":{},\"flush_ns\":{},\"interval_ns\":{},\"drain_ns\":{},\"cpu_peak\":{},\"misses\":{:?},\"frame_sha256\":{:?}}}\n",report.frames,report.bytes,report.encoded_bytes,report.write_time.as_nanos(),report.io_time.as_nanos(),report.flush_time.as_nanos(),end.as_nanos(),origin.elapsed().saturating_sub(end).as_nanos(),peak,misses,report.frame_hashes).as_bytes())?;
        if !misses.is_empty() {
            return Err(io::Error::other("writer probe overflow").into());
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    native::main()
}
#[cfg(not(target_os = "macos"))]
fn main() {}
