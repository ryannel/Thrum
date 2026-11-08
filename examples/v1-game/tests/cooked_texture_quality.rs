//! G5 texture quality: every level of every compressed texture product, decoded by the GPU
//! through the engine's own format mapping, against the same level of the portable product (the
//! exact source and its linear-light mips). BC4 volumes are decoded on the CPU against the R8
//! product. The floor sits well below every real product and well above a channel-swapped or
//! colour-space-swapped substitute, both of which this test also measures.
#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use thrum_asset::{AssetStore, Ktx2, Ktx2Format, Profile};
use thrum_asset_gpu::wgpu_format;
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};

/// PSNR floor in dB for every compressed level of at least `FLOOR_SIZE` texels a side. Measured
/// 8 November on Apple M-series: base levels 48–53 dB, levels of 16 and up at least 34 dB, a
/// red/blue swap at most 17 dB and the other colour-space label at most 14 dB.
const FLOOR_DB: f64 = 30.0;
/// Levels below this are one to four blocks of the card's busiest content (21–36 dB measured)
/// and are recorded, not floored.
const FLOOR_SIZE: u32 = 16;
const CARD: u32 = 256;

/// Peak signal-to-noise ratio of 8-bit samples, over the channels `channels` selects in
/// `stride`-sized texels.
fn psnr(a: &[u8], b: &[u8], stride: usize, channels: &[usize]) -> f64 {
    assert_eq!(a.len(), b.len());
    let (mut sum, mut count) = (0.0_f64, 0_u64);
    for (x, y) in a.chunks_exact(stride).zip(b.chunks_exact(stride)) {
        for &channel in channels {
            let difference = f64::from(x[channel]) - f64::from(y[channel]);
            sum += difference * difference;
            count += 1;
        }
    }
    let mse = sum / count as f64;
    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

struct Decoder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
}

