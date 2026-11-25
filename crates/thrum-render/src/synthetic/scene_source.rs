//! Composition of the prepared scene's forward shader source, the text a
//! [`crate::contribution::SceneSurface`] lends to contributions.

use super::air_shadow;
use crate::capabilities::{ShadowSelection, SunShadowMethod};
use crate::shaders::wgsl;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SunShadows {
    Disabled,
    Enabled,
}

impl From<bool> for SunShadows {
    fn from(enabled: bool) -> Self {
        if enabled {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// Prepare one coherent forward source, shader module and entry selection.
pub(super) fn prepare(
    device: &crate::runtime::DeviceSite,
    atmosphere: crate::shaders::AtmosphereVariant,
    cloud: Option<&crate::atmosphere::SunOpticalDepth>,
    shadowed: SunShadows,
    selection: ShadowSelection,
    (soft_sun, ambient): (bool, bool),
    fail_pipeline: bool,
) -> (String, wgpu::ShaderModule, &'static str) {
    let shader = crate::shaders::Shader::Scene(crate::shaders::SceneVariant {
        atmosphere,
        cloud: crate::shaders::CloudVariant::from_provider(cloud),
        shadows: shadowed,
        sun: selection.sun,
        traced_lamps: selection.lamps,
        soft_sun,
        ambient,
    });
    let (source, module) =
        shader.compile_with_source(device, "prepared synthetic shader ABI2", fail_pipeline);
    let entry = match shadowed {
        SunShadows::Enabled => "shadowed_fragment",
        SunShadows::Disabled => "fragment",
    };
    (source, module, entry)
}

fn traced(
    mut source: String,
    shadowed: SunShadows,
    selection: ShadowSelection,
    soft_sun: bool,
) -> String {
    let sun = shadowed == SunShadows::Enabled && selection.sun == SunShadowMethod::Traced;
    if !sun && !selection.lamps {
        return source;
    }
    if sun {
        let start = source.find("fn sun_visibility(").expect("Sun operation");
        let end = start
            + source[start..]
                .find("// Within a few degrees")
                .expect("end of Sun operation");
        let hard = wgsl!("synthetic/raytrace/sun.wgsl");
        if soft_sun {
            source.replace_range(
                start..end,
                &format!(
                    "{hard}\n{}\n{}",
                    wgsl!("synthetic/raytrace/soft_common.wgsl"),
                    wgsl!("synthetic/raytrace/soft_forward.wgsl")
                ),
            );
        } else {
            source.replace_range(start..end, &format!("{hard}\nfn sun_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {{ return hard_sun_visibility(input, receiver); }}\n"));
        }
        source = source.replace(
            "dpdx(input.world_position),\n        dpdy(input.world_position),normalize(input.normal)",
            "dpdx(input.shadow_surface.xyz),\n        dpdy(input.shadow_surface.xyz),normalize(input.normal)",
        );
        source = source.replace(
            "@location(6) current_clip: vec4<f32>,",
            "@location(6) current_clip: vec4<f32>,\n    @location(7) shadow_surface: vec4<f32>,",
        );
        source = source.replace(
            "output.uv = vertex.b.zw;",
            "output.shadow_surface = shadow_tangent_surface(model, local, output.normal);\n    output.uv = vertex.b.zw;",
        );
    }
    if selection.lamps {
        source = source
            .replace("    var sum = vec3(0.0);\n    for", "    var sum = vec3(0.0);\n    var lamp_rays = 0u;\n    for")
            .replace(
                "        let hv = l+v;",
                "        if dot(n,l) > 0.0 && distance_squared > 1e-12 {
            if lamp_rays < lamp_shadow_quality.ray_limit {
                lamp_rays += 1u;
                if lamp_blocked(world_position, geometric_normal, source.position_radius.xyz) { continue; }
            } else {
                atomicAdd(&unshadowed_lamp_references, 1u);
            }
        }
        let hv = l+v;",
            )
            .replace(
                "n: vec3<f32>, v: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>)",
                "n: vec3<f32>, v: vec3<f32>, world_position: vec3<f32>, screen_position: vec4<f32>, geometric_normal: vec3<f32>)",
            )
            .replace(
                "n,v,input.world_position,input.position)",
                "n,v,input.world_position,input.position,normalize(input.normal))",
            );
        source.push_str(wgsl!("synthetic/raytrace/lamps.wgsl"));
    }
    format!(
        "enable wgpu_ray_query;\n{source}\n{}",
        wgsl!("synthetic/raytrace/shadows.wgsl"),
    )
}

/// Both main and secondary views consume the provider's selected mapping layout.
pub(super) fn cloud_variant(source: String, variant: crate::shaders::CloudVariant) -> String {
    if variant.gpu() || variant.ground() {
        source.replace(
            crate::atmosphere::CLOUD_SHADER_SOURCE,
            &variant.sampling_source(),
        )
    } else {
        source
    }
}

fn apply_providers(source: String, cloud: bool, shadowed: SunShadows) -> String {
    // Resource reflection precedes backend constant folding. A false argument
    // to shade() must not make an unshadowed entry require absent Sun bindings.
    let source = if shadowed == SunShadows::Enabled {
        source
    } else {
        source.replace("sun_factor = sun_visibility(input,receiver);", "")
    };
    if !cloud {
        return source;
    }
    let shaded = source.replace(
        "sun_irradiance(input.world_position)*sun_factor",
        "sun_irradiance(input.world_position)*sun_factor*cloud_factor(input.world_position)",
    );
    format!("{shaded}\n{}", crate::atmosphere::CLOUD_SHADER_SOURCE)
}

/// The forward WGSL for a scene with or without the atmosphere and its aerial shadow.
pub(super) fn compose(atmosphere: bool, air_shadow: bool, ambient: bool) -> String {
    compose_aerial(
        atmosphere,
        air_shadow::aerial_source(atmosphere, air_shadow),
        ambient,
    )
}

fn compose_aerial(atmosphere: bool, aerial: &str, ambient: bool) -> String {
    let surface = if ambient {
        wgsl!(@ambient_opaque_forward)
    } else {
        wgsl!("synthetic.wgsl")
    };
    format!(
        "{}{}\n{}\n{}",
        concat!(
            wgsl!("atmosphere/storage.wgsl"),
            "\n",
            wgsl!("atmosphere/brdf.wgsl"),
            "\n",
            wgsl!("atmosphere/irradiance.wgsl"),
            "\n",
            wgsl!("directional_light.wgsl"),
            "\n",
            wgsl!("environment_light.wgsl"),
            "\n"
        ),
        surface,
        aerial,
        if atmosphere {
            concat!(
                wgsl!("atmosphere/geometry.wgsl"),
                "\n",
                wgsl!("synthetic_sun.wgsl"),
                "\n",
                wgsl!("synthetic_environment.wgsl")
            )
        } else {
            ""
        }
    )
}

/// A view changes only the decoder's ray origin/distance. The main source is
/// assembled as before, and no view declarations add text to that source.
pub(super) fn secondary(
    atmosphere: bool,
    shadowed_air: bool,
    cloud: bool,
    shadows: SunShadows,
) -> String {
    let aerial = match (atmosphere, shadowed_air) {
        (true, true) => concat!(
            wgsl!("synthetic_aerial.wgsl"),
            "\n",
            wgsl!("synthetic_aerial_visibility.wgsl"),
            "\n",
            wgsl!("synthetic/secondary/aerial_ray.wgsl"),
            "\n",
            wgsl!("synthetic/secondary/aerial_shadowed.wgsl")
        ),
        (true, false) => concat!(
            wgsl!("synthetic_aerial.wgsl"),
            "\n",
            wgsl!("synthetic/secondary/aerial_ray.wgsl"),
            "\n",
            wgsl!("synthetic/secondary/aerial_clear.wgsl")
        ),
        _ => wgsl!("synthetic_plain.wgsl"),
    };
    let source = apply_providers(compose_aerial(atmosphere, aerial, false), cloud, shadows);
    if shadows == SunShadows::Enabled {
        // Cascades are view-local; only the distant top-down height layer is
        // sampled from the scene's array. Keep the default source untouched.
        format!(
            "{}\n@group(0) @binding(7) var scene_height_maps: texture_depth_2d_array;",
            source.replace(
                "textureLoad(shadow_maps,texel_index,",
                "textureLoad(scene_height_maps,texel_index,"
            )
        )
    } else {
        source
    }
}

pub(crate) fn main_source(variant: crate::shaders::SceneVariant) -> String {
    let source = if matches!(
        variant.atmosphere,
        crate::shaders::AtmosphereVariant::ViewVolume
    ) {
        let aerial = format!(
            "{}{}",
            crate::atmosphere::gpu::view_volume::aerial_source(),
            wgsl!("synthetic_aerial_clear.wgsl"),
        );
        apply_providers(
            compose_aerial(true, &aerial, variant.ambient),
            variant.cloud.present(),
            variant.shadows,
        )
    } else {
        apply_providers(
            compose(
                variant.atmosphere.present(),
                variant.atmosphere.air_shadow(),
                variant.ambient,
            ),
            variant.cloud.present(),
            variant.shadows,
        )
    };
    let source = if variant.ambient {
        let closure = if variant.atmosphere.present() {
            wgsl!(@ambient_environment)
        } else {
            wgsl!(@bytes "synthetic/raytrace/ambient_plain.wgsl")
        };
        format!(
            "{source}\n{closure}{}{}{}",
            wgsl!("synthetic/raytrace/ambient_common.wgsl"),
            wgsl!("synthetic/raytrace/ambient_forward.wgsl"),
            wgsl!("synthetic/raytrace/ambient_receiver.wgsl"),
        )
    } else {
        source
    };
    let source = cloud_variant(source, variant.cloud);
    traced(
        source,
        variant.shadows,
        ShadowSelection {
            sun: variant.sun,
            lamps: variant.traced_lamps,
            ..ShadowSelection::default()
        },
        variant.soft_sun,
    )
}

#[cfg(test)]
mod tests {
    use super::{SunShadows, apply_providers, compose};
    use crate::contribution::{HookPointError, material_hook_point};

    #[test]
    fn traced_forward_sources_validate() {
        use crate::capabilities::{RayStructureSelection, ShadowSelection, SunShadowMethod};
        use wgpu::naga;

        for shadows in [false, true] {
            for (sun, lamps) in [
                (SunShadowMethod::Traced, false),
                (SunShadowMethod::Traced, true),
                (SunShadowMethod::Maps, true),
            ] {
                let shadows = SunShadows::from(shadows);
                let selection =
                    ShadowSelection::resolve(sun, lamps, RayStructureSelection::Enabled);
                let source = super::traced(
                    apply_providers(compose(true, true, false), false, shadows),
                    shadows,
                    selection,
                    sun == SunShadowMethod::Traced,
                );
                let module = naga::front::wgsl::parse_str(&source)
                    .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
            }
        }
    }

    #[test]
    fn unshadowed_entry_requires_no_sun_bindings() {
        use wgpu::naga;
        for atmosphere in [false, true] {
            for shadowed in [false, true] {
                let source = apply_providers(
                    compose(atmosphere, atmosphere && shadowed, false),
                    false,
                    SunShadows::from(shadowed),
                );
                let module = naga::front::wgsl::parse_str(&source).expect("WGSL");
                let info = naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .expect("valid shader");
                let entry = if shadowed {
                    "shadowed_fragment"
                } else {
                    "fragment"
                };
                let entry = module
                    .entry_points
                    .iter()
                    .position(|point| point.name == entry)
                    .expect("entry");
                let mut checked = 0;
                for (handle, global) in module.global_variables.iter() {
                    if global.binding.as_ref().is_some_and(|binding| {
                        binding.group == 0 && matches!(binding.binding, 2 | 3)
                    }) {
                        assert_eq!(!info.get_entry_point(entry)[handle].is_empty(), shadowed);
                        checked += 1;
                    }
                }
                assert_eq!(checked, 2, "positive control includes both Sun resources");
            }
        }
    }

    /// The body of `struct name { .. }` in `source`.
    fn struct_body<'a>(source: &'a str, name: &str) -> &'a str {
        let open = format!("struct {name} {{");
        let start = source.find(&open).expect("struct declared") + open.len();
        let end = start + source[start..].find('}').expect("struct closed");
        &source[start..end]
    }

