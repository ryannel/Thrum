use super::*;
use crate::synthetic::SyntheticView;

#[test]
fn policy_preserves_fixed_reciprocal_and_leaves_metered_value_gpu_owned() {
    for exposure in [1.0 / 16_000.0, 0.25, 1.234_567, 10.0] {
        let view = SyntheticView::default().with_exposure(exposure).unwrap();
        let packed = view.exposure_policy(0.5, true);
        assert_eq!(&packed[..4], &(1.0 / exposure).to_le_bytes());
        assert_eq!(&view.pack_view(&view, 0.5)[148..152], &packed[..4]);
        assert_eq!(&packed[16..20], &0_u32.to_le_bytes());
    }
    let view = SyntheticView::default()
        .with_incident_exposure(0.0, 0.0, 18.0)
        .unwrap();
    let packed = view.exposure_policy(1.0 / 16_000.0, true);
    assert_eq!(&packed[..4], &(1.0_f32 / (1.0 / 16_000.0)).to_le_bytes());
    assert_eq!(&packed[16..20], &1_u32.to_le_bytes());
    let legacy = view.pack_view(&view, 0.5);
    assert_eq!(legacy.len(), 160);
    assert_eq!(&legacy[148..152], &0.0_f32.to_le_bytes());
    assert_eq!(
        &legacy[152..160],
        &[0; 8],
        "jitter bytes retain their meaning"
    );
}

#[test]
fn policy_refuses_unrepresentable_or_reversed_ev_settings() {
    for (compensation, min, max) in [
        (f32::NAN, 0.0, 18.0),
        (0.0, f32::NEG_INFINITY, 18.0),
        (0.0, 0.0, f32::INFINITY),
        (0.0, 18.0, 0.0),
        (25.0, 0.0, 18.0),
        (0.0, -25.0, 18.0),
        (0.0, 0.0, 25.0),
    ] {
        assert!(
            SyntheticView::default()
                .with_incident_exposure(compensation, min, max)
                .is_err()
        );
    }
    assert!(
        SyntheticView::default()
            .with_incident_exposure(-2.0, 0.0, 0.0)
            .is_ok()
    );
}

#[test]
fn policy_shader_layouts_validate_without_a_device() {
    for (source, expected) in [
        (
            include_str!("../exposure.wgsl"),
            &[("Policy", 32), ("ExposureSample", 32), ("Serials", 32)][..],
        ),
        (
            crate::synthetic::display::SHADER,
            &[("ExposureSample", 32)][..],
        ),
    ] {
        let module = wgpu::naga::front::wgsl::parse_str(source).unwrap();
        wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        for &(name, expected_span) in expected {
            let ty = module
                .types
                .iter()
                .find(|(_, ty)| ty.name.as_deref() == Some(name))
                .unwrap()
                .1;
            let wgpu::naga::TypeInner::Struct { span, .. } = ty.inner else {
                panic!("expected struct");
            };
            assert_eq!(span, expected_span, "{name}");
        }
    }
}

#[test]
fn policy_graph_reads_the_actual_diffuse_write_before_tone() {
    use std::collections::BTreeSet;
    let mut graph = FrameGraph::default();
    let directional = graph
        .resource(
            ResourceDescriptor::Buffer {
                bytes: 64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Imported { initialized: true },
        )
        .unwrap();
    graph.pass(Pass {
        name: "physical diffuse".into(),
        scope: PassScope::View(1),
        pipeline: None,
        accesses: vec![Access {
            resource: directional.clone(),
            range: 48..64,
            kind: AccessKind::StorageWrite,
            store: true,
        }],
        external_side_effect: false,
    });
    let key = PipelineKey {
        device_generation: 41,
        revision: 11,
        role: 6,
    };
    let sample = ExposureResolve::append_graph(&mut graph, key, directional).unwrap();
    graph.pass(Pass {
        name: "tone".into(),
        scope: PassScope::View(1),
        pipeline: None,
        accesses: vec![Access {
            resource: sample,
            range: 0..SAMPLE_BYTES,
            kind: AccessKind::UniformRead,
            store: true,
        }],
        external_side_effect: true,
    });
    let compiled = graph.compile(41, &BTreeSet::from([key])).unwrap();
    assert_eq!(compiled.dependencies, [(0, 1), (1, 2)]);
}

#[cfg(not(target_arch = "wasm32"))]
mod gpu;