impl Decoder {
    fn new() -> Option<Self> {
        pollster::block_on(async {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .ok()?;
            let wanted =
                wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::TEXTURE_COMPRESSION_ASTC;
            if !adapter.features().contains(wanted) {
                eprintln!("skipped: the adapter lacks BC or ASTC sampling");
                return None;
            }
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    required_features: wanted,
                    ..Default::default()
                })
                .await
                .ok()?;
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("decode"),
                source: wgpu::ShaderSource::Wgsl(
                    r"
@group(0) @binding(0) var source: texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fragment(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(source, vec2<i32>(p.xy), 0);
}"
                    .into(),
                ),
            });
            Some(Self {
                device,
                queue,
                shader,
            })
        })
    }

    /// Every level of `ktx` uploaded as `format`, each decoded into an RGBA8 target that
    /// re-encodes like `target` and read back tightly packed.
    fn decode(
        &self,
        ktx: &Ktx2,
        format: wgpu::TextureFormat,
        target: wgpu::TextureFormat,
    ) -> Vec<Vec<u8>> {
        let levels = ktx.levels();
        let base = &levels[0];
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("compressed"),
            size: wgpu::Extent3d {
                width: base.width,
                height: base.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: u32::try_from(levels.len()).expect("levels"),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let (block_width, block_height) = format.block_dimensions();
        let block_bytes = format.block_copy_size(None).expect("block size");
        for (index, level) in levels.iter().enumerate() {
            let blocks_wide = level.width.div_ceil(block_width);
            let blocks_high = level.height.div_ceil(block_height);
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: u32::try_from(index).expect("level"),
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                ktx.level_data(level),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(blocks_wide * block_bytes),
                    rows_per_image: Some(blocks_high),
                },
                wgpu::Extent3d {
                    width: blocks_wide * block_width,
                    height: blocks_high * block_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("decode"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fragment"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(target.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        levels
            .iter()
            .enumerate()
            .map(|(index, level)| {
                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: u32::try_from(index).expect("level"),
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    }],
                });
                self.render(&pipeline, &bind, target, level.width, level.height)
            })
            .collect()
    }

    fn render(
        &self,
        pipeline: &wgpu::RenderPipeline,
        bind: &wgpu::BindGroup,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("decoded"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let view = target.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("decode"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            extent,
        );
        self.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |result| {
            result.expect("map");
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("GPU wait");
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .expect("mapped readback");
        mapped
            .chunks_exact(row as usize)
            .flat_map(|line| &line[..(width * 4) as usize])
            .copied()
            .collect()
    }
}

/// BC4 (unsigned) block decode into `width`×`height` R8 texels of one slice.
fn decode_bc4(blocks: &[u8], width: u32, height: u32) -> Vec<u8> {
    let wide = width.div_ceil(4) as usize;
    let mut out = vec![0_u8; (width * height) as usize];
    for (index, block) in blocks.chunks_exact(8).enumerate() {
        let (r0, r1) = (u32::from(block[0]), u32::from(block[1]));
        let palette: [u32; 8] = std::array::from_fn(|i| {
            let i = i as u32;
            match i {
                0 => r0,
                1 => r1,
                _ if r0 > r1 => ((8 - i) * r0 + (i - 1) * r1) / 7,
                _ if i < 6 => ((6 - i) * r0 + (i - 1) * r1) / 5,
                6 => 0,
                _ => 255,
            }
        });
        let bits = block[2..8]
            .iter()
            .rev()
            .fold(0_u64, |bits, &byte| bits << 8 | u64::from(byte));
        let (bx, by) = ((index % wide) as u32 * 4, (index / wide) as u32 * 4);
        for texel in 0..16_u32 {
            let (x, y) = (bx + texel % 4, by + texel / 4);
            if x < width && y < height {
                let code = (bits >> (3 * texel)) & 7;
                out[(y * width + x) as usize] = palette[code as usize] as u8;
            }
        }
    }
    out
}

fn manifest_product(store: &AssetStore, name: &str, profile: Profile) -> Ktx2 {
    let manifest = store
        .load_manifest(&store.read_head().expect("head").expect("published"))
        .expect("manifest");
    let entry = manifest.resolve(name, &[profile]).expect("product");
    assert_eq!(entry.profile, profile, "{name}");
    Ktx2::open(store.map(&entry.product).expect("bytes")).expect("ktx2")
}

fn cook(root: &Path, quality: Quality) -> AssetStore {
    let source = root.join("source");
    std::fs::create_dir_all(source.join("textures")).expect("source tree");
    demo::write_test_card_png(&source.join("textures/card.png"), CARD, false).expect("card");
    demo::write_test_card_png(&source.join("textures/cutout.png"), CARD, true).expect("alpha card");
    demo::write_test_card_png(&source.join("textures/data.linear.png"), CARD, false)
        .expect("linear card");
    demo::write_cloud_volume(&source.join("cloud.tvol"), 64, 3).expect("volume");
    let store = root.join("store");
    let report = Cooker::open(CookConfig {
        source,
        store: store.clone(),
        profiles: Profile::ALL.to_vec(),
        quality,
        durability: Durability::Process,
    })
    .expect("cooker")
    .cook_all()
    .expect("cook");
    assert!(
        report.failed.is_empty() && report.refused.is_empty(),
        "{report:?}"
    );
    AssetStore::new(store)
}

/// The same format with the other colour-space label: the substitute a wrong label produces.
fn other_colour_space(format: Ktx2Format) -> Ktx2Format {
    match format {
        Ktx2Format::Bc7Srgb => Ktx2Format::Bc7Unorm,
        Ktx2Format::Bc7Unorm => Ktx2Format::Bc7Srgb,
        Ktx2Format::Astc4x4Srgb => Ktx2Format::Astc4x4Unorm,
        Ktx2Format::Astc4x4Unorm => Ktx2Format::Astc4x4Srgb,
        other => panic!("not a compressed colour format: {other:?}"),
    }
}

#[test]
fn every_compressed_product_level_clears_the_quality_floor_and_substitutes_do_not() {
    let Some(decoder) = Decoder::new() else {
        return;
    };
    let root = std::env::temp_dir().join(format!("thrum-g5-quality-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    for quality in [Quality::Fast, Quality::Final] {
        let store = cook(&root.join(format!("{quality:?}")), quality);
        for name in ["textures/card", "textures/cutout", "textures/data"] {
            let portable = manifest_product(&store, name, Profile::Portable);
            let reference: Vec<&[u8]> = portable
                .levels()
                .iter()
                .map(|level| portable.level_data(level))
                .collect();
            let alpha = name == "textures/cutout";
            let channels: &[usize] = if alpha { &[0, 1, 2, 3] } else { &[0, 1, 2] };
            for profile in [Profile::DesktopBc, Profile::AppleAstc] {
                let product = manifest_product(&store, name, profile);
                // The portable product's own format re-encodes the decode exactly as stored.
                let target = wgpu_format(portable.format());
                let decoded = decoder.decode(&product, wgpu_format(product.format()), target);
                let mut levels = Vec::new();
                for (index, (decoded, reference)) in decoded.iter().zip(&reference).enumerate() {
                    let level = &product.levels()[index];
                    let db = psnr(decoded, reference, 4, channels);
                    levels.push(format!("{}:{db:.1}", level.width));
                    if level.width >= FLOOR_SIZE && level.height >= FLOOR_SIZE {
                        assert!(
                            db >= FLOOR_DB,
                            "{quality:?} {name} {:?} level {index} is {db:.1} dB",
                            product.format()
                        );
                    }
                }
                // Substitutes, measured at the base level: swapped red and blue, and the other
                // colour-space label on the same blocks.
                let swapped: Vec<u8> = decoded[0]
                    .chunks_exact(4)
                    .flat_map(|t| [t[2], t[1], t[0], t[3]])
                    .collect();
                let swapped_db = psnr(&swapped, reference[0], 4, channels);
                let relabelled = decoder.decode(
                    &product,
                    wgpu_format(other_colour_space(product.format())),
                    target,
                );
                let relabelled_db = psnr(&relabelled[0], reference[0], 4, channels);
                println!(
                    "QUALITY {quality:?} {name} {:?}: levels {} | swapped {swapped_db:.1} dB, relabelled {relabelled_db:.1} dB",
                    product.format(),
                    levels.join(" ")
                );
                assert!(
                    swapped_db < FLOOR_DB && relabelled_db < FLOOR_DB,
                    "{quality:?} {name} {:?}: a substitute clears the floor ({swapped_db:.1}, {relabelled_db:.1} dB)",
                    product.format()
                );
            }
        }
        let portable = manifest_product(&store, "cloud", Profile::Portable);
        let bc4 = manifest_product(&store, "cloud", Profile::DesktopBc);
        assert_eq!(bc4.format(), Ktx2Format::Bc4Unorm);
        let mut levels = Vec::new();
        for (index, (level, reference)) in bc4.levels().iter().zip(portable.levels()).enumerate() {
            let blocks = bc4.level_data(level);
            let per_slice = blocks.len() / level.depth as usize;
            let decoded: Vec<u8> = blocks
                .chunks_exact(per_slice)
                .flat_map(|slice| decode_bc4(slice, level.width, level.height))
                .collect();
            let db = psnr(&decoded, portable.level_data(reference), 1, &[0]);
            levels.push(format!("{}:{db:.1}", level.width));
            if level.width >= FLOOR_SIZE && level.height >= FLOOR_SIZE {
                assert!(
                    db >= FLOOR_DB,
                    "{quality:?} BC4 volume level {index} is {db:.1} dB"
                );
            }
        }
        println!(
            "QUALITY {quality:?} cloud Bc4Unorm: levels {}",
            levels.join(" ")
        );
    }
    std::fs::remove_dir_all(&root).expect("remove test store");
}
