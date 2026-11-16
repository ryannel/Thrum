//! Every bound struct is discovered from the actual shader, not from a list of shader names.
use std::collections::BTreeSet;
use wgpu::naga::{self, TypeInner};

fn bound_structs(
    module: &naga::Module,
    ty: naga::Handle<naga::Type>,
    found: &mut BTreeSet<naga::Handle<naga::Type>>,
) {
    match &module.types[ty].inner {
        TypeInner::Struct { members, .. } => {
            if found.insert(ty) {
                for member in members {
                    bound_structs(module, member.ty, found);
                }
            }
        }
        TypeInner::Array { base, .. } | TypeInner::BindingArray { base, .. } => {
            bound_structs(module, *base, found);
        }
        _ => {}
    }
}

fn check(source: &str) {
    let module = naga::front::wgsl::parse_str(source).expect("sea WGSL parses");
    let layouts = thrum_sea::records::layouts();
    let mut found = BTreeSet::new();
    for (_, global) in module.global_variables.iter() {
        if global.binding.is_some()
            && matches!(
                global.space,
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. }
            )
        {
            bound_structs(&module, global.ty, &mut found);
        }
    }
    assert!(!found.is_empty());
    for ty in found {
        let name = module.types[ty]
            .name
            .as_deref()
            .expect("bound structs are named");
        let rust = layouts
            .iter()
            .find(|l| l.name == name)
            .unwrap_or_else(|| panic!("bound WGSL struct {name} has no Rust Pod record"));
        let TypeInner::Struct { members, span } = &module.types[ty].inner else {
            unreachable!()
        };
        assert_eq!(*span as usize, rust.size, "{name} size");
        assert_eq!(members.len(), rust.members.len(), "{name} member count");
        for (member, &(name, offset)) in members.iter().zip(&rust.members) {
            assert_eq!(member.name.as_deref(), Some(name));
            assert_eq!(member.offset as usize, offset, "{name} offset");
        }
    }
}

#[test]
fn every_bound_struct_has_equal_host_layout() {
    check(thrum_sea::SHADER);
    check(thrum_sea::FILTERED_SHADER);
}

#[test]
#[should_panic(expected = "has no Rust Pod record")]
fn a_new_bound_record_cannot_escape_reflection() {
    check(&format!(
        "{}\nstruct Unregistered {{ value: vec4<f32> }}\n@group(2) @binding(0) var<uniform> new_record: Unregistered;",
        thrum_sea::SHADER
    ));
}

#[test]
#[should_panic(expected = "Params size")]
fn a_changed_bound_record_cannot_escape_reflection() {
    check(&thrum_sea::SHADER.replace(
        "lengths: vec4<f32>,",
        "lengths: vec4<f32>, extra: vec4<f32>,",
    ));
}
