//! EFFECT-GRAPH M1: real Metal texture-kind and preparation validation witnesses.
#![cfg(target_os = "macos")]
#[path = "graph_kinds_support/support.rs"]
mod graph_kinds_support;
use graph_kinds_support::{access, descriptor, device, pass};
use std::collections::BTreeSet;
use thrum_render::graph::{
    AccessKind as A, FrameGraph, GraphError as E, ResourceDescriptor, ResourceScope as S,
    TextureKind as K,
};

fn bad_shape(kind: K, layers: u32, samples: u32) -> E {
    let mut graph = FrameGraph::default();
    let mut desc = descriptor(kind, 4);
    if let ResourceDescriptor::Texture {
        layers: l,
        samples: s,
        ..
    } = &mut desc
    {
        *l = layers;
        *s = samples;
    }
    graph
        .resource(desc, S::Transient)
        .expect_err("invalid texture shape")
}
#[test]
fn refuses_volume_layers() {
    assert_eq!(bad_shape(K::D3 { depth: 4 }, 2, 1), E::InvalidVolume);
}
#[test]
fn refuses_volume_multisampling() {
    assert_eq!(bad_shape(K::D3 { depth: 4 }, 1, 4), E::InvalidVolume);
}
#[test]
fn refuses_cube_with_other_than_six_faces() {
    assert_eq!(bad_shape(K::Cube, 5, 1), E::InvalidCube);
}
#[test]
fn refuses_nonsquare_cube() {
    let mut graph = FrameGraph::default();
    let mut desc = descriptor(K::Cube, 4);
    if let ResourceDescriptor::Texture { height, .. } = &mut desc {
        *height = 2;
    }
    assert_eq!(
        graph
            .resource(desc, S::Transient)
            .expect_err("nonsquare cube"),
        E::InvalidCube
    );
}

fn refused(
    kind: K,
    range: std::ops::Range<u64>,
    usage: A,
    view: Option<wgpu::TextureViewDimension>,
    error: &E,
) {
    let mut graph = FrameGraph::default();
    let resource = graph
        .resource(descriptor(kind, 4), S::Imported { initialized: true })
        .expect("valid graph-kind fixture");
    let mut a = access(&resource, range, usage);
    a.view = view;
    graph.pass(pass("misuse", vec![a]));
    assert_eq!(
        graph
            .compile(1, &BTreeSet::new())
            .expect_err("invalid declaration must be refused"),
        *error
    );
}

#[test]
fn refuses_cube_storage_view() {
    refused(
        K::Cube,
        0..6,
        A::StorageWrite,
        None,
        &E::CubeStorageView { pass: 0 },
    );
}
#[test]
fn refuses_volume_attachment() {
    refused(
        K::D3 { depth: 4 },
        0..1,
        A::AttachmentClear,
        None,
        &E::VolumeAttachment { pass: 0 },
    );
}
#[test]
fn refuses_mip_range_outside_descriptor() {
    refused(
        K::D3 { depth: 4 },
        3..4,
        A::SampledRead,
        None,
        &E::InvalidRange { pass: 0 },
    );
}
#[test]
fn refuses_layer_range_outside_descriptor() {
    refused(
        K::Cube,
        17..19,
        A::SampledRead,
        None,
        &E::InvalidRange { pass: 0 },
    );
}
#[test]
fn refuses_storage_binding_spanning_mips() {
    refused(
        K::D2,
        0..8,
        A::StorageWrite,
        None,
        &E::InvalidTextureView { pass: 0 },
    );
}
#[test]
fn refuses_incompatible_view() {
    refused(
        K::D3 { depth: 4 },
        0..1,
        A::SampledRead,
        Some(wgpu::TextureViewDimension::D2),
        &E::InvalidTextureView { pass: 0 },
    );
}

