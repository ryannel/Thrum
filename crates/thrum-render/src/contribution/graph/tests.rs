use super::*;

#[test]
fn history_previous_and_next_are_distinct_and_result_orders_its_consumer() {
    let access = |kind, history| EffectAccess {
        resource: 0,
        range: 0..4,
        kind,
        view: None,
        history: Some(history),
    };
    let mut effect = EffectGraph {
        tick_passes: Vec::new(),
        opaque_reads: vec![access(AccessKind::StorageRead, HistoryBinding::Result)],
        view_volume_media: Vec::new(),
        sky_overlays: Vec::new(),
        sun_paths: Vec::new(),
        after_opaque: Vec::new(),
        resources: vec![(
            ResourceDescriptor::Buffer {
                bytes: 4,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::History {
                owner: 1,
                epoch: 1,
                initialized: true,
            },
        )],
        passes: vec![
            EffectPass {
                name: "producer".into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![
                    access(AccessKind::StorageRead, HistoryBinding::Previous),
                    access(AccessKind::StorageWrite, HistoryBinding::Next),
                ],
            },
            EffectPass {
                name: "consumer".into(),
                cadence: EffectCadence::EveryFrame,
                async_eligible: false,
                accesses: vec![access(AccessKind::StorageRead, HistoryBinding::Result)],
            },
        ],
    };
    let mut graph = FrameGraph::default();
    // Reserve the renderer's existing history owner: effects have their own namespace.
    graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 4,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::History {
                owner: 1,
                epoch: 1,
                initialized: true,
            },
        )
        .expect("view history");
    let mut keys = BTreeSet::new();
    let reads = effect
        .append(
            &mut graph,
            PipelineKey {
                device_generation: 1,
                revision: 1,
                role: 0,
            },
            &mut keys,
            &[],
        )
        .expect("effect declaration");
    assert_eq!(reads.opaque_reads.len(), 1);
    let compiled = graph
        .compile(1, &keys)
        .expect("distinct history sides compile without in-pass aliasing");
    assert_eq!(
        compiled.pass_names().collect::<Vec<_>>(),
        ["effect 0: producer", "effect 1: consumer"]
    );
    effect.passes[0].async_eligible = true;
    let mut flagged = FrameGraph::default();
    effect
        .append(
            &mut flagged,
            PipelineKey {
                device_generation: 1,
                revision: 1,
                role: 0,
            },
            &mut keys,
            &[],
        )
        .expect("flagged graph");
    let flagged = flagged.compile(1, &keys).expect("flagged compile");
    assert_eq!(
        flagged.pass_names().collect::<Vec<_>>(),
        compiled.pass_names().collect::<Vec<_>>()
    );
    assert_eq!(flagged.async_eligible, ["effect 0: producer"]);
}

#[test]
fn tick_stage_rules_only_allow_committed_retained_compute_reads_in_3d() {
    use crate::error::EffectGraphError as Error;

    let mut graph = EffectGraph {
        resources: [
            ResourceScope::Persistent {
                owner: 0,
                epoch: 1,
                initialized: true,
            },
            ResourceScope::History {
                owner: 1,
                epoch: 1,
                initialized: true,
            },
            ResourceScope::Transient,
            ResourceScope::Imported { initialized: true },
        ]
        .map(|scope| {
            (
                ResourceDescriptor::Buffer {
                    bytes: 16,
                    usage: wgpu::BufferUsages::STORAGE,
                },
                scope,
            )
        })
        .to_vec(),
        tick_passes: vec![TickPass {
            name: "query field".into(),
            reads: vec![],
        }],
        ..Default::default()
    };
    let access = |resource, kind, history| EffectAccess {
        resource,
        range: 0..4,
        kind,
        view: None,
        history,
    };
    let read = AccessKind::StorageRead;
    graph.tick_passes[0].reads = vec![
        access(0, read, None),
        access(1, read, Some(HistoryBinding::Previous)),
    ];
    assert_eq!(graph.validate(), Ok(()));
    assert_eq!(
        graph.validate_2d_stages(),
        Err(Error::TickPassIn2d("query field".into()))
    );
    for history in [HistoryBinding::Next, HistoryBinding::Result] {
        graph.tick_passes[0].reads = vec![access(1, read, Some(history))];
        assert_eq!(
            graph.validate(),
            Err(Error::TickPassRequiresPrevious("query field".into()))
        );
    }
    let write = AccessKind::StorageWrite;
    for (resource, kind, history) in [
        (0, write, None),
        (1, write, Some(HistoryBinding::Next)),
        (2, write, None),
        (3, AccessKind::AttachmentLoad, None),
        (2, read, None),
        (3, AccessKind::SampledRead, None),
    ] {
        graph.tick_passes[0].reads = vec![access(resource, kind, history)];
        assert_eq!(
            graph.validate(),
            Err(Error::TickPassRequiresRetainedRead("query field".into()))
        );
    }
}

