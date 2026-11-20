use super::{Mip, Readback};

fn device(test: &str) -> Option<(wgpu::Device, wgpu::Queue)> {
    let gpu = crate::gpu(test, None)?;
    Some(
        pollster::block_on(
            gpu.adapter
                .request_device(&wgpu::DeviceDescriptor::default()),
        )
        .expect("readback test device"),
    )
}

fn span(range: &std::ops::Range<u64>) -> std::ops::Range<usize> {
    usize::try_from(range.start).unwrap()..usize::try_from(range.end).unwrap()
}

/// A byte that differs by level, layer, row and column, so a misplaced read cannot match.
fn byte(level: u32, layer: u32, row: u32, column: u32) -> u8 {
    (level * 61 + layer * 29 + row * 7 + column).to_le_bytes()[0]
}

/// Packed bytes of one mip: `layers` images of `rows` rows of `row_bytes` bytes.
fn image(level: u32, layers: std::ops::Range<u32>, rows: u32, row_bytes: u32) -> Vec<u8> {
    layers
        .flat_map(|layer| {
            (0..rows).flat_map(move |row| {
                (0..row_bytes).map(move |column| byte(level, layer, row, column))
            })
        })
        .collect()
}

fn filled_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    dimension: wgpu::TextureDimension,
    size: wgpu::Extent3d,
    levels: u32,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("readback source"),
        size,
        mip_level_count: levels,
        sample_count: 1,
        dimension,
        format,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let block = format.block_copy_size(None).unwrap();
    for level in 0..levels {
        let mip = size.mip_level_size(level, dimension);
        let row_bytes = mip.width * block;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image(level, 0..mip.depth_or_array_layers, mip.height, row_bytes),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(mip.height),
            },
            mip,
        );
    }
    texture
}

#[test]
fn readback_buffer_whole_and_byte_ranges() {
    let Some((device, queue)) = device("readback_buffer_whole_and_byte_ranges") else {
        return;
    };
    let data: Vec<u8> = (0..=255).collect();
    let source = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback source"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&source, 0, &data);
    assert_eq!(super::buffer(&device, &queue, &source, ..), data);
    // Unaligned starts and ends return exactly the bytes asked for.
    for range in [3..13, 0..1, 250..256, 17..17] {
        assert_eq!(
            super::buffer(&device, &queue, &source, range.clone()),
            data[span(&range)],
            "{range:?}"
        );
    }
    assert_eq!(super::buffer(&device, &queue, &source, ..=6), data[..=6]);
}

#[test]
fn readback_appends_to_the_callers_encoder_after_its_work() {
    let Some((device, queue)) = device("readback_appends_to_the_callers_encoder_after_its_work")
    else {
        return;
    };
    let source = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback source"),
        size: 64,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&source, 0, &[9; 64]);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let before = Readback::buffer(&device, &mut encoder, &source, ..);
    encoder.clear_buffer(&source, 16, Some(32));
    let after = Readback::buffer(&device, &mut encoder, &source, 8..56);
    let submission = queue.submit([encoder.finish()]);
    assert_eq!(
        after.wait(&device, submission.clone()),
        [[9; 8], [0; 8], [0; 8], [0; 8], [0; 8], [9; 8]].concat()
    );
    assert_eq!(before.wait(&device, submission), [9; 64]);
}

#[test]
fn readback_whole_mip_or_one_layer_of_an_array_texture() {
    let Some((device, queue)) = device("readback_whole_mip_or_one_layer_of_an_array_texture")
    else {
        return;
    };
    // 70 texels of 4 bytes is 280 bytes a row, padded to 512 in the copy.
    let size = wgpu::Extent3d {
        width: 70,
        height: 5,
        depth_or_array_layers: 3,
    };
    let texture = filled_texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureDimension::D2,
        size,
        3,
    );
    for level in 0..3 {
        let mip = size.mip_level_size(level, wgpu::TextureDimension::D2);
        let row_bytes = mip.width * 4;
        assert_eq!(
            super::texture(&device, &queue, &texture, Mip::level(level), ..),
            image(level, 0..3, mip.height, row_bytes),
            "mip {level}"
        );
        for layer in 0..3 {
            assert_eq!(
                super::texture(&device, &queue, &texture, Mip::layer(level, layer), ..),
                image(level, layer..layer + 1, mip.height, row_bytes),
                "mip {level} layer {layer}"
            );
        }
    }
}

#[test]
fn readback_whole_mip_or_one_slice_of_a_3d_texture() {
    let Some((device, queue)) = device("readback_whole_mip_or_one_slice_of_a_3d_texture") else {
        return;
    };
    let size = wgpu::Extent3d {
        width: 9,
        height: 4,
        depth_or_array_layers: 6,
    };
    let texture = filled_texture(
        &device,
        &queue,
        wgpu::TextureFormat::R32Uint,
        wgpu::TextureDimension::D3,
        size,
        2,
    );
    // A 3D mip halves its depth too: mip 1 is 4 by 2 by 3.
    for level in 0..2 {
        let mip = size.mip_level_size(level, wgpu::TextureDimension::D3);
        let row_bytes = mip.width * 4;
        assert_eq!(
            super::texture(&device, &queue, &texture, Mip::level(level), ..),
            image(level, 0..mip.depth_or_array_layers, mip.height, row_bytes),
            "mip {level}"
        );
        let last = mip.depth_or_array_layers - 1;
        assert_eq!(
            super::texture(&device, &queue, &texture, Mip::layer(level, last), ..),
            image(level, last..last + 1, mip.height, row_bytes),
            "mip {level} slice {last}"
        );
    }
}

#[test]
fn readback_byte_range_of_a_texture_crosses_rows_and_layers() {
    let Some((device, queue)) = device("readback_byte_range_of_a_texture_crosses_rows_and_layers")
    else {
        return;
    };
    let size = wgpu::Extent3d {
        width: 70,
        height: 5,
        depth_or_array_layers: 2,
    };
    let texture = filled_texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureDimension::D2,
        size,
        1,
    );
    let whole = image(0, 0..2, 5, 280);
    for range in [0..1, 279..281, 1000..1700, 2700..2800] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let read = Readback::texture(
            &device,
            &mut encoder,
            &texture,
            Mip::level(0),
            range.clone(),
        );
        let submission = queue.submit([encoder.finish()]);
        assert_eq!(
            read.wait(&device, submission),
            whole[span(&range)],
            "{range:?}"
        );
    }
    assert_eq!(
        super::texture(&device, &queue, &texture, Mip::layer(0, 1), 300..400),
        image(0, 1..2, 5, 280)[300..400]
    );
}

#[test]
fn readback_ranges_resolve_and_refuse_past_the_end() {
    assert_eq!(super::bounds(&(..), 16), 0..16);
    assert_eq!(super::bounds(&(3..=5), 16), 3..6);
    assert_eq!(super::bounds(&(..7), 16), 0..7);
    assert!(std::panic::catch_unwind(|| super::bounds(&(8..20), 16)).is_err());
    assert!(
        std::panic::catch_unwind(|| {
            let (start, end) = (9, 8);
            super::bounds(&(start..end), 16)
        })
        .is_err()
    );
}
