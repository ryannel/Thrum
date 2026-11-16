//! The actual prepared bindings and graph are checked against entry-point reflection.
#[expect(
    dead_code,
    reason = "shared Metal harness also contains full field oracles"
)]
mod support;

#[test]
fn bound_resources_equal_declared_accesses_for_every_pass() {
    use thrum_render::graph::AccessKind;
    use wgpu::naga::{self, valid::GlobalUse};
    let Some(h) = support::Harness::new(false) else {
        return;
    };
    let module = naga::front::wgsl::parse_str(thrum_sea::SHADER).expect("valid shader");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("valid shader");
    let bindings = h.sea.binding_accesses();
    let graph = h.sea.graph();
    for pass in &graph.passes {
        let actual: Vec<_> = bindings.iter().filter(|b| b.pass == pass.name).collect();
        assert_eq!(
            actual.len(),
            pass.accesses.len(),
            "{} binding count",
            pass.name
        );
        let entry = module
            .entry_points
            .iter()
            .position(|p| p.name == actual[0].entry)
            .expect("prepared entry point");
        let uses = info.get_entry_point(entry);
        let mut reflected = 0;
        for (handle, global) in module.global_variables.iter() {
            let Some(slot) = &global.binding else {
                continue;
            };
            let usage = uses[handle];
            if usage.is_empty() {
                continue;
            }
            reflected += 1;
            let binding = actual
                .iter()
                .find(|b| b.group == slot.group && b.binding == slot.binding)
                .expect("every shader resource is bound");
            let expected = if global.space == naga::AddressSpace::Uniform {
                AccessKind::UniformRead
            } else if usage.contains(GlobalUse::WRITE) {
                if usage.contains(GlobalUse::READ) {
                    AccessKind::StorageReadWrite
                } else {
                    AccessKind::StorageWrite
                }
            } else {
                AccessKind::StorageRead
            };
            assert_eq!(
                binding.access, expected,
                "{} slot {}",
                pass.name, slot.binding
            );
            let declaration = pass
                .accesses
                .iter()
                .find(|a| a.resource == binding.resource)
                .expect("bound resource declared");
            assert_eq!(declaration.kind, binding.access);
            let thrum_render::graph::ResourceDescriptor::Buffer { bytes, .. } =
                &graph.resources[binding.resource].0
            else {
                panic!("sea buffer")
            };
            assert_eq!(declaration.range, 0..*bytes);
        }
        assert_eq!(
            reflected,
            actual.len(),
            "{} binds no unused resources",
            pass.name
        );
    }
    h.save("declared-accesses");
}