#[test]
fn transient_reuse_requires_identical_dimension() {
    let mut graph = FrameGraph::default();
    for kind in [K::D2, K::D3 { depth: 1 }, K::D2] {
        let mut desc = descriptor(kind, 4);
        // Same physical sizes/layers/mips; only the dimension differs.
        if let ResourceDescriptor::Texture { layers, .. } = &mut desc {
            *layers = 1;
        }
        let resource = graph
            .resource(desc, S::Transient)
            .expect("valid graph-kind fixture");
        graph.pass(pass(
            "write",
            vec![access(&resource, 0..1, A::StorageWrite)],
        ));
        graph.pass(pass("read", vec![access(&resource, 0..1, A::SampledRead)]));
    }
    let compiled = graph
        .compile(1, &BTreeSet::new())
        .expect("valid graph-kind fixture");
    assert_eq!(compiled.transient_slots, 2);
    let slots: Vec<_> = compiled
        .lifetimes
        .iter()
        .map(|life| life.transient_slot)
        .collect();
    assert_eq!(slots, [Some(0), Some(1), Some(0)]);
}

#[test]
fn capability_record_refuses_storage_access_counts_and_extents() {
    let (_, _, caps) = device(1);
    for (format, kind) in [
        (wgpu::TextureFormat::Rgba8UnormSrgb, A::StorageWrite),
        (wgpu::TextureFormat::Rgba32Float, A::StorageReadWrite),
    ] {
        let mut graph = FrameGraph::default();
        let mut desc = descriptor(K::D3 { depth: 4 }, 4);
        if let ResourceDescriptor::Texture { format: f, .. } = &mut desc {
            *f = format;
        }
        let resource = graph
            .resource(desc, S::Imported { initialized: true })
            .expect("valid graph-kind fixture");
        graph.pass(pass(
            "unsupported access",
            vec![access(&resource, 0..1, kind)],
        ));
        assert_eq!(
            graph
                .compile_with_capabilities(&caps, &BTreeSet::new())
                .expect_err("invalid declaration must be refused"),
            E::StorageFormat { pass: 0 }
        );
    }
    let mut graph = FrameGraph::default();
    let mut accesses = Vec::new();
    for _ in 0..=caps.limits().max_storage_textures_per_shader_stage {
        let resource = graph
            .resource(descriptor(K::D3 { depth: 4 }, 4), S::Transient)
            .expect("valid graph-kind fixture");
        accesses.push(access(&resource, 0..1, A::StorageWrite));
    }
    graph.pass(pass("too many bindings", accesses));
    assert_eq!(
        graph
            .compile_with_capabilities(&caps, &BTreeSet::new())
            .expect_err("invalid declaration must be refused"),
        E::StorageTextureLimit { pass: 0 }
    );
    for kind in [K::D2, K::Cube, K::D3 { depth: 4 }] {
        let mut graph = FrameGraph::default();
        let limit = if matches!(kind, K::D3 { .. }) {
            caps.limits().max_texture_dimension_3d
        } else {
            caps.limits().max_texture_dimension_2d
        };
        graph
            .resource(descriptor(kind, limit + 1), S::Transient)
            .expect("valid graph-kind fixture");
        assert_eq!(
            graph
                .compile_with_capabilities(&caps, &BTreeSet::new())
                .expect_err("invalid declaration must be refused"),
            E::TextureExtentLimit
        );
    }
    let mut graph = FrameGraph::default();
    graph
        .resource(
            descriptor(
                K::D3 {
                    depth: caps.limits().max_texture_dimension_3d + 1,
                },
                4,
            ),
            S::Transient,
        )
        .expect("valid graph-kind fixture");
    assert_eq!(
        graph
            .compile_with_capabilities(&caps, &BTreeSet::new())
            .expect_err("invalid declaration must be refused"),
        E::TextureExtentLimit
    );
}

#[test]
fn metal_every_texel_every_mip_all_scopes_resize_and_replacement() {
    graph_kinds_support::prove_kinds();
}

#[test]
fn existing_scene_variants_keep_compiled_order_storage_and_slots() {
    graph_kinds_support::prove_scenes();
}
