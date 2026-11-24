//! Reflect every supported view fragment variant so a new shader input cannot
//! silently outgrow the stage counts admitted before pipeline creation.
use super::*;

#[test]
fn secondary_shader_layout_is_bounded_by_the_admission_counts() {
    for atmosphere in [false, true] {
        for shadows in [false, true] {
            for cloud in [false, true] {
                if cloud && !atmosphere {
                    continue;
                }
                let source = shader_source(&crate::synthetic::scene_source::secondary(
                    atmosphere,
                    atmosphere && shadows,
                    cloud,
                    crate::synthetic::scene_source::SunShadows::from(shadows),
                ));
                let module = wgpu::naga::front::wgsl::parse_str(&source).expect("view shader");
                let info = wgpu::naga::valid::Validator::new(
                    wgpu::naga::valid::ValidationFlags::all(),
                    wgpu::naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .expect("valid view shader");
                let entry = module
                    .entry_points
                    .iter()
                    .position(|entry| {
                        entry.name
                            == if shadows {
                                "secondary_shadowed_fragment"
                            } else {
                                "secondary_fragment"
                            }
                    })
                    .expect("view fragment");
                let mut uniforms = 0;
                let mut textures = 0;
                for (handle, global) in module.global_variables.iter() {
                    if global.binding.is_none() || info.get_entry_point(entry)[handle].is_empty() {
                        continue;
                    }
                    uniforms += u32::from(global.space == wgpu::naga::AddressSpace::Uniform);
                    textures += u32::from(matches!(
                        module.types[global.ty].inner,
                        wgpu::naga::TypeInner::Image { .. }
                    ));
                }
                let counts = shader_input_counts(
                    atmosphere,
                    shadows,
                    if cloud {
                        crate::shaders::CloudVariant::Sampled
                    } else {
                        crate::shaders::CloudVariant::Absent
                    },
                );
                assert_eq!(
                    uniforms, counts[0],
                    "atmosphere={atmosphere} shadows={shadows} cloud={cloud}"
                );
                // The material layout also reserves the unused normal texture array.
                assert_eq!(
                    textures + 1,
                    counts[1],
                    "atmosphere={atmosphere} shadows={shadows} cloud={cloud}"
                );
            }
        }
    }
}
