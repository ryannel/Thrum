//! CPU graph, ABI and temporal metadata contract tests.
use std::collections::BTreeSet;
use thrum_render::abi::*;
use thrum_render::graph::*;
use thrum_render::history::*;

fn buffer(graph: &mut FrameGraph, scope: ResourceScope) -> ResourceId {
    graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 32,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            },
            scope,
        )
        .expect("valid test fixture")
}
fn pass(graph: &mut FrameGraph, accesses: Vec<Access>) {
    graph.pass(Pass {
        name: "test".into(),
        scope: PassScope::View(1),
        pipeline: None,
        accesses,
        external_side_effect: false,
    });
}
fn access(resource: &ResourceId, range: std::ops::Range<u64>, kind: AccessKind) -> Access {
    Access {
        resource: resource.clone(),
        range,
        kind,
        store: true,
    }
}

#[test]
fn read_write_ordering_and_disjoint_ranges() {
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Imported { initialized: true });
    pass(
        &mut graph,
        vec![access(&resource, 0..16, AccessKind::StorageRead)],
    );
    pass(
        &mut graph,
        vec![access(&resource, 16..32, AccessKind::StorageWrite)],
    );
    pass(
        &mut graph,
        vec![access(&resource, 0..8, AccessKind::StorageWrite)],
    );
    pass(
        &mut graph,
        vec![access(&resource, 4..12, AccessKind::StorageRead)],
    );
    pass(
        &mut graph,
        vec![access(&resource, 0..8, AccessKind::StorageWrite)],
    );
    let compiled = graph
        .compile(1, &BTreeSet::new())
        .expect("valid test fixture");
    assert_eq!(
        compiled.dependencies,
        [(0, 2), (0, 4), (2, 3), (2, 4), (3, 4)]
    );
    assert!(
        compiled
            .hazards
            .iter()
            .any(|h| h.before == 2 && h.after == 3 && h.range == (4..8))
    );
    assert_eq!(compiled.lifetimes[0].first, 0);
    assert_eq!(compiled.lifetimes[0].last, 4);
    assert_eq!(compiled.transient_slots, 0);
}

#[test]
fn adjacent_writes_initialize_union_but_discard_invalidates_only_its_range() {
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Transient);
    pass(
        &mut graph,
        vec![
            access(&resource, 0..16, AccessKind::CopyWrite),
            access(&resource, 16..32, AccessKind::CopyWrite),
        ],
    );
    pass(
        &mut graph,
        vec![access(&resource, 0..32, AccessKind::StorageRead)],
    );
    assert!(graph.compile(1, &BTreeSet::new()).is_ok());
    let mut discard = access(&resource, 8..16, AccessKind::StorageWrite);
    discard.store = false;
    pass(&mut graph, vec![discard]);
    pass(
        &mut graph,
        vec![access(&resource, 16..32, AccessKind::StorageRead)],
    );
    assert!(graph.compile(1, &BTreeSet::new()).is_ok());
    pass(
        &mut graph,
        vec![access(&resource, 7..9, AccessKind::StorageRead)],
    );
    assert_eq!(
        graph.compile(1, &BTreeSet::new()).unwrap_err(),
        GraphError::UninitializedRead { pass: 4, resource }
    );
}

#[test]
fn undefined_read_load_and_conflicting_same_pass_refuse() {
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Transient);
    pass(
        &mut graph,
        vec![access(&resource, 0..32, AccessKind::StorageReadWrite)],
    );
    assert!(matches!(
        graph.compile(1, &BTreeSet::new()),
        Err(GraphError::UninitializedRead { .. })
    ));
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Imported { initialized: true });
    pass(
        &mut graph,
        vec![
            access(&resource, 0..8, AccessKind::StorageRead),
            access(&resource, 4..12, AccessKind::StorageWrite),
        ],
    );
    assert_eq!(
        graph.compile(1, &BTreeSet::new()).unwrap_err(),
        GraphError::ConflictingPassAccess { pass: 0 }
    );
}

