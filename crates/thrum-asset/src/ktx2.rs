//! Minimal validated KTX2 reader for the formats Thrum cooks.
//!
//! Supports single-face, non-array 2D and 3D textures without supercompression. Every level's
//! offset, alignment and exact byte length are checked against the format's block size, so a
//! level slice can be handed straight to a GPU texture upload.

use crate::{AssetError, ProductBytes};

/// The KTX2 file identifier.
pub const IDENTIFIER: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];
/// Byte length of the fixed header before the level index.
pub const HEADER_LEN: usize = 80;
/// Byte length of one level-index record.
pub const LEVEL_INDEX_LEN: usize = 24;

/// Pixel formats Thrum cooks, with their Vulkan format codes as stored in KTX2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ktx2Format {
    /// `VK_FORMAT_R8_UNORM`.
    R8Unorm,
    /// `VK_FORMAT_R8G8B8A8_UNORM`.
    Rgba8Unorm,
    /// `VK_FORMAT_R8G8B8A8_SRGB`.
    Rgba8Srgb,
    /// `VK_FORMAT_BC4_UNORM_BLOCK`.
    Bc4Unorm,
    /// `VK_FORMAT_BC7_UNORM_BLOCK`.
    Bc7Unorm,
    /// `VK_FORMAT_BC7_SRGB_BLOCK`.
    Bc7Srgb,
    /// `VK_FORMAT_ASTC_4x4_UNORM_BLOCK`.
    Astc4x4Unorm,
    /// `VK_FORMAT_ASTC_4x4_SRGB_BLOCK`.
    Astc4x4Srgb,
}

impl Ktx2Format {
    const ALL: [Self; 8] = [
        Self::R8Unorm,
        Self::Rgba8Unorm,
        Self::Rgba8Srgb,
        Self::Bc4Unorm,
        Self::Bc7Unorm,
        Self::Bc7Srgb,
        Self::Astc4x4Unorm,
        Self::Astc4x4Srgb,
    ];

    /// Vulkan format code stored in the KTX2 header.
    pub fn vk_format(self) -> u32 {
        match self {
            Self::R8Unorm => 9,
            Self::Rgba8Unorm => 37,
            Self::Rgba8Srgb => 43,
            Self::Bc4Unorm => 139,
            Self::Bc7Unorm => 145,
            Self::Bc7Srgb => 146,
            Self::Astc4x4Unorm => 157,
            Self::Astc4x4Srgb => 158,
        }
    }

    /// Decode a supported Vulkan format code.
    pub fn from_vk_format(code: u32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format| format.vk_format() == code)
    }

    /// Texel block width and height (depth is always one).
    pub fn block_dimensions(self) -> (u32, u32) {
        match self {
            Self::R8Unorm | Self::Rgba8Unorm | Self::Rgba8Srgb => (1, 1),
            _ => (4, 4),
        }
    }

    /// Bytes per texel block.
    pub fn block_bytes(self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rgba8Unorm | Self::Rgba8Srgb => 4,
            Self::Bc4Unorm => 8,
            Self::Bc7Unorm | Self::Bc7Srgb | Self::Astc4x4Unorm | Self::Astc4x4Srgb => 16,
        }
    }

    /// Whether the format stores sRGB-encoded colour.
    pub fn is_srgb(self) -> bool {
        matches!(self, Self::Rgba8Srgb | Self::Bc7Srgb | Self::Astc4x4Srgb)
    }

    /// Required level-offset alignment: `lcm(block bytes, 4)`.
    pub fn level_alignment(self) -> u64 {
        let bytes = u64::from(self.block_bytes());
        if bytes.is_multiple_of(4) {
            bytes
        } else {
            bytes * 4
        }
    }

    /// Bytes in one level of the given texel extent.
    pub fn level_bytes(self, width: u32, height: u32, depth: u32) -> u64 {
        let (block_width, block_height) = self.block_dimensions();
        // Saturates rather than wraps, so a hostile extent can never match a real level length.
        u64::from(width.div_ceil(block_width))
            .saturating_mul(u64::from(height.div_ceil(block_height)))
            .saturating_mul(u64::from(depth.max(1)))
            .saturating_mul(u64::from(self.block_bytes()))
    }
}

/// One mip level: its texel extent and byte range in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ktx2Level {
    /// Texel width.
    pub width: u32,
    /// Texel height.
    pub height: u32,
    /// Texel depth; one for 2D textures.
    pub depth: u32,
    /// Byte offset in the file.
    pub offset: u64,
    /// Byte length.
    pub len: u64,
}

impl Ktx2Level {
    /// Bytes in one row of texel blocks.
    pub fn bytes_per_row(&self, format: Ktx2Format) -> u32 {
        self.width.div_ceil(format.block_dimensions().0) * format.block_bytes()
    }

    /// Rows of texel blocks in one depth slice.
    pub fn rows_per_image(&self, format: Ktx2Format) -> u32 {
        self.height.div_ceil(format.block_dimensions().1)
    }
}

