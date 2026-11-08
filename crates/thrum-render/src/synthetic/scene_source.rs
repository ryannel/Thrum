//! Composition of the prepared scene's forward shader source, the text a
//! [`crate::contribution::SceneSurface`] lends to contributions.

use super::air_shadow;

/// The forward WGSL for a scene with or without the atmosphere and its aerial shadow.
pub(crate) fn compose(atmosphere: bool, air_shadow: bool, receiver_resolve: bool) -> String {
    format!(
        "{}\n{}\n{}\n{}",
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
        },
        if receiver_resolve {
            concat!(
                include_str!("receiver_forward.wgsl"),
                "\n",
                include_str!("receiver_raster.wgsl")
            )
        } else {
            "fn last_cascade_visibility(input: VertexOutput, receiver: ShadowReceiver) -> f32 {
                return cascade_visibility(1u, receiver);
            }"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::compose;
    use crate::contribution::{HookPointError, material_hook_point};

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