#[test]
fn disjoint_same_pass_buffer_usages_follow_whole_buffer_compatibility() {
    for (first, second, compatible) in [
        (AccessKind::StorageWrite, AccessKind::VertexRead, false),
        (AccessKind::VertexRead, AccessKind::StorageWrite, false),
        (AccessKind::StorageWrite, AccessKind::StorageRead, false),
        (AccessKind::CopyWrite, AccessKind::StorageWrite, false),
        (AccessKind::StorageRead, AccessKind::VertexRead, true),
        (AccessKind::StorageWrite, AccessKind::StorageReadWrite, true),
    ] {
        let mut graph = FrameGraph::default();
        let resource = graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 32,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Imported { initialized: true },
            )
            .expect("valid test fixture");
        pass(
            &mut graph,
            vec![
                access(&resource, 0..16, first),
                access(&resource, 16..32, second),
            ],
        );
        let result = graph.compile(1, &BTreeSet::new());
        if compatible {
            assert!(result.is_ok(), "{first:?} + {second:?}: {result:?}");
        } else {
            assert_eq!(
                result.unwrap_err(),
                GraphError::ConflictingPassAccess { pass: 0 },
                "{first:?} + {second:?}"
            );
        }
    }
}

#[test]
fn separate_pass_buffer_hazards_and_same_pass_texture_usage_keep_ranges() {
    let mut graph = FrameGraph::default();
    let resource = graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 32,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            },
            ResourceScope::Imported { initialized: true },
        )
        .expect("valid test fixture");
    pass(
        &mut graph,
        vec![access(&resource, 0..16, AccessKind::StorageWrite)],
    );
    pass(
        &mut graph,
        vec![access(&resource, 16..32, AccessKind::VertexRead)],
    );
    let plan = graph
        .compile(1, &BTreeSet::new())
        .expect("valid test fixture");
    assert_eq!(plan.dependencies, Vec::new());
    assert_eq!(plan.hazards, Vec::new());

    let mut graph = FrameGraph::default();
    let texture = graph
        .resource(
            ResourceDescriptor::Texture {
                width: 16,
                height: 16,
                layers: 2,
                mips: 1,
                samples: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Imported { initialized: true },
        )
        .expect("valid test fixture");
    pass(
        &mut graph,
        vec![
            access(&texture, 0..1, AccessKind::AttachmentClear),
            access(&texture, 1..2, AccessKind::SampledRead),
        ],
    );
    assert!(graph.compile(1, &BTreeSet::new()).is_ok());
}

#[test]
fn reuse_requires_exact_descriptor_nonoverlap_and_transient_ownership() {
    let mut graph = FrameGraph::default();
    let a = buffer(&mut graph, ResourceScope::Transient);
    let b = buffer(&mut graph, ResourceScope::Transient);
    let overlapping = buffer(&mut graph, ResourceScope::Transient);
    let persistent = buffer(
        &mut graph,
        ResourceScope::Persistent {
            owner: 9,
            epoch: 2,
            initialized: false,
        },
    );
    let different = graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 64,
                usage: wgpu::BufferUsages::COPY_DST,
            },
            ResourceScope::Transient,
        )
        .expect("valid test fixture");
    let _unused = buffer(&mut graph, ResourceScope::Transient);
    pass(
        &mut graph,
        vec![
            access(&a, 0..32, AccessKind::CopyWrite),
            access(&overlapping, 0..32, AccessKind::CopyWrite),
        ],
    );
    pass(
        &mut graph,
        vec![
            access(&b, 0..32, AccessKind::CopyWrite),
            access(&overlapping, 0..32, AccessKind::StorageRead),
            access(&persistent, 0..32, AccessKind::CopyWrite),
            access(&different, 0..64, AccessKind::CopyWrite),
        ],
    );
    let plan = graph
        .compile(1, &BTreeSet::new())
        .expect("valid test fixture");
    let slot = |id: &ResourceId| {
        plan.lifetimes
            .iter()
            .find(|l| &l.resource == id)
            .expect("valid test fixture")
            .transient_slot
    };
    assert_eq!(slot(&a), slot(&b));
    assert_ne!(slot(&a), slot(&overlapping));
    assert_ne!(slot(&a), slot(&different));
    assert_eq!(slot(&persistent), None);
    assert_eq!(plan.lifetimes.len(), 5);
    assert_eq!(plan.transient_slots, 3);
}

