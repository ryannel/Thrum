use super::Prepared;
use thrum_render::{
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass, SunPathResources},
    graph::{AccessKind as A, ResourceDescriptor as D, ResourceScope as S, TextureKind as K},
};

pub(super) fn declaration(
    gpu: &Prepared,
    epoch: u64,
    ground: &wgpu::Texture,
    mapping: &wgpu::Buffer,
) -> (EffectGraph, usize) {
    let textures: Vec<_> = gpu
        .weather
        .textures()
        .iter()
        .chain(gpu.sun.products())
        .chain(std::iter::once(ground))
        .collect();
    let mut resources: Vec<_> = textures
        .iter()
        .enumerate()
        .map(|(i, t)| {
            (
                D::Texture {
                    dimension: K::D2,
                    width: t.width(),
                    height: t.height(),
                    layers: t.depth_or_array_layers(),
                    mips: 1,
                    samples: 1,
                    format: t.format(),
                    usage: t.usage(),
                },
                S::Persistent {
                    owner: i as u64,
                    epoch,
                    initialized: false,
                },
            )
        })
        .collect();
    resources.push((
        D::Buffer {
            bytes: mapping.size(),
            usage: mapping.usage(),
        },
        S::Persistent {
            owner: 7,
            epoch,
            initialized: true,
        },
    ));
    for b in gpu.weather.uniforms().iter().chain(gpu.sun.uniforms()) {
        resources.push((
            D::Buffer {
                bytes: b.size(),
                usage: b.usage(),
            },
            S::Imported { initialized: true },
        ));
    }
    for b in gpu.weather.axes() {
        resources.push((
            D::Buffer {
                bytes: b.size(),
                usage: b.usage(),
            },
            S::Imported { initialized: true },
        ));
    }
    let shape_noise = resources.len();
    resources.push(gpu.sun.shape_noise.resource());
    let access = |resource, range, kind| EffectAccess {
        resource,
        range,
        kind,
        view: None,
        history: None,
    };
    let mut weather: Vec<_> = (0..3).map(|i| access(i, 0..4, A::StorageWrite)).collect();
    for i in 0..4 {
        weather.push(access(
            8 + i,
            0..gpu.weather.uniforms()[i].size(),
            A::UniformRead,
        ));
        weather.push(access(
            16 + i,
            0..gpu.weather.axes()[i].size(),
            A::StorageRead,
        ));
    }
    let mut passes = vec![EffectPass {
        name: crate::weather::PASS.into(),
        cadence: EffectCadence::TimeSliced,
        async_eligible: true,
        accesses: weather,
    }];
    for i in 0..3 {
        passes.push(EffectPass {
            name: crate::sun_path::PATH_PASSES[i].into(),
            cadence: EffectCadence::TimeSliced,
            async_eligible: true,
            accesses: vec![
                access(0, 0..4, A::SampledRead),
                access(1, 0..4, A::SampledRead),
                access(
                    3 + i,
                    0..u64::from(textures[3 + i].depth_or_array_layers()),
                    A::StorageWrite,
                ),
                access(12 + i, 0..gpu.sun.uniforms()[i].size(), A::UniformRead),
                gpu.sun.shape_noise.read(shape_noise),
            ],
        });
    }
    let mut ground: Vec<_> = (3..6)
        .map(|i| {
            access(
                i,
                0..u64::from(textures[i].depth_or_array_layers()),
                A::SampledRead,
            )
        })
        .collect();
    ground.push(access(6, 0..1, A::StorageWrite));
    ground.push(access(15, 0..gpu.sun.uniforms()[3].size(), A::UniformRead));
    passes.push(EffectPass {
        name: crate::sun_path::GROUND_PASS.into(),
        cadence: EffectCadence::Skippable,
        async_eligible: true,
        accesses: ground,
    });
    passes.push(EffectPass {
        name: crate::sun_path::FAR_PASS.into(),
        cadence: EffectCadence::Skippable,
        async_eligible: true,
        accesses: vec![
            access(
                5,
                0..u64::from(textures[5].depth_or_array_layers()),
                A::SampledRead,
            ),
            access(7, 48..52, A::StorageWrite),
        ],
    });
    (
        EffectGraph {
            resources,
            passes,
            opaque_reads: vec![
                access(6, 0..1, A::SampledRead),
                access(7, 0..mapping.size(), A::UniformRead),
            ],
            view_volume_media: Vec::new(),
            sun_paths: vec![SunPathResources {
                optical_depth: 6,
                mapping: 7,
            }],
            after_opaque: Vec::new(),
        },
        shape_noise,
    )
}

pub(super) fn append_placed_map(
    graph: &mut EffectGraph,
    map: &crate::shapes::map::ShapeMap,
    mut reads: Vec<EffectAccess>,
    epoch: u64,
) {
    let texture = graph.resources.len();
    graph.resources.push((
        D::of_texture(&map.texture),
        S::Persistent {
            owner: 105,
            epoch,
            initialized: false,
        },
    ));
    let uniforms = graph.resources.len();
    graph.resources.push((
        D::Buffer {
            bytes: map.params.size(),
            usage: map.params.usage(),
        },
        S::Imported { initialized: true },
    ));
    reads.truncate(2);
    reads.extend([
        EffectAccess {
            resource: texture,
            range: 0..4,
            kind: A::StorageWrite,
            view: None,
            history: None,
        },
        EffectAccess {
            resource: uniforms,
            range: 0..map.params.size(),
            kind: A::UniformRead,
            view: None,
            history: None,
        },
    ]);
    graph.passes.push(EffectPass {
        name: crate::shapes::map::PASS.into(),
        cadence: EffectCadence::Skippable,
        async_eligible: true,
        accesses: reads,
    });
}
