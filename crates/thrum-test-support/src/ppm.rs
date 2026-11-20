//! Write 8-bit pixels as a binary PPM image a person can open.

use std::io::Write as _;
use std::path::Path;

/// Channel order of the pixels handed to [`write`]. Alpha is dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pixels {
    /// Three bytes a pixel: red, green, blue.
    Rgb8,
    /// Four bytes a pixel: red, green, blue, alpha.
    Rgba8,
    /// Four bytes a pixel: blue, green, red, alpha.
    Bgra8,
}

impl Pixels {
    const fn stride(self) -> usize {
        match self {
            Self::Rgb8 => 3,
            Self::Rgba8 | Self::Bgra8 => 4,
        }
    }
}

/// Write `pixels`, `extent[0]` by `extent[1]` in rows from the top, to `path` as a P6 image.
///
/// # Errors
/// Reports a pixel count that does not match `extent`, or a failed write.
pub fn write(
    path: impl AsRef<Path>,
    extent: [u32; 2],
    pixels: &[u8],
    order: Pixels,
) -> std::io::Result<()> {
    let count = extent[0] as usize * extent[1] as usize;
    if pixels.len() != count * order.stride() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{} bytes for a {}x{} {order:?} image",
                pixels.len(),
                extent[0],
                extent[1]
            ),
        ));
    }
    let mut out = format!("P6\n{} {}\n255\n", extent[0], extent[1]).into_bytes();
    out.reserve(count * 3);
    for pixel in pixels.chunks_exact(order.stride()) {
        match order {
            Pixels::Bgra8 => out.extend([pixel[2], pixel[1], pixel[0]]),
            Pixels::Rgb8 | Pixels::Rgba8 => out.extend_from_slice(&pixel[..3]),
        }
    }
    std::fs::File::create(path)?.write_all(&out)
}