#[test]
fn texture_subresources_require_their_own_producer() {
    let mut graph = FrameGraph::default();
    let texture = graph
        .resource(
            ResourceDescriptor::Texture {
                width: 64,
                height: 64,
                layers: 2,
                mips: 3,
                samples: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Transient,
        )
        .expect("valid test fixture");
    pass(
        &mut graph,
        vec![access(&texture, 0..1, AccessKind::AttachmentClear)],
    );
    pass(
        &mut graph,
        vec![access(&texture, 0..1, AccessKind::SampledRead)],
    );
    assert!(graph.compile(1, &BTreeSet::new()).is_ok());
    pass(
        &mut graph,
        vec![access(&texture, 1..2, AccessKind::AttachmentLoad)],
    );
    assert!(matches!(
        graph.compile(1, &BTreeSet::new()),
        Err(GraphError::UninitializedRead { pass: 2, .. })
    ));
}

#[test]
fn wrong_graph_bounds_usage_and_unprepared_generations_refuse() {
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Imported { initialized: true });
    let mut another = FrameGraph::default();
    buffer(&mut another, ResourceScope::Imported { initialized: true });
    pass(
        &mut another,
        vec![access(&resource, 0..32, AccessKind::StorageRead)],
    );
    assert_eq!(
        another.compile(1, &BTreeSet::new()).unwrap_err(),
        GraphError::InvalidRange { pass: 0 }
    );
    pass(
        &mut graph,
        vec![access(&resource, 0..33, AccessKind::StorageRead)],
    );
    assert_eq!(
        graph.compile(1, &BTreeSet::new()).unwrap_err(),
        GraphError::InvalidRange { pass: 0 }
    );
    let mut graph = FrameGraph::default();
    let resource = buffer(&mut graph, ResourceScope::Imported { initialized: true });
    pass(
        &mut graph,
        vec![access(&resource, 0..32, AccessKind::VertexRead)],
    );
    assert_eq!(
        graph.compile(1, &BTreeSet::new()).unwrap_err(),
        GraphError::UsageMismatch { pass: 0 }
    );
    let mut graph = FrameGraph::default();
    let key = PipelineKey {
        device_generation: 3,
        revision: 9,
        role: 2,
    };
    graph.pass(Pass {
        name: "required".into(),
        scope: PassScope::Shared,
        pipeline: Some(key),
        accesses: vec![],
        external_side_effect: true,
    });
    assert!(matches!(
        graph.compile(3, &BTreeSet::new()),
        Err(GraphError::PipelineNotReady { .. })
    ));
    assert!(matches!(
        graph.compile(4, &BTreeSet::from([key])),
        Err(GraphError::PipelineNotReady { .. })
    ));
    assert!(graph.compile(3, &BTreeSet::from([key])).is_ok());
}

