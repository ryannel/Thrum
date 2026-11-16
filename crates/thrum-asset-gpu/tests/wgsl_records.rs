//! Reflect bound records from every owned WGSL file, including declarations appended to scene WGSL.
use std::collections::{BTreeMap, BTreeSet};
use std::mem::{offset_of, size_of};
use thrum_asset::layout::MeshletBounds;
use thrum_asset_gpu::records::{CookedVolume, Cull, Instance, Material};
use wgpu::naga::{self, TypeInner};

// Parse the actual declarations independently of function bodies that depend on scene-owned
// shader libraries. No list of bound records or bindings is supplied to the discovery pass.
fn declarations(source: &str) -> String {
    let mut result = String::new();
    for marker in ["struct ", "@group("] {
        let mut rest = source;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at..];
            let end = if marker == "struct " {
                rest.find('}').expect("struct closes") + 1
            } else {
                rest.find(';').expect("binding ends") + 1
            };
            result.push_str(&rest[..end]);
            result.push('\n');
            rest = &rest[end..];
        }
    }
    result
}

fn discover(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    found: &mut BTreeSet<naga::Handle<naga::Type>>,
) {
    match &module.types[ty].inner {
        TypeInner::Struct { members, .. } if found.insert(ty) => {
            for member in members {
                discover(module, member.ty, found);
            }
        }
        TypeInner::Array { base, .. } | TypeInner::BindingArray { base, .. } => {
            discover(module, *base, found);
        }
        _ => {}
    }
}

fn check(source: &str) -> usize {
    // Each entry names an actual Pod type and its compiler-derived field offsets.
    fn pod<T: bytemuck::Pod>(
        members: Vec<(&'static str, usize)>,
    ) -> (usize, Vec<(&'static str, usize)>) {
        (size_of::<T>(), members)
    }
    let layouts = BTreeMap::from([
        (
            "MaterialRecord",
            pod::<Material>(vec![
                ("base", offset_of!(Material, base_rgba)),
                ("emissive_cutoff", offset_of!(Material, emissive_cutoff)),
                ("surface", offset_of!(Material, surface)),
                ("flags", offset_of!(Material, flags)),
                ("texture_layers", offset_of!(Material, textures)),
            ]),
        ),
        (
            "Cull",
            pod::<Cull>(vec![
                ("view_proj", offset_of!(Cull, view_proj)),
                ("model", offset_of!(Cull, model)),
                ("camera_scale", offset_of!(Cull, camera_scale)),
                ("counts", offset_of!(Cull, counts)),
            ]),
        ),
        (
            "CookedVolume",
            pod::<CookedVolume>(vec![
                ("model", offset_of!(CookedVolume, model)),
                ("inverse_model", offset_of!(CookedVolume, inverse_model)),
                ("params", offset_of!(CookedVolume, params)),
            ]),
        ),
        (
            "Bounds",
            pod::<MeshletBounds>(vec![
                ("center", offset_of!(MeshletBounds, center)),
                ("radius", offset_of!(MeshletBounds, radius)),
                ("cone_apex", offset_of!(MeshletBounds, cone_apex)),
                ("cone_cutoff", offset_of!(MeshletBounds, cone_cutoff)),
                ("cone_axis", offset_of!(MeshletBounds, cone_axis)),
                ("reserved", offset_of!(MeshletBounds, reserved)),
            ]),
        ),
    ]);
    let module = naga::front::wgsl::parse_str(&declarations(source))
        .expect("owned shader declarations parse");
    let mut found = BTreeSet::new();
    for (_, global) in module.global_variables.iter() {
        if global.binding.is_some()
            && matches!(
                global.space,
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. }
            )
        {
            discover(&module, global.ty, &mut found);
        }
    }
    for ty in &found {
        let name = module.types[*ty].name.as_deref().expect("record name");
        let (size, fields) = layouts
            .get(name)
            .unwrap_or_else(|| panic!("bound WGSL struct {name} has no Rust Pod record"));
        let TypeInner::Struct { members, span } = &module.types[*ty].inner else {
            unreachable!()
        };
        assert_eq!(*span as usize, *size, "{name} size");
        assert_eq!(members.len(), fields.len(), "{name} fields");
        for (member, (field, offset)) in members.iter().zip(fields) {
            assert_eq!(member.name.as_deref(), Some(*field));
            assert_eq!(member.offset as usize, *offset, "{name}.{field}");
        }
    }
    found.len()
}

#[test]
fn every_owned_bound_record_has_a_pod_layout() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let count: usize = std::fs::read_dir(root)
        .expect("sources")
        .map(|e| e.expect("source").path())
        .filter(|p| p.extension().is_some_and(|e| e == "wgsl"))
        .map(|p| check(&std::fs::read_to_string(p).expect("WGSL")))
        .sum();
    assert_eq!(
        count, 3,
        "Cull, Bounds and CookedVolume; new bindings must have records too"
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
