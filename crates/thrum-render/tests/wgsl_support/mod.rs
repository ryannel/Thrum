use std::collections::BTreeSet;

use wgpu::naga::{self, Handle, Module, Type, TypeInner};

pub(super) fn parse(name: &str, source: &str) -> Module {
    naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|err| panic!("{name}: {}", err.emit_to_string(source)))
}

pub(super) fn bound_structs(module: &Module) -> BTreeSet<Handle<Type>> {
    let mut found = BTreeSet::new();
    for (_, global) in module.global_variables.iter() {
        if global.binding.is_some()
            && matches!(
                global.space,
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. }
            )
        {
            visit(module, global.ty, &mut found);
        }
    }
    found
}

fn visit(module: &Module, ty: Handle<Type>, found: &mut BTreeSet<Handle<Type>>) {
    match &module.types[ty].inner {
        TypeInner::Struct { members, .. } => {
            if found.insert(ty) {
                for member in members {
                    visit(module, member.ty, found);
                }
            }
        }
        TypeInner::Array { base, .. } | TypeInner::BindingArray { base, .. } => {
            visit(module, *base, found);
        }
        _ => {}
    }
}