#[test]
fn abi_encodes_semantic_offsets_and_rejects_wrong_references() {
    let textures = TextureClasses::defaults(&wgpu::Limits::default()).expect("valid test fixture");
    let material = MaterialRecord {
        base_rgba: [0.25, 0.5, 0.75, 1.0],
        emissive_cutoff: [2.0, 3.0, 4.0, 0.5],
        surface: [0.8, 0.6, 0.4, 2.0],
        flags: [11, 22, 33, 44],
        texture_layers: [0; 4],
    };
    let bytes = material.pack(&textures).expect("valid test fixture");
    assert_eq!(bytes.len(), 80);
    for (offset, expected) in [(0, 0.25_f32), (16, 2.0), (28, 0.5), (32, 0.8), (44, 2.0)] {
        assert_eq!(&bytes[offset..offset + 4], &expected.to_le_bytes());
    }
    assert_eq!(&bytes[60..64], &44_u32.to_le_bytes());
    let mut bad = material.clone();
    bad.texture_layers[2] = 1;
    assert_eq!(bad.pack(&textures), Err(AbiError::TextureLayer));
    let mut bad = material.clone();
    bad.base_rgba[0] = f32::NAN;
    assert_eq!(bad.pack(&textures), Err(AbiError::InvalidFactor));
    let mut instance = InstanceRecord::identity(17, 1);
    instance.current[3][0] = 3.25;
    instance.previous[2][2] = 0.75;
    instance.ids[3] = 29;
    let bytes = instance.pack(2).expect("valid test fixture");
    assert_eq!(bytes.len(), 192);
    assert_eq!(&bytes[48..52], &3.25_f32.to_le_bytes());
    assert_eq!(&bytes[104..108], &0.75_f32.to_le_bytes());
    assert_eq!(&bytes[176..180], &17_u32.to_le_bytes());
    assert_eq!(&bytes[180..184], &1_u32.to_le_bytes());
    assert_eq!(&bytes[188..192], &29_u32.to_le_bytes());
    assert_eq!(instance.pack(1), Err(AbiError::MaterialIndex));
    instance.normal[0][0] = f32::INFINITY;
    assert_eq!(instance.pack(2), Err(AbiError::InvalidTransform));
}

#[test]
fn texture_class_and_storage_capacity_are_not_silently_truncated() {
    let limits = wgpu::Limits::default();
    let defaults = TextureClasses::defaults(&limits).expect("valid test fixture");
    assert_eq!(
        defaults.arrays()[0].class.format(),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
    assert_eq!(
        defaults.arrays()[2].class.format(),
        wgpu::TextureFormat::Rgba8Unorm
    );
    let mut arrays = *defaults.arrays();
    arrays[0].layers = limits.max_texture_array_layers + 1;
    assert!(matches!(
        TextureClasses::new(arrays, &limits),
        Err(AbiError::TextureClass)
    ));
    let mut arrays = *defaults.arrays();
    arrays.swap(0, 1);
    assert!(matches!(
        TextureClasses::new(arrays, &limits),
        Err(AbiError::TextureClass)
    ));
    assert_eq!(table_bytes(0, 100, &limits), Err(AbiError::TableCapacity));
    assert_eq!(
        table_bytes(2, 10_000, &limits).expect("valid test fixture"),
        (160, 1_920_000)
    );
    let limited = wgpu::Limits {
        max_storage_buffer_binding_size: 1024,
        ..limits
    };
    assert_eq!(
        table_bytes(2, 10_000, &limited),
        Err(AbiError::TableCapacity)
    );
}

#[test]
fn histories_advance_only_for_exact_submitted_and_accepted_samples() {
    let key = HistoryKey {
        view: 1,
        provider: 2,
        extent: [640, 360],
        device_generation: 3,
        content_epoch: 4,
        profile_epoch: 5,
        origin_epoch: 6,
        correspondence_epoch: 7,
    };
    let mut history = ViewHistory::new(key);
    let first = Sample {
        serial: 10,
        time_seconds: 1.0,
        committed_endpoint: 90,
    };
    assert!(!history.accept(key, first));
    assert!(history.submitted(key, first));
    assert_eq!(history.accepted(), None);
    assert!(history.accept(key, first));
    let next = Sample {
        serial: 12,
        time_seconds: 1.1,
        committed_endpoint: 92,
    };
    assert!(history.submitted(key, next));
    assert_eq!(history.last_submitted(), Some(next));
    assert_eq!(history.accepted(), None);
    assert!(!history.accept(key, first));
    assert!(!history.submitted(key, first));
    assert!(history.accept(key, next));
    let resized = HistoryKey {
        extent: [1280, 720],
        ..key
    };
    history.synchronize(resized);
    assert_eq!(history.last_submitted(), None);
    assert!(!history.submitted(key, next));
    assert!(!history.submitted(
        resized,
        Sample {
            time_seconds: f64::NAN,
            ..next
        }
    ));
    assert!(history.submitted(resized, next));
}