/// A validated KTX2 product that owns its (usually mapped) bytes.
#[derive(Debug)]
pub struct Ktx2 {
    bytes: ProductBytes,
    format: Ktx2Format,
    levels: Vec<Ktx2Level>,
    dimension_3d: bool,
}

impl Ktx2 {
    /// Validate a KTX2 product.
    ///
    /// # Errors
    /// Refuses malformed headers, unsupported shapes or formats, and inexact level ranges.
    pub fn open(bytes: ProductBytes) -> Result<Self, AssetError> {
        let (format, levels, dimension_3d) = parse(&bytes)?;
        Ok(Self {
            bytes,
            format,
            levels,
            dimension_3d,
        })
    }

    /// Pixel format.
    pub fn format(&self) -> Ktx2Format {
        self.format
    }

    /// Whether this is a 3D texture (a baked volume).
    pub fn is_3d(&self) -> bool {
        self.dimension_3d
    }

    /// Levels, largest first.
    pub fn levels(&self) -> &[Ktx2Level] {
        &self.levels
    }

    /// Bytes of one level, ready for a GPU upload.
    pub fn level_data(&self, level: &Ktx2Level) -> &[u8] {
        // Ranges were checked against the file length in `parse`.
        let start = usize::try_from(level.offset).unwrap_or(usize::MAX);
        let len = usize::try_from(level.len).unwrap_or(usize::MAX);
        self.bytes
            .get(start..start.saturating_add(len))
            .unwrap_or_default()
    }
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn read_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from(read_u32(bytes, at)) | (u64::from(read_u32(bytes, at + 4)) << 32)
}

/// Validate KTX2 bytes and return format, levels (largest first) and whether the texture is 3D.
///
/// # Errors
/// As [`Ktx2::open`].
pub fn parse(bytes: &[u8]) -> Result<(Ktx2Format, Vec<Ktx2Level>, bool), AssetError> {
    if bytes.len() < HEADER_LEN || bytes[..12] != IDENTIFIER {
        return Err(AssetError::Malformed("KTX2 identifier"));
    }
    let format = Ktx2Format::from_vk_format(read_u32(bytes, 12))
        .ok_or(AssetError::UnsupportedTexture("format"))?;
    let [
        type_size,
        width,
        height,
        depth,
        layers,
        faces,
        level_count,
        supercompression,
    ] = std::array::from_fn(|index| read_u32(bytes, 16 + 4 * index));
    if type_size != 1 || layers > 1 || faces != 1 || supercompression != 0 {
        return Err(AssetError::UnsupportedTexture(
            "type size, arrays, cube faces or supercompression",
        ));
    }
    if width == 0 || height == 0 {
        return Err(AssetError::UnsupportedTexture("1D or empty texture"));
    }
    let (block_width, block_height) = format.block_dimensions();
    if !width.is_multiple_of(block_width) || !height.is_multiple_of(block_height) {
        // GPU APIs require block-compressed base levels to span whole blocks.
        return Err(AssetError::UnsupportedTexture(
            "compressed base extent is not a whole number of blocks",
        ));
    }
    let largest = width.max(height).max(depth);
    let level_count = level_count as usize;
    if level_count == 0 || level_count > (32 - largest.leading_zeros()) as usize {
        return Err(AssetError::Malformed("KTX2 level count"));
    }
    let index_end = HEADER_LEN + level_count * LEVEL_INDEX_LEN;
    if bytes.len() < index_end {
        return Err(AssetError::Malformed("KTX2 level index"));
    }
    let dfd_offset = read_u32(bytes, 48) as usize;
    let dfd_len = read_u32(bytes, 52) as usize;
    if dfd_offset < index_end || dfd_offset.saturating_add(dfd_len) > bytes.len() || dfd_len < 4 {
        return Err(AssetError::Malformed("KTX2 data format descriptor"));
    }
    let mut levels = Vec::with_capacity(level_count);
    for level in 0..level_count {
        let record = HEADER_LEN + level * LEVEL_INDEX_LEN;
        let offset = read_u64(bytes, record);
        let len = read_u64(bytes, record + 8);
        let extent = |size: u32| (size >> level).max(1);
        let (level_width, level_height) = (extent(width), extent(height));
        let level_depth = if depth == 0 { 1 } else { extent(depth) };
        let expected = format.level_bytes(level_width, level_height, level_depth);
        let in_file = offset
            .checked_add(len)
            .is_some_and(|end| end <= bytes.len() as u64);
        if len != expected
            || read_u64(bytes, record + 16) != len
            || !offset.is_multiple_of(format.level_alignment())
            || offset < index_end as u64
            || !in_file
        {
            return Err(AssetError::Malformed("KTX2 level range"));
        }
        levels.push(Ktx2Level {
            width: level_width,
            height: level_height,
            depth: level_depth,
            offset,
            len,
        });
    }
    Ok((format, levels, depth > 0))
}
