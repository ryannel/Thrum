//! Composition of the prepared scene's forward shader source, the text a
//! [`crate::contribution::SceneSurface`] lends to contributions.

use super::air_shadow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SunShadows {
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
    atmosphere: bool,
    air_shadow: bool,
    cloud: bool,
    shadowed: SunShadows,
) -> (String, wgpu::ShaderModule, &'static str) {
    let source = compose_with_cloud(atmosphere, air_shadow, cloud, shadowed);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("prepared synthetic shader ABI2"),
        source: wgpu::ShaderSource::Wgsl(source.as_str().into()),
    });
    let entry = if shadowed == SunShadows::Enabled {
        "shadowed_fragment"
    } else {
        "fragment"
    };
    (source, module, entry)
}

fn compose_with_cloud(
    atmosphere: bool,
    air_shadow: bool,
    cloud: bool,
    shadowed: SunShadows,
) -> String {
    let source = compose(atmosphere, air_shadow);
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
    format!("{shaded}\n{}", include_str!("../atmosphere/cloud.wgsl"))
}

/// The forward WGSL for a scene with or without the atmosphere and its aerial shadow.
pub(super) fn compose(atmosphere: bool, air_shadow: bool) -> String {
    format!(
        "{}\n{}\n{}",
        concat!(
            include_str!("../atmosphere/storage.wgsl"),
            "\n",
            include_str!("../atmosphere/brdf.wgsl"),
            "\n",
            include_str!("../atmosphere/irradiance.wgsl"),
            "\n",
            include_str!("../synthetic.wgsl")
        ),
        air_shadow::aerial_source(atmosphere, air_shadow),
        if atmosphere {
            concat!(
                include_str!("../atmosphere/geometry.wgsl"),
                "\n",
                include_str!("../synthetic_sun.wgsl"),
                "\n",
                include_str!("../synthetic_environment.wgsl")
            )
        } else {
            ""
        }
    )
}

#[cfg(test)]
mod tests {
    use super::{SunShadows, compose, compose_with_cloud};
    use crate::contribution::{HookPointError, material_hook_point};

    #[test]
    fn unshadowed_entry_requires_no_sun_bindings() {
        use wgpu::naga;
        for atmosphere in [false, true] {
            for shadowed in [false, true] {
                let source = compose_with_cloud(
                    atmosphere,
                    atmosphere && shadowed,
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
                let source = compose(atmosphere, air_shadow);
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
