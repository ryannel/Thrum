use super::*;
use crate::pass_key::PassKey;
use crate::timing::TimingCategory;

#[test]
fn only_live_variant_uses_determine_attachment_storage() {
    let scopes = [
        ResourceScope::Transient,
        ResourceScope::History {
            owner: 1,
            epoch: 1,
            initialized: false,
        },
        ResourceScope::Persistent {
            owner: 1,
            epoch: 1,
            initialized: false,
        },
        ResourceScope::Imported { initialized: false },
    ];
    for scope in scopes {
        for explicit_store in [false, true] {
            let mut graph = FrameGraph::default();
            let depth = graph
                .resource(
                    ResourceDescriptor::Texture {
                        dimension: TextureKind::D2,
                        width: 4,
                        height: 4,
                        layers: 1,
                        mips: 1,
                        samples: 1,
                        format: wgpu::TextureFormat::Depth32Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_SRC,
                    },
                    scope,
                )
                .unwrap();
            graph.pass(Pass {
                name: "forward".into(),
                scope: PassScope::View(1),
                pipeline: None,
                accesses: vec![Access {
                    view: None,
                    resource: depth.clone(),
                    range: 0..1,
                    kind: AccessKind::AttachmentClear,
                    store: explicit_store,
                }],
                external_side_effect: true,
                timing: TimingCategory::Effects,
            });
            let day = graph.compile(1, &BTreeSet::new()).unwrap();
            assert_eq!(
                day.memoryless_eligible(&depth),
                scope == ResourceScope::Transient && !explicit_store
            );
            for reader in [
                AccessKind::SampledRead,
                AccessKind::CopyRead,
                AccessKind::AttachmentLoad,
            ] {
                let readback = PassBody {
                    scope: PassScope::View(1),
                    pipeline: None,
                    accesses: vec![Access {
                        view: None,
                        resource: depth.clone(),
                        range: 0..1,
                        kind: reader,
                        store: false,
                    }],
                    external_side_effect: true,
                };
                let capture = graph
                    .compile_surface_variant(
                        Some((PassKey::Readback, readback)),
                        1,
                        &BTreeSet::new(),
                        None,
                        None,
                    )
                    .unwrap();
                assert!(!capture.memoryless_eligible(&depth));
                assert_eq!(
                    day.memoryless_eligible(&depth),
                    scope == ResourceScope::Transient && !explicit_store
                );
            }
        }
    }
}

#[test]
fn transient_storage_respects_locked_wgpu_layer_and_mip_restrictions() {
    for (layers, mips, eligible) in [(1, 1, true), (2, 1, false), (1, 2, false)] {
        let mut graph = FrameGraph::default();
        let depth = graph
            .resource(
                ResourceDescriptor::Texture {
                    dimension: TextureKind::D2,
                    width: 4,
                    height: 4,
                    layers,
                    mips,
                    samples: 1,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                },
                ResourceScope::Transient,
            )
            .unwrap();
        graph.pass(Pass {
            name: "render".into(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: vec![Access {
                view: None,
                resource: depth.clone(),
                range: 0..1,
                kind: AccessKind::AttachmentClear,
                store: false,
            }],
            external_side_effect: true,
            timing: TimingCategory::Effects,
        });
        assert_eq!(
            graph
                .compile(1, &BTreeSet::new())
                .unwrap()
                .memoryless_eligible(&depth),
            eligible
        );
    }
}
