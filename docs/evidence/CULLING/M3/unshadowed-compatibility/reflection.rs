use wgpu::naga;
fn main() {
    let source = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all()).validate(&module).unwrap();
    for (index, entry) in module.entry_points.iter().enumerate() {
        if entry.name != "fragment" { continue; }
        for (handle, global) in module.global_variables.iter() {
            if let Some(binding) = &global.binding {
                if binding.group == 0 && (binding.binding == 2 || binding.binding == 3) {
                    println!("{} {:?} {:?}", entry.name, binding, info.get_entry_point(index)[handle]);
                }
            }
        }
    }
}
