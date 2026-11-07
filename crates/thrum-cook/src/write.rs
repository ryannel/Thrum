//! Writers for the formats `thrum-asset` reads: Thrum containers and KTX2.

use bytemuck::Pod;
use thrum_asset::Ktx2Format;
use thrum_asset::ktx2::{HEADER_LEN, IDENTIFIER, LEVEL_INDEX_LEN};
use thrum_asset::layout::{CONTAINER_FORMAT, ContainerHeader, SECTION_ALIGN, Section};

/// Builds a Thrum container: header, section table, 64-byte-aligned payloads.
pub(crate) struct ContainerWriter {
    magic: [u8; 8],
    sections: Vec<([u8; 4], u32, u64, Vec<u8>)>,
}

impl ContainerWriter {
    pub(crate) fn new(magic: [u8; 8]) -> Self {
        Self {
            magic,
            sections: Vec::new(),
        }
    }

    pub(crate) fn section<T: Pod>(mut self, tag: [u8; 4], values: &[T]) -> Self {
        let element_size = u32::try_from(size_of::<T>()).expect("element types are small");
        self.sections.push((
            tag,
            element_size,
            values.len() as u64,
            bytemuck::cast_slice(values).to_vec(),
        ));
        self
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        let table_end = size_of::<ContainerHeader>() + self.sections.len() * size_of::<Section>();
        let mut offset = table_end.next_multiple_of(SECTION_ALIGN);
        let mut records = Vec::with_capacity(self.sections.len());
        for (tag, element_size, count, payload) in &self.sections {
            records.push(Section {
                tag: *tag,
                element_size: *element_size,
                offset: offset as u64,
                count: *count,
                reserved: 0,
            });
            offset = (offset + payload.len()).next_multiple_of(SECTION_ALIGN);
        }
        let total = records.last().map_or(table_end, |last| {
            usize::try_from(last.offset).unwrap_or(usize::MAX)
                + self.sections.last().map_or(0, |section| section.3.len())
        });
        let header = ContainerHeader {
            magic: self.magic,
            format: CONTAINER_FORMAT,
            section_count: u32::try_from(records.len()).expect("few sections"),
            total_len: total as u64,
            reserved: [0; 5],
        };
        let mut out = vec![0; total];
        out[..size_of::<ContainerHeader>()].copy_from_slice(bytemuck::bytes_of(&header));
        out[size_of::<ContainerHeader>()..table_end]
            .copy_from_slice(bytemuck::cast_slice(&records));
        for (record, (_, _, _, payload)) in records.iter().zip(&self.sections) {
            let start = usize::try_from(record.offset).unwrap_or(usize::MAX);
            out[start..start + payload.len()].copy_from_slice(payload);
        }
        out
    }
}

/// Encode a KTX2 file. `levels` are largest first; `depth` is zero for a 2D texture.
pub(crate) fn ktx2(
    format: Ktx2Format,
    width: u32,
    height: u32,
    depth: u32,
    levels: &[Vec<u8>],
) -> Vec<u8> {
    let dfd = data_format_descriptor(format);
    let index_end = HEADER_LEN + levels.len() * LEVEL_INDEX_LEN;
    let dfd_offset = index_end.next_multiple_of(4);
    let alignment = usize::try_from(format.level_alignment()).unwrap_or(16);
    // KTX2 stores level data smallest first.
    let mut cursor = dfd_offset + dfd.len();
    let mut offsets = vec![0; levels.len()];
    for (level, data) in levels.iter().enumerate().rev() {
        cursor = cursor.next_multiple_of(alignment);
        offsets[level] = cursor;
        cursor += data.len();
    }
    let mut out = vec![0; cursor];
    out[..12].copy_from_slice(&IDENTIFIER);
    let level_count = u32::try_from(levels.len()).expect("few levels");
    let fields = [
        format.vk_format(),
        1,
        width,
        height,
        depth,
        0,
        1,
        level_count,
        0,
    ];
    for (index, value) in fields.into_iter().enumerate() {
        put_u32(&mut out, 12 + 4 * index, value);
    }
    put_u32(
        &mut out,
        48,
        u32::try_from(dfd_offset).expect("small header"),
    );
    put_u32(
        &mut out,
        52,
        u32::try_from(dfd.len()).expect("small descriptor"),
    );
    for (level, data) in levels.iter().enumerate() {
        let record = HEADER_LEN + level * LEVEL_INDEX_LEN;
        put_u64(&mut out, record, offsets[level] as u64);
        put_u64(&mut out, record + 8, data.len() as u64);
        put_u64(&mut out, record + 16, data.len() as u64);
        out[offsets[level]..offsets[level] + data.len()].copy_from_slice(data);
    }
    out[dfd_offset..dfd_offset + dfd.len()].copy_from_slice(&dfd);
    out
}

