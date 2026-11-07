//! Offscreen review capture of a sky scene: the same host and contribution as the window,
//! rendered once to a PNG without a surface. It makes no presentation or timing claim.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use thrum_render::{capabilities::DeviceCapabilities, runtime::FrameConfig};

use super::{ContentStatus, SkyGame};
use crate::Config;

/// Longest wait for the contribution's first drawn generation.
const CONTENT_WAIT: Duration = Duration::from_secs(30);

#[expect(
    clippy::print_stdout,
    reason = "the capture names the file it wrote for the reviewer"
)]
#[expect(
    clippy::print_stderr,
    reason = "capture failures are explained to the person who asked for them"
)]
pub(super) fn run(config: Config, game: impl FnOnce() -> Result<SkyGame, String>, path: &Path) {
    match capture(config, game, path) {
        Ok([width, height]) => {
            println!(
                "{}: wrote {} ({width}x{height})",
                config.name,
                path.display()
            );
        }
        Err(error) => {
            eprintln!("{}: capture to {}: {error}", config.name, path.display());
            std::process::exit(1);
        }
    }
}

fn capture(
    config: Config,
    game: impl FnOnce() -> Result<SkyGame, String>,
    path: &Path,
) -> Result<[u32; 2], String> {
    let mut game = game()?;
    apply_params(config.name, &mut game)?;
    let (device, queue, capabilities) = pollster::block_on(headless_device())?;
    let extent = config.window_size;
    let mut host = game.build_host(
        &capabilities,
        &queue,
        FrameConfig {
            render_extent: extent,
            surface: None,
        },
    )?;
    let deadline = Instant::now() + CONTENT_WAIT;
    while let Some(status) = game.poll_status() {
        match status {
            ContentStatus::Drawn(_) => break,
            ContentStatus::Failed(error) => return Err(format!("content failed: {error}")),
            ContentStatus::Loading => {}
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "content was not drawn within {} s; check the asset store and the cooker's output",
                CONTENT_WAIT.as_secs()
            ));
        }
        // Each explicit render records the contribution's staged uploads.
        drop(host.capture_pixels().map_err(|error| error.to_string())?);
        std::thread::sleep(Duration::from_millis(2));
    }
    let pixels = host.capture_pixels().map_err(|error| error.to_string())?;
    drop(device);
    let png = encode_png(extent, &pixels)?;
    std::fs::write(path, png).map_err(|error| format!("write: {error}"))?;
    Ok(extent)
}

/// Apply `THRUM_PARAMS` as the window would at start; a capture refuses a file it cannot read.
#[expect(
    clippy::print_stderr,
    reason = "parameter diagnostics are for the person editing the file"
)]
fn apply_params(name: &str, game: &mut SkyGame) -> Result<(), String> {
    let Some(path) = std::env::var_os("THRUM_PARAMS").map(PathBuf::from) else {
        return Ok(());
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("parameter file {}: {error}", path.display()))?;
    let report = game
        .controls
        .apply_file(&text)
        .map_err(|error| format!("parameter file {}: {error}", path.display()))?;
    eprintln!("{name}: {}: {report}", path.display());
    for problem in &report.problems {
        eprintln!("  {problem}");
    }
    Ok(())
}

async fn headless_device() -> Result<(wgpu::Device, wgpu::Queue, DeviceCapabilities), String> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("adapter: {error}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter)
                | DeviceCapabilities::requested_content_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|error| format!("device: {error}"))?;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 1);
    Ok((device, queue, capabilities))
}

/// Encode tightly packed RGBA8 display pixels as an RGB PNG with stored (uncompressed)
/// deflate blocks: larger than a compressed file, but exact and dependency-free.
pub(super) fn encode_png(extent: [u32; 2], rgba: &[u8]) -> Result<Vec<u8>, String> {
    let [width, height] = extent;
    let row = width as usize * 4;
    if width == 0 || height == 0 || rgba.len() != row * height as usize {
        return Err(format!(
            "{} pixel bytes do not form a {width}x{height} RGBA image",
            rgba.len()
        ));
    }
    let mut raw = Vec::with_capacity((width as usize * 3 + 1) * height as usize);
    for line in rgba.chunks_exact(row) {
        // Filter type zero: each scanline is stored as is.
        raw.push(0);
        for pixel in line.as_chunks::<4>().0 {
            raw.extend_from_slice(&pixel[..3]);
        }
    }
    let mut zlib = Vec::with_capacity(raw.len() + raw.len() / 65_535 * 5 + 11);
    zlib.extend_from_slice(&[0x78, 0x01]);
    let mut blocks = raw.chunks(65_535).peekable();
    while let Some(block) = blocks.next() {
        zlib.push(u8::from(blocks.peek().is_none()));
        let length = u16::try_from(block.len()).expect("stored blocks hold at most 65 535 bytes");
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // Eight bits per channel, RGB, deflate, adaptive filtering, no interlace.
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, *b"IHDR", &header)?;
    chunk(&mut png, *b"IDAT", &zlib)?;
    chunk(&mut png, *b"IEND", &[])?;
    Ok(png)
}

fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) -> Result<(), String> {
    let length = u32::try_from(data.len()).map_err(|_| "PNG chunk exceeds 4 GiB")?;
    png.extend_from_slice(&length.to_be_bytes());
    let start = png.len();
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
    Ok(())
}

pub(super) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in bytes.chunks(5_552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}