#[test]
fn tick_stage_rules_validate_texture_read_descriptors() {
    use crate::{
        error::EffectGraphError,
        graph::{GraphError, TextureKind},
    };

    let name = "query texture";
    let mut graph = EffectGraph {
        resources: vec![(
            ResourceDescriptor::Texture {
                dimension: TextureKind::D2,
                width: 8,
                height: 8,
                layers: 1,
                mips: 1,
                samples: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Persistent {
                owner: 0,
                epoch: 1,
                initialized: true,
            },
        )],
        tick_passes: vec![TickPass {
            name: name.into(),
            reads: vec![EffectAccess {
                resource: 0,
                range: 0..1,
                kind: AccessKind::SampledRead,
                view: None,
                history: None,
            }],
        }],
        ..Default::default()
    };
    assert_eq!(graph.validate(), Ok(()));

    let invalid_read = |source| {
        Err(EffectGraphError::InvalidTickRead {
            pass: name.into(),
            source,
        })
    };
    graph.tick_passes[0].reads[0].range = 1..2;
    assert_eq!(
        graph.validate(),
        invalid_read(GraphError::InvalidRange { pass: 0 })
    );
    graph.tick_passes[0].reads[0].range = 0..1;
    graph.tick_passes[0].reads[0].view = Some(wgpu::TextureViewDimension::D3);
    assert_eq!(
        graph.validate(),
        invalid_read(GraphError::InvalidTextureView { pass: 0 })
    );
    graph.tick_passes[0].reads[0].view = None;
    graph.tick_passes[0].reads[0].kind = AccessKind::StorageRead;
    assert_eq!(
        graph.validate(),
        invalid_read(GraphError::UsageMismatch { pass: 0 })
    );
}

#[test]
fn tick_stage_rules_enforce_device_storage_texture_support_per_pass() {
    use crate::{
        error::EffectGraphError,
        graph::{GraphError, TextureKind},
    };

    let texture = |format| ResourceDescriptor::Texture {
        dimension: TextureKind::D2,
        width: 8,
        height: 8,
        layers: 1,
        mips: 1,
        samples: 1,
        format,
        usage: wgpu::TextureUsages::STORAGE_BINDING,
    };
    let read = |resource| EffectAccess {
        resource,
        range: 0..1,
        kind: AccessKind::StorageRead,
        view: None,
        history: None,
    };
    let mut graph = EffectGraph {
        resources: [0, 1]
            .map(|owner| {
                (
                    texture(wgpu::TextureFormat::R32Uint),
                    ResourceScope::Persistent {
                        owner,
                        epoch: 1,
                        initialized: true,
                    },
                )
            })
            .to_vec(),
        tick_passes: vec![
            TickPass {
                name: "query first texture".into(),
                reads: vec![read(0)],
            },
            TickPass {
                name: "query second texture".into(),
                reads: vec![read(1)],
            },
        ],
        ..Default::default()
    };
    let limits = wgpu::Limits {
        max_storage_textures_per_shader_stage: 1,
        ..wgpu::Limits::default()
    };
    let validate = |graph: &EffectGraph| {
        graph.validate()?;
        graph.validate_tick_texture_support(&limits, wgpu::Features::empty())
    };
    assert_eq!(validate(&graph), Ok(()));

    let invalid_read = |source| {
        Err(EffectGraphError::InvalidTickRead {
            pass: "query second texture".into(),
            source,
        })
    };
    graph.resources[1].0 = texture(wgpu::TextureFormat::Rgba8UnormSrgb);
    assert_eq!(
        validate(&graph),
        invalid_read(GraphError::StorageFormat { pass: 1 })
    );

    graph.resources[1].0 = texture(wgpu::TextureFormat::R32Uint);
    graph.tick_passes[1].reads.push(read(0));
    assert_eq!(
        validate(&graph),
        invalid_read(GraphError::StorageTextureLimit { pass: 1 })
    );
}

fn layered_draw_graph() -> EffectGraph {
    use crate::graph::TextureKind;
    use AccessKind as A;
    use EffectCadence as C;
    use wgpu::{TextureFormat as F, TextureUsages as U};

    let access = |kind, range| EffectAccess {
        resource: 0,
        range,
        kind,
        view: None,
        history: None,
    };
    let pass = |name: &str, cadence, accesses| EffectPass {
        name: name.into(),
        cadence,
        async_eligible: false,
        accesses,
    };
    EffectGraph {
        resources: vec![(
            ResourceDescriptor::Texture {
                dimension: TextureKind::D2,
                width: 4,
                height: 4,
                layers: 2,
                mips: 2,
                samples: 1,
                format: F::Rgba16Float,
                usage: U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::STORAGE_BINDING,
            },
            ResourceScope::Persistent {
                owner: 0,
                epoch: 0,
                initialized: true,
            },
        )],
        passes: vec![
            pass(
                "left",
                C::EveryFrame,
                vec![access(A::AttachmentClear, 0..1)],
            ),
            pass(
                "right",
                C::Skippable,
                vec![access(A::AttachmentClear, 1..2)],
            ),
            pass("reader", C::EveryFrame, vec![access(A::SampledRead, 0..2)]),
            pass(
                "other mip",
                C::EveryFrame,
                vec![access(A::StorageWrite, 2..3)],
            ),
        ],
        opaque_reads: vec![access(A::SampledRead, 0..2)],
        ..EffectGraph::default()
    }
}

#[test]
fn simulate_draw_rules_validate_attachment_ranges_and_ownership() {
    use super::draw::check_draw_format;
    use crate::graph::TextureKind;
    use AccessKind as A;
    use EffectCadence as C;
    use wgpu::TextureFormat as F;

    type Edit = fn(&mut EffectGraph);

    let graph = layered_draw_graph();
    graph.validate().expect("disjoint layers and mips");
    let mut frame = FrameGraph::default();
    let mut keys = BTreeSet::new();
    graph
        .append(
            &mut frame,
            PipelineKey {
                device_generation: 1,
                revision: 1,
                role: 0,
            },
            &mut keys,
            &[],
        )
        .expect("declare draws and sampled reads");
    let compiled = frame
        .compile(1, &keys)
        .expect("draws ordered before the reads");
    assert_eq!(
        compiled.pass_names().take(3).collect::<Vec<_>>(),
        ["effect 0: left", "effect 1: right", "effect 2: reader"]
    );
    assert_eq!(
        graph.validate_2d_stages(),
        Err(draw_error(
            "left",
            "2D scenes do not support simulate draws"
        ))
    );

    let cases: &[(&str, &str, Edit)] = &[
        ("load", "left", |g| {
            g.passes[0].accesses[0].kind = A::AttachmentLoad;
        }),
        ("two attachments", "left", |g| {
            let other = g.passes[1].accesses[0].clone();
            g.passes[0].accesses.push(other);
        }),
        ("two layers", "left", |g| {
            g.passes[0].accesses[0].range = 0..2;
        }),
        ("two mips", "left", |g| g.passes[0].accesses[0].range = 0..4),
        ("draw overlap", "left", |g| {
            g.passes[1].accesses[0].range = 0..1;
        }),
        ("storage overlap", "left", |g| {
            g.passes[3].accesses[0].range = 0..1;
        }),
        ("copy overlap", "left", |g| {
            g.passes[3].accesses[0].range = 0..1;
            g.passes[3].accesses[0].kind = A::CopyWrite;
        }),
        ("time slice", "left", |g| {
            g.passes[0].cadence = C::TimeSliced;
        }),
        ("skipped transient", "right", |g| {
            g.resources[0].1 = ResourceScope::Transient;
        }),
        ("async draw", "left", |g| g.passes[0].async_eligible = true),
        ("history", "left", |g| {
            g.resources[0].1 = ResourceScope::History {
                owner: 0,
                epoch: 0,
                initialized: true,
            };
            g.passes[0].accesses[0].history = Some(HistoryBinding::Next);
        }),
        ("depth", "left", |g| {
            if let ResourceDescriptor::Texture { format, .. } = &mut g.resources[0].0 {
                *format = F::Depth32Float;
            }
        }),
        ("3D", "left", |g| {
            if let ResourceDescriptor::Texture {
                dimension, layers, ..
            } = &mut g.resources[0].0
            {
                *dimension = TextureKind::D3 { depth: 2 };
                *layers = 1;
            }
        }),
        ("multisample", "left", |g| {
            if let ResourceDescriptor::Texture {
                samples,
                mips,
                layers,
                ..
            } = &mut g.resources[0].0
            {
                *samples = 4;
                *mips = 1;
                *layers = 1;
            }
        }),
    ];
    for (case, name, edit) in cases {
        let mut invalid = graph.clone();
        edit(&mut invalid);
        let err = invalid.validate().expect_err(case);
        assert!(
            matches!(err,
                EffectGraphError::SimulateDraw { ref pass, .. }
                | EffectGraphError::CadenceRequiresRetainedOutput(ref pass)
                | EffectGraphError::AsyncRequiresCompute(ref pass) if pass == name
            ),
            "{case}: {err}"
        );
    }

    let mut transient = graph.clone();
    transient.resources[0].1 = ResourceScope::Transient;
    transient.passes[1].cadence = C::EveryFrame;
    let mut frame = FrameGraph::default();
    let mut keys = BTreeSet::new();
    transient
        .append(
            &mut frame,
            PipelineKey {
                device_generation: 1,
                revision: 1,
                role: 0,
            },
            &mut keys,
            &[],
        )
        .expect("declare transient effect-owned draw target");
    let compiled = frame
        .compile(1, &keys)
        .expect("both draw producers initialize the sampled layers");
    assert_eq!(
        compiled.transient_slots, 0,
        "the prepared view already has physical storage"
    );

    let features = wgpu::Features::empty();
    for (format, blend, reason) in [
        (
            F::Rgb9e5Ufloat,
            false,
            "format is not supported as a colour attachment",
        ),
        (
            F::R32Uint,
            true,
            "colour attachment format does not support blending",
        ),
    ] {
        assert_eq!(
            check_draw_format(
                "left",
                format,
                format.guaranteed_format_features(features),
                blend
            ),
            Err(draw_error("left", reason))
        );
    }
    assert!(
        check_draw_format(
            "left",
            F::Rgba16Float,
            F::Rgba16Float.guaranteed_format_features(features),
            true
        )
        .is_ok()
    );
}
