//! Composition of the prepared scene's forward shader source, the text a
//! [`crate::contribution::SceneSurface`] lends to contributions.

use super::air_shadow;

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
    use super::compose;
    use crate::contribution::{HookPointError, MATERIAL_HOOK_POINT, ShaderHookPoint};

    /// The body of `struct name { .. }` in `source`.
    fn struct_body<'a>(source: &'a str, name: &str) -> &'a str {
        let open = format!("struct {name} {{");
        let start = source.find(&open).expect("struct declared") + open.len();
        let end = start + source[start..].find('}').expect("struct closed");
        &source[start..end]
    }

    #[test]
    fn every_forward_variant_holds_the_material_hook_point_once_in_shade() {
        for atmosphere in [false, true] {
            for air_shadow in [false, true] {
                let source = compose(atmosphere, air_shadow);
                let variant = format!("atmosphere {atmosphere}, air shadow {air_shadow}");
                let at = MATERIAL_HOOK_POINT
                    .locate(&source)
                    .unwrap_or_else(|error| panic!("{variant}: {error}"));
                let shade = source
                    .find("fn shade(input: VertexOutput,")
                    .expect("the forward shader shades in shade()");
                let first_sample = shade
                    + source[shade..]
                        .find("textureSample")
                        .expect("shade samples textures");
                assert!(
                    shade < at && at < first_sample,
                    "{variant}: the hook point leaves shade() before its texture samples"
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
        let point = ShaderHookPoint {
            name: "probe",
            expression: "read(x)",
        };
        assert_eq!(point.locate("a read(x) b"), Ok(2));
        assert_eq!(point.locate("a b"), Err(HookPointError::Missing("probe")));
        assert_eq!(
            point.locate("read(x) read(x) read(x)"),
            Err(HookPointError::Duplicate("probe", 3))
        );
        assert_eq!(
            HookPointError::Missing("probe").to_string(),
            "the scene shader has no `probe` hook point"
        );
    }
}
