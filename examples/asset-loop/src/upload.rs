//! Create GPU resources from mapped products on the loader thread.
//!
//! Each upload targets a *new* resource, so nothing the render thread is drawing is ever
//! overwritten, and data is written in bounded chunks so the queue's staging lock is never held
//! for a whole large product. The render thread neither waits nor copies.

use thrum_asset::{Ktx2, Ktx2Format, MeshProduct};

/// A mesh ready to draw with the portable indexed path.
pub(crate) struct GpuMesh {
    pub(crate) positions: wgpu::Buffer,
    pub(crate) attributes: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    pub(crate) index_count: u32,
}

/// A sampled texture or volume.
pub(crate) struct GpuTexture {
    pub(crate) view: wgpu::TextureView,
}

pub(crate) fn wgpu_format(format: Ktx2Format) -> wgpu::TextureFormat {
    use wgpu::{AstcBlock, AstcChannel, TextureFormat};
    match format {
        Ktx2Format::R8Unorm => TextureFormat::R8Unorm,
        Ktx2Format::Rgba8Unorm => TextureFormat::Rgba8Unorm,
        Ktx2Format::Rgba8Srgb => TextureFormat::Rgba8UnormSrgb,
        Ktx2Format::Bc4Unorm => TextureFormat::Bc4RUnorm,
        Ktx2Format::Bc7Unorm => TextureFormat::Bc7RgbaUnorm,
        Ktx2Format::Bc7Srgb => TextureFormat::Bc7RgbaUnormSrgb,
        Ktx2Format::Astc4x4Unorm => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::Unorm,
        },
        Ktx2Format::Astc4x4Srgb => TextureFormat::Astc {
            block: AstcBlock::B4x4,
            channel: AstcChannel::UnormSrgb,
        },
    }
}

/// Chunked buffer upload; returns the buffer and bytes written.
fn upload_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    usage: wgpu::BufferUsages,
    bytes: &[u8],
    chunk: usize,
) -> wgpu::Buffer {
    let size = (bytes.len() as u64)
        .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT)
        .max(4);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let chunk = chunk.next_multiple_of(4).max(4);
    for (index, part) in bytes.chunks(chunk).enumerate() {
        let padded;
        let data = if part.len().is_multiple_of(4) {
            part
        } else {
            padded = [part, &[0; 3][..4 - part.len() % 4]].concat();
            &padded
        };
        queue.write_buffer(&buffer, (index * chunk) as u64, data);
        std::thread::yield_now();
    }
    buffer
}

pub(crate) fn upload_mesh(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mesh: &MeshProduct,
    chunk: usize,
) -> (GpuMesh, u64) {
    let positions: &[u8] = bytemuck::cast_slice(mesh.positions());
    let attributes: &[u8] = bytemuck::cast_slice(mesh.attributes());
    let indices: &[u8] = bytemuck::cast_slice(mesh.indices());
    let bytes = (positions.len() + attributes.len() + indices.len()) as u64;
    let gpu = GpuMesh {
        positions: upload_buffer(
            device,
            queue,
            "asset-loop positions",
            wgpu::BufferUsages::VERTEX,
            positions,
            chunk,
        ),
        attributes: upload_buffer(
            device,
            queue,
            "asset-loop attributes",
            wgpu::BufferUsages::VERTEX,
            attributes,
            chunk,
        ),
        indices: upload_buffer(
            device,
            queue,
            "asset-loop indices",
            wgpu::BufferUsages::INDEX,
            indices,
            chunk,
        ),
        index_count: u32::try_from(mesh.indices().len()).unwrap_or(u32::MAX),
    };
    (gpu, bytes)
}

/// Upload every level of a KTX2 product into a new texture, in row (2D) or slice (3D) chunks.
pub(crate) fn upload_ktx2(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &Ktx2,
    chunk: usize,
) -> (GpuTexture, u64) {
    let format = texture.format();
    let base = texture.levels()[0];
    let dimension = if texture.is_3d() {
        wgpu::TextureDimension::D3
    } else {
        wgpu::TextureDimension::D2
    };
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("asset-loop product"),
        size: wgpu::Extent3d {
            width: base.width,
            height: base.height,
            depth_or_array_layers: base.depth,
        },
        mip_level_count: u32::try_from(texture.levels().len()).unwrap_or(1),
        sample_count: 1,
        dimension,
        format: wgpu_format(format),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (block_width, block_height) = format.block_dimensions();
    let mut bytes = 0;
    for (mip, level) in texture.levels().iter().enumerate() {
        let data = texture.level_data(level);
        bytes += data.len() as u64;
        let row_bytes = level.bytes_per_row(format) as usize;
        let rows = level.rows_per_image(format);
        let slice_bytes = row_bytes * rows as usize;
        let physical = |extent: u32, block: u32| extent.next_multiple_of(block);
        // 3D levels split by whole slices; 2D levels by whole block rows.
        let (unit_bytes, units, unit_is_slice) = if texture.is_3d() {
            (slice_bytes, level.depth, true)
        } else {
            (row_bytes, rows, false)
        };
        let per_chunk = u32::try_from((chunk / unit_bytes.max(1)).max(1)).unwrap_or(u32::MAX);
        let mut first = 0;
        while first < units {
            let count = per_chunk.min(units - first);
            let start = first as usize * unit_bytes;
            let end = start + count as usize * unit_bytes;
            let (origin, extent) = if unit_is_slice {
                (
                    wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: first,
                    },
                    wgpu::Extent3d {
                        width: physical(level.width, block_width),
                        height: physical(level.height, block_height),
                        depth_or_array_layers: count,
                    },
                )
            } else {
                (
                    wgpu::Origin3d {
                        x: 0,
                        y: first * block_height,
                        z: 0,
                    },
                    wgpu::Extent3d {
                        width: physical(level.width, block_width),
                        height: count * block_height,
                        depth_or_array_layers: 1,
                    },
                )
            };
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: u32::try_from(mip).unwrap_or(0),
                    origin,
                    aspect: wgpu::TextureAspect::All,
                },
                &data[start..end],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32::try_from(row_bytes).unwrap_or(u32::MAX)),
                    rows_per_image: Some(rows),
                },
                extent,
            );
            std::thread::yield_now();
            first += count;
        }
    }
    let view = gpu.create_view(&wgpu::TextureViewDescriptor::default());
    (GpuTexture { view }, bytes)
}
