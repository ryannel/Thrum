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
        graph.validate_2d_tick_stage(),
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
