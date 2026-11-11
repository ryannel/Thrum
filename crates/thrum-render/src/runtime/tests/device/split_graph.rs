use super::*;

#[test]
fn metal_both_recipes_prepare_one_graph_and_reuse_scene_textures_on_surface_resize() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (capabilities, queue) = gpu(&instance, 7).await;
        for recipe in ["portable", "apple-tile"] {
            let selected = capabilities.clone().with_recipe_override(recipe).unwrap();
            let mut host = host(&selected, &queue, Policy::default()).await;
            let output_view = host.slots[0].target.output_view().clone();
            let before = host.slots[0].target.texture_allocation();
            for extent in [[64, 64], [97, 51]] {
                host.resize_surface(extent).unwrap();
                let target = &host.slots[0].target;
                for capture in [false, true] {
                    let plan = host.prepared.graph(target, capture).unwrap();
                    let boundary = plan.pre_acquire_passes.expect("late acquisition partition");
                    let names: Vec<_> = plan
                        .pass_bindings()
                        .map(|(_, pass)| pass.name.as_str())
                        .collect();
                    assert_eq!(&names[boundary..], &[crate::present::PASS_NAME]);
                    assert!(names[..boundary].contains(&"synthetic tone encode"));
                    assert!(plan.surface_resource().is_some());
                    assert_eq!(plan.device_generation, selected.generation());
                }
                assert_eq!(target.output_view(), &output_view);
                assert_eq!(
                    target.texture_allocation(),
                    before,
                    "surface resize reuses all scene textures"
                );
            }
        }
    });
}
