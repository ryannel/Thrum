//! Reflect bound records from every owned WGSL file, including declarations appended to scene WGSL.
use std::mem::{offset_of, size_of};
use thrum_asset::layout::{Meshlet, MeshletBounds};
use thrum_asset_gpu::records::{CookedVolume, Cull, Instance, Material};
use thrum_test_support::{record, records};

// Parse the actual declarations independently of function bodies that depend on scene-owned
// shader libraries. No list of bound records or bindings is supplied to the discovery pass.
fn check(source: &str) -> usize {
    // Each entry names an actual Pod type and its compiler-derived field offsets, sizes and scalar kinds, under the
    // names the WGSL gives them.
    let layouts = [
        record!(
            Material as "MaterialRecord",
            base_rgba as "base",
            emissive_cutoff,
            surface,
            flags,
            textures as "texture_layers",
        ),
        record!(Cull, view_proj, model, camera_scale, counts),
        record!(CookedVolume, model, inverse_model, params),
        record!(
            MeshletBounds as "Bounds",
            center,
            radius,
            cone_apex,
            cone_cutoff,
            cone_axis,
            reserved,
        ),
        record!(Instance as "CookedMeshPlacement", model, history as "draw"),
        record!(
            Meshlet as "CookedMeshlet",
            vertex_offset,
            triangle_offset,
            vertex_count,
            triangle_count,
        ),
    ];
    let source = format!(
        "enable wgpu_mesh_shader;\n{}",
        records::declarations(source)
    );
    records::bound("owned shader declarations", &source, &layouts).len()
}

#[test]
fn every_owned_bound_record_has_a_pod_layout() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    // Compose the owned declarations so both cull routes see the shared Bounds. The scene
    // supplies VertexOutput. Its raster interface is checked by the real Metal pipeline proof.
    let owned: Vec<_> = std::fs::read_dir(root)
        .expect("sources")
        .map(|e| e.expect("source").path())
        .filter(|p| p.extension().is_some_and(|e| e == "wgsl"))
        .map(|p| std::fs::read_to_string(p).expect("WGSL"))
        .collect();
    let source = format!(
        "struct VertexOutput {{ @builtin(position) position: vec4<f32> }}\n{}",
        owned.join("\n")
    );
    let count = check(&source);
    assert_eq!(
        count, 5,
        "culling, placement, meshlet and volume bindings have records"
    );
    assert_eq!(size_of::<Material>(), thrum_render::abi::MATERIAL_BYTES);
    assert_eq!(
        [
            offset_of!(Material, base_rgba),
            offset_of!(Material, emissive_cutoff),
            offset_of!(Material, surface),
            offset_of!(Material, flags),
            offset_of!(Material, textures)
        ],
        thrum_render::abi::MATERIAL_OFFSETS
    );
    assert_eq!(size_of::<Instance>(), 80);
    assert_eq!(offset_of!(Instance, history), 64);
}

#[test]
#[should_panic(expected = "has no Rust Pod record")]
fn a_new_bound_struct_is_discovered() {
    check(
        "struct Unknown { value: vec4<f32> }\n@group(7) @binding(0) var<uniform> unknown: Unknown;",
    );
}

#[test]
fn material_upload_matches_the_renderers_actual_declaration() {
    let declaration = thrum_render::abi::material_wgsl_declaration().expect("scene material ABI");
    assert_eq!(
        check(&format!(
            "{declaration}\n@group(2) @binding(0) var<storage, read> materials: array<MaterialRecord>;"
        )),
        1
    );
}

#[test]
#[should_panic(expected = "MaterialRecord size")]
fn changed_renderer_material_layout_is_refused() {
    let declaration = thrum_render::abi::material_wgsl_declaration().expect("scene material ABI");
    let changed = declaration.replace(
        "texture_layers: vec4<u32>,",
        "texture_layers: vec4<u32>, extra: vec4<f32>,",
    );
    check(&format!(
        "{changed}\n@group(2) @binding(0) var<storage, read> materials: array<MaterialRecord>;"
    ));
}

#[test]
#[should_panic(expected = "Cull size")]
fn changed_record_layout_is_refused() {
    check(
        &thrum_asset_gpu::records::CULL_SHADER
            .replace("counts: vec4<u32>,", "counts: vec4<u32>, extra: vec4<f32>,"),
    );
}