fn put_u32(out: &mut [u8], at: usize, value: u32) {
    out[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], at: usize, value: u64) {
    out[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

// Khronos Data Format 1.3 basic descriptor block constants.
const MODEL_RGBSDA: u8 = 1;
const MODEL_BC4: u8 = 131;
const MODEL_BC7: u8 = 134;
const MODEL_ASTC: u8 = 162;
const PRIMARIES_BT709: u8 = 1;
const TRANSFER_LINEAR: u8 = 1;
const TRANSFER_SRGB: u8 = 2;
const CHANNEL_ALPHA: u8 = 15;
const SAMPLE_LINEAR: u8 = 0x10;

/// One sample: bit offset, bit length, channel/qualifier byte, upper value.
type Sample = (u16, u8, u8, u32);

fn data_format_descriptor(format: Ktx2Format) -> Vec<u8> {
    let srgb = format.is_srgb();
    let (model, samples): (u8, Vec<Sample>) = match format {
        Ktx2Format::R8Unorm => (MODEL_RGBSDA, vec![(0, 8, 0, 255)]),
        Ktx2Format::Rgba8Unorm | Ktx2Format::Rgba8Srgb => {
            let alpha = if srgb {
                CHANNEL_ALPHA | SAMPLE_LINEAR
            } else {
                CHANNEL_ALPHA
            };
            (
                MODEL_RGBSDA,
                vec![
                    (0, 8, 0, 255),
                    (8, 8, 1, 255),
                    (16, 8, 2, 255),
                    (24, 8, alpha, 255),
                ],
            )
        }
        Ktx2Format::Bc4Unorm => (MODEL_BC4, vec![(0, 64, 0, u32::MAX)]),
        Ktx2Format::Bc7Unorm | Ktx2Format::Bc7Srgb => (MODEL_BC7, vec![(0, 128, 0, u32::MAX)]),
        Ktx2Format::Astc4x4Unorm | Ktx2Format::Astc4x4Srgb => {
            (MODEL_ASTC, vec![(0, 128, 0, u32::MAX)])
        }
    };
    let (block_width, block_height) = format.block_dimensions();
    let block_size = 24 + 16 * samples.len();
    let mut out = Vec::with_capacity(4 + block_size);
    out.extend_from_slice(&u32::try_from(4 + block_size).expect("small").to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // Khronos vendor, basic descriptor type.
    out.extend_from_slice(&2_u16.to_le_bytes()); // Data Format Descriptor version 1.3.
    out.extend_from_slice(&u16::try_from(block_size).expect("small").to_le_bytes());
    out.extend_from_slice(&[
        model,
        PRIMARIES_BT709,
        if srgb { TRANSFER_SRGB } else { TRANSFER_LINEAR },
        0,
    ]);
    let dimension = |extent: u32| u8::try_from(extent - 1).expect("small block");
    out.extend_from_slice(&[dimension(block_width), dimension(block_height), 0, 0]);
    let mut planes = [0_u8; 8];
    planes[0] = u8::try_from(format.block_bytes()).expect("small block");
    out.extend_from_slice(&planes);
    for (bit_offset, bit_length, channel, upper) in samples {
        out.extend_from_slice(&bit_offset.to_le_bytes());
        out.extend_from_slice(&[bit_length - 1, channel, 0, 0, 0, 0]);
        out.extend_from_slice(&0_u32.to_le_bytes());
        out.extend_from_slice(&upper.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use thrum_asset::layout::{MESH_MAGIC, mesh_tag};
    use thrum_asset::{Ktx2, Ktx2Format, ProductBytes};

    use super::{ContainerWriter, ktx2};

    #[test]
    fn ktx2_round_trips_through_the_runtime_reader() {
        let levels = vec![vec![1; 4 * 16], vec![2; 4 * 4], vec![3; 4]];
        let bytes = ktx2(Ktx2Format::Rgba8Srgb, 4, 4, 0, &levels);
        let texture = Ktx2::open(ProductBytes::copy_from(&bytes)).expect("valid KTX2");
        assert_eq!(texture.levels().len(), 3);
        assert!(!texture.is_3d());
        for (level, expected) in texture.levels().iter().zip(&levels) {
            assert_eq!(texture.level_data(level), &expected[..]);
        }
        // Smallest level is stored first.
        assert!(texture.levels()[2].offset < texture.levels()[0].offset);

        let volume = ktx2(
            Ktx2Format::Bc4Unorm,
            8,
            8,
            4,
            &[vec![9; 8 * 4 * 4], vec![8; 8 * 2]],
        );
        let volume = Ktx2::open(ProductBytes::copy_from(&volume)).expect("valid 3D KTX2");
        assert!(volume.is_3d());
        assert_eq!(volume.levels()[1].depth, 2);
    }

    #[test]
    fn ktx2_reader_refuses_inexact_levels_and_truncation() {
        let bytes = ktx2(Ktx2Format::R8Unorm, 4, 4, 0, &[vec![1; 15]]);
        assert!(
            Ktx2::open(ProductBytes::copy_from(&bytes)).is_err(),
            "short level"
        );
        let bytes = ktx2(Ktx2Format::R8Unorm, 4, 4, 0, &[vec![1; 16]]);
        assert!(Ktx2::open(ProductBytes::copy_from(&bytes[..bytes.len() - 1])).is_err());
        let mut wrong_format = bytes.clone();
        wrong_format[12] = 200;
        assert!(Ktx2::open(ProductBytes::copy_from(&wrong_format)).is_err());
    }

    #[test]
    fn container_sections_are_aligned_and_refused_when_damaged() {
        let bytes = ContainerWriter::new(MESH_MAGIC)
            .section(mesh_tag::INDICES, &[1_u32, 2, 3])
            .finish();
        assert_eq!(bytes.len() % 4, 0);
        let mut damaged = bytes.clone();
        damaged[8] = 9; // format version
        assert!(thrum_asset::MeshProduct::open(ProductBytes::copy_from(&damaged)).is_err());
    }
}