    /// The byte range inside the braces of the one function whose declaration starts with
    /// `signature`.
    fn function_body(source: &str, signature: &str) -> std::ops::Range<usize> {
        assert_eq!(source.matches(signature).count(), 1, "one `{signature}`");
        let declared = source.find(signature).expect("declared");
        let open = declared + source[declared..].find('{').expect("body opens");
        let mut depth = 0_usize;
        for (offset, byte) in source.bytes().enumerate().skip(open) {
            match byte {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return open + 1..offset;
                    }
                }
                _ => {}
            }
        }
        panic!("`{signature}` body never closes");
    }

    #[test]
    fn every_forward_variant_holds_the_material_hook_point_once_in_shade() {
        for atmosphere in [false, true] {
            for air_shadow in [false, true] {
                let source = compose(atmosphere, air_shadow, false);
                let variant = format!("atmosphere {atmosphere}, air shadow {air_shadow}");
                let point = material_hook_point(&source)
                    .unwrap_or_else(|error| panic!("{variant}: {error}"));
                let shade = function_body(&source, "fn shade(input: VertexOutput,");
                let first_sample = shade.start
                    + source[shade.clone()]
                        .find("textureSample")
                        .expect("shade samples textures");
                assert!(
                    shade.start <= point.start && point.end <= first_sample,
                    "{variant}: the hook point is not in shade()'s body before its texture samples"
                );
                let record = struct_body(&source, "MaterialRecord");
                for field in [
                    "base: vec4<f32>",
                    "emissive_cutoff: vec4<f32>",
                    "surface: vec4<f32>",
                ] {
                    assert!(
                        record.contains(field),
                        "{variant}: MaterialRecord lost {field}"
                    );
                }
                let output = struct_body(&source, "VertexOutput");
                for field in [
                    "uv: vec2<f32>",
                    "normal: vec3<f32>",
                    "world_position: vec3<f32>",
                ] {
                    assert!(
                        output.contains(field),
                        "{variant}: VertexOutput lost {field}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_missing_or_repeated_hook_point_is_refused_by_name() {
        let read = "materials[input.material]";
        assert_eq!(
            material_hook_point(&format!("a {read} b")),
            Ok(2..2 + read.len())
        );
        assert_eq!(material_hook_point("a b"), Err(HookPointError::Missing));
        assert_eq!(
            material_hook_point(&format!("{read}{read}{read}")),
            Err(HookPointError::Duplicate(3))
        );
        assert_eq!(
            HookPointError::Missing.to_string(),
            "the scene shader has no `scene material read` hook point"
        );
        assert_eq!(
            HookPointError::Duplicate(2).to_string(),
            "the scene shader has 2 `scene material read` hook points where exactly one is required"
        );
    }
}
