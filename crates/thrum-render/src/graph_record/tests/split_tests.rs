use super::*;

fn split_plan(
    post: &BTreeSet<usize>,
    scene_after_surface: bool,
) -> Result<crate::graph::CompiledGraph, GraphError> {
    let mut graph = FrameGraph::default();
    let mut buffer = || {
        graph
            .resource(
                ResourceDescriptor::Buffer {
                    bytes: 8,
                    usage: wgpu::BufferUsages::COPY_DST,
                },
                ResourceScope::Imported { initialized: false },
            )
            .unwrap()
    };
    let scene = buffer();
    let surface = buffer();
    for (name, resource) in [("scene", scene), ("surface", surface.clone())] {
        graph.pass(Pass {
            name: name.into(),
            scope: PassScope::Shared,
            pipeline: None,
            accesses: vec![Access {
                resource,
                range: 0..8,
                kind: AccessKind::CopyWrite,
                store: true,
            }],
            external_side_effect: true,
        });
    }
    if scene_after_surface {
        graph.dependency(1, 0).unwrap();
    }
    graph.compile_late_surface(1, &BTreeSet::new(), &surface, post)
}

#[test]
fn late_surface_requires_only_phase_two_binding_and_preserves_order() {
    let plan = split_plan(&BTreeSet::from([1]), false).unwrap();
    assert_eq!(plan.pre_acquire_passes, Some(1));
    let executable = plan
        .bind(vec![
            binding(0),
            RecordBinding::<TestFamily>::new(1, PassKind::Copy, |frame, _| {
                if !frame.surface_bound {
                    return Err(());
                }
                frame.order[frame.written] = 1;
                frame.written += 1;
                Ok(())
            }),
        ])
        .unwrap();
    let mut frame = Frame::default();
    executable.execute_before_acquire(&mut frame).unwrap();
    assert_eq!(frame.written, 1);
    assert_eq!(executable.execute_after_acquire(&mut frame), Err(()));
    assert_eq!(frame.written, 1);
    frame.surface_bound = true;
    executable.execute_after_acquire(&mut frame).unwrap();
    assert_eq!(frame.order, [0, 1]);
}

#[test]
fn unrelated_scene_cannot_be_declared_post_acquire_or_depend_on_surface() {
    assert!(matches!(
        split_plan(&BTreeSet::from([0, 1]), false),
        Err(GraphError::SurfacePhase)
    ));
    assert!(matches!(
        split_plan(&BTreeSet::from([1]), true),
        Err(GraphError::SurfacePhase)
    ));
    assert!(matches!(
        split_plan(&BTreeSet::new(), false),
        Err(GraphError::SurfacePhase)
    ));
}
