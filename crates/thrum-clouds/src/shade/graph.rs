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
            tick_passes: Vec::new(),
            resources,
            passes,
            opaque_reads: vec![
                access(6, 0..1, A::SampledRead),
                access(7, 0..mapping.size(), A::UniformRead),
            ],
            view_volume_media: Vec::new(),
            sky_overlays: Vec::new(),
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

pub(super) fn append_ground(
    graph: &mut EffectGraph,
    ground: &crate::ground::GroundGpu,
    targets: &crate::ground::GroundTargets,
    shape_noise: EffectAccess,
) -> Vec<EffectAccess> {
    let access = |resource, range, kind| EffectAccess {
        resource,
        range,
        kind,
        view: None,
        history: None,
    };
    let surface = graph.resources.len();
    graph.resources.push((
        D::of_texture(&ground.field.surface),
        S::Imported { initialized: true },
    ));
    let band = graph.resources.len();
    graph.resources.push((
        D::of_texture(&targets.band.texture),
        S::Imported { initialized: false },
    ));
    let medium = &targets.medium.target;
    let coefficients = graph.resources.len();
    graph.resources.push((
        D::of_texture(&medium.coefficients),
        S::Imported { initialized: false },
    ));
    let mut buffer = |buffer: &wgpu::Buffer, kind, initialized| {
        let index = graph.resources.len();
        graph.resources.push((
            D::Buffer {
                bytes: buffer.size(),
                usage: buffer.usage(),
            },
            S::Imported { initialized },
        ));
        access(index, 0..buffer.size(), kind)
    };
    let ground_inputs = [
        access(surface, 0..1, A::SampledRead),
        buffer(&ground.field.frame, A::UniformRead, true),
        buffer(&ground.field.rows, A::StorageRead, true),
        buffer(&targets.band.mapping, A::UniformRead, true),
    ];
    let phase = buffer(&ground.phases.buffer, A::StorageRead, true);
    let observer = buffer(&medium.observer, A::UniformRead, true);
    let history = buffer(&medium.history, A::UniformRead, true);
    let hemisphere = buffer(&targets.medium.lighting.ambient, A::StorageReadWrite, true);
    let deck_inputs = targets.medium.sky_params.as_ref().map(|params| {
        [
            buffer(params, A::UniformRead, true),
            buffer(&targets.medium.lighting.phase, A::StorageRead, true),
            access(0, 0..4, A::SampledRead),
            access(1, 0..4, A::SampledRead),
            shape_noise,
        ]
    });
    let mut paths: Vec<_> = (3..6)
        .map(|resource| {
            let D::Texture { layers, .. } = graph.resources[resource].0 else {
                unreachable!("Sun path texture")
            };
            access(resource, 0..u64::from(layers), A::SampledRead)
        })
        .collect();
    let D::Buffer { bytes, .. } = graph.resources[7].0 else {
        unreachable!("Sun path mapping")
    };
    paths.push(access(7, 0..bytes, A::UniformRead));
    let band_read = access(band, 0..1, A::SampledRead);
    graph.passes.push(EffectPass {
        name: crate::ground::band::PASS.into(),
        cadence: EffectCadence::EveryFrame,
        async_eligible: false,
        accesses: paths
            .iter()
            .chain(&ground_inputs)
            .cloned()
            .chain([
                access(6, 0..1, A::SampledRead),
                access(band, 0..1, A::StorageWrite),
            ])
            .collect(),
    });
    let trace_inputs: Vec<_> = ground_inputs
        .into_iter()
        .chain([phase, band_read.clone()])
        .collect();
    paths.extend(trace_inputs.iter().cloned());
    paths.extend([
        observer,
        history,
        hemisphere,
        access(
            coefficients,
            0..u64::from(medium.coefficients.depth_or_array_layers()),
            A::StorageWrite,
        ),
    ]);
    paths.extend(deck_inputs.into_iter().flatten());
    graph.passes.push(EffectPass {
        name: crate::ground::medium::PASS.into(),
        cadence: EffectCadence::EveryFrame,
        async_eligible: false,
        accesses: paths,
    });
    graph.opaque_reads.push(band_read);
    graph
        .view_volume_media
        .push(thrum_render::contribution::ViewVolumeResources {
            coefficients,
            ground_band: Some(band),
        });
    trace_inputs
}

pub(super) fn ground_bytes(
    ground: &crate::ground::GroundGpu,
    targets: &crate::ground::GroundTargets,
) -> Vec<(String, u64)> {
    let surface = ground.field.surface.size();
    let band = targets.band.texture.size();
    let coefficients = targets.medium.target.coefficients.size();
    [
        (
            "cloud medium coefficient texture",
            u64::from(coefficients.width)
                * u64::from(coefficients.height)
                * u64::from(coefficients.depth_or_array_layers)
                * 8,
        ),
        (
            "cloud surface field",
            u64::from(surface.width) * u64::from(surface.height) * 16,
        ),
        ("cloud ground frame", ground.field.frame.size()),
        ("cloud fog table", ground.field.rows.size()),
        ("cloud fog phase tables", ground.phases.buffer.size()),
        (
            "cloud ground band",
            u64::from(band.width)
                * u64::from(band.height)
                * u64::from(band.depth_or_array_layers)
                * 8,
        ),
        ("cloud ground band mapping", targets.band.mapping.size()),
        (
            "cloud medium hemisphere light",
            targets.medium.lighting.ambient.size(),
        ),
        (
            "cloud medium droplet phase tables",
            targets.medium.lighting.phase.size(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.into(), bytes))
    .collect()
}
