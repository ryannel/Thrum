//! Production WGSL recipes. The catalog also enumerates the sources reflected by the ABI checks.
use std::borrow::Cow;

use crate::{capabilities::SunShadowMethod, synthetic::scene_source::SunShadows};

pub(crate) const CLUSTER_PROBE_WORKGROUP: u32 = 64;

/// A source marker gives reflected declarations a file identity across compositions.
macro_rules! wgsl {
    (@lighting_constants) => {
        include_str!(concat!(env!("OUT_DIR"), "/lighting_constants.wgsl"))
    };
    ("present.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/srgb.wgsl")),
            $crate::shaders::wgsl!(@file "present.wgsl"),
        )
    };
    ("synthetic/display/srgb.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/srgb.wgsl")),
            $crate::shaders::wgsl!(@file "synthetic/display/srgb.wgsl"),
        )
    };
    ("atmosphere/common.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/atmosphere_luts.wgsl")),
            $crate::shaders::wgsl!(@file "records/observer.wgsl"),
            $crate::shaders::wgsl!(@file "atmosphere/common.wgsl"),
        )
    };
    ("atmosphere/geometry.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/atmosphere_geometry.wgsl")),
            $crate::shaders::wgsl!(@file "atmosphere/geometry.wgsl"),
        )
    };
    ("atmosphere/sky.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/sun.wgsl")),
            "\n// thrum-wgsl-source: atmosphere/sky.wgsl\n",
            $crate::shaders::wgsl!(@bytes "atmosphere/sky.wgsl"),
            $crate::shaders::wgsl!(@bytes "atmosphere/sky_draw.wgsl"),
            "\n// thrum-wgsl-end\n",
        )
    };
    ("atmosphere/volume_sky.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/sun.wgsl")),
            $crate::shaders::wgsl!(@file "atmosphere/sky.wgsl"),
            $crate::shaders::wgsl!(@file "atmosphere/volume_sky.wgsl"),
        )
    };
    ("atmosphere/transport.wgsl") => {
        concat!(
            "\n// thrum-wgsl-source: atmosphere/transport.wgsl\n",
            $crate::shaders::wgsl!(@transport "atmosphere/transport_extinction.wgsl", "atmosphere/transport_scattering.wgsl"),
            "\n// thrum-wgsl-end\n",
        )
    };
    (@transport $extinction:literal, $scattering:literal) => {
        concat!(
            $crate::shaders::wgsl!(@transport_core $extinction, $scattering),
            $crate::shaders::wgsl!(@bytes "atmosphere/transport_lit.wgsl"),
        )
    };
    (@transport_core $extinction:literal, $scattering:literal) => {
        concat!(
            $crate::shaders::wgsl!(@bytes "atmosphere/transport.wgsl"),
            $crate::shaders::wgsl!(@bytes $extinction),
            $crate::shaders::wgsl!(@bytes "atmosphere/transport_segments.wgsl"),
            $crate::shaders::wgsl!(@bytes $scattering),
        )
    };
    ("atmosphere/lut.wgsl") => {
        concat!(
            "\n// thrum-wgsl-source: atmosphere/lut.wgsl\n",
            $crate::shaders::wgsl!(@lut "atmosphere/lut_multiple.wgsl", "atmosphere/lut_sky_view.wgsl"),
            "\n// thrum-wgsl-end\n",
        )
    };
    (@lut $multiple:literal, $sky_view:literal) => {
        concat!(
            $crate::shaders::wgsl!(@bytes "atmosphere/lut.wgsl"),
            $crate::shaders::wgsl!(@bytes $multiple),
            $crate::shaders::wgsl!(@bytes "atmosphere/lut_sky_transport.wgsl"),
            $crate::shaders::wgsl!(@bytes $sky_view),
        )
    };
    ("atmosphere/aerial.wgsl") => {
        concat!(
            "\n// thrum-wgsl-source: atmosphere/aerial.wgsl\n",
            $crate::shaders::wgsl!(@bytes "atmosphere/aerial.wgsl"),
            $crate::shaders::wgsl!(@bytes "atmosphere/aerial_main.wgsl"),
            "\n// thrum-wgsl-end\n",
        )
    };
    ("atmosphere/storage.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/atmosphere_storage.wgsl")),
            $crate::shaders::wgsl!(@file "atmosphere/storage.wgsl"),
        )
    };
    ("synthetic.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/cluster.wgsl")),
            include_str!(concat!(env!("OUT_DIR"), "/scene_shadows.wgsl")),
            $crate::shaders::wgsl!(@file "records/view.wgsl"),
            $crate::shaders::wgsl!(@file "records/view_aux.wgsl"),
            $crate::shaders::wgsl!(@file "records/instance.wgsl"),
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "records/mesh_vertex.wgsl"),
            $crate::shaders::wgsl!(@file "records/history.wgsl"),
            $crate::shaders::wgsl!(@file "records/shadow_cascades.wgsl"),
            $crate::shaders::wgsl!(@file "records/cluster_head.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic.wgsl"),
        )
    };
    ("synthetic/air_shadow.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/air_shadow.wgsl")),
            $crate::shaders::wgsl!(@file "records/shadow_cascades.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/air_shadow.wgsl"),
        )
    };
    ("synthetic/cluster.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/cluster.wgsl")),
            $crate::shaders::wgsl!(@file "records/cluster_head.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/cluster.wgsl"),
        )
    };
    ("synthetic/exposure.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/exposure_modes.wgsl")),
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/exposure.wgsl"),
        )
    };
    ("synthetic/history_gpu.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/history.wgsl")),
            $crate::shaders::wgsl!(@file "records/instance.wgsl"),
            $crate::shaders::wgsl!(@file "records/history.wgsl"),
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/history_gpu.wgsl"),
        )
    };
    ("synthetic/raytrace/view.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "synthetic/raytrace/view.wgsl"),
            $crate::shaders::wgsl!(@file "records/view_aux.wgsl"),
        )
    };
    ("synthetic/reference.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/reconstruction.wgsl")),
            $crate::shaders::wgsl!(@file "records/reconstruction.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/reference.wgsl"),
        )
    };
    ("synthetic/shadow.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/shadow_view.wgsl"),
            $crate::shaders::wgsl!(@file "records/instance.wgsl"),
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "records/mesh_vertex.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/shadow.wgsl"),
        )
    };
    ("synthetic/stars.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            $crate::shaders::wgsl!(@file "records/observer.wgsl"),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/stars.wgsl"),
        )
    };
    ("synthetic/taau.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/reconstruction.wgsl")),
            $crate::shaders::wgsl!(@file "records/reconstruction.wgsl"),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/taau.wgsl"),
        )
    };
    ("synthetic/temporal.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/temporal.wgsl"),
        )
    };
    ("synthetic_aerial.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/observer.wgsl"),
            "\n// thrum-wgsl-source: synthetic_aerial.wgsl\n",
            $crate::shaders::wgsl!(@bytes "synthetic_aerial.wgsl"),
            $crate::shaders::wgsl!(@bytes "synthetic_aerial_sample.wgsl"),
            $crate::shaders::wgsl!(@bytes "synthetic_aerial_decode.wgsl"),
            "\n// thrum-wgsl-end\n",
        )
    };
    ("synthetic_display.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@lighting_constants),
            include_str!(concat!(env!("OUT_DIR"), "/display_selectors.wgsl")),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic_display.wgsl"),
        )
    };
    ("visibility.wgsl") => {
        concat!(
            include_str!(concat!(env!("OUT_DIR"), "/visibility.wgsl")),
            $crate::shaders::wgsl!(@file "records/view.wgsl"),
            $crate::shaders::wgsl!(@file "records/instance.wgsl"),
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "visibility.wgsl"),
        )
    };
    ("visibility/casters.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/shadow_view.wgsl"),
            $crate::shaders::wgsl!(@file "visibility/casters.wgsl"),
        )
    };
    ($path:literal) => {
        $crate::shaders::wgsl!(@file $path)
    };
    (@file $path:literal) => {
        concat!("\n// thrum-wgsl-source: ", $path, "\n", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/", $path)), "\n// thrum-wgsl-end\n")
    };
    (@bytes $path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/", $path))
    };
}
pub(crate) use wgsl;

#[derive(Clone, Copy)]
pub(crate) enum CloudVariant {
    Absent,
    Sampled,
    Gpu,
}

impl CloudVariant {
    pub(crate) fn from_provider(provider: Option<&crate::atmosphere::SunOpticalDepth>) -> Self {
        match provider {
            Some(field) if field.gpu_written() => Self::Gpu,
            Some(_) => Self::Sampled,
            None => Self::Absent,
        }
    }

    pub(crate) fn present(self) -> bool {
        !matches!(self, Self::Absent)
    }

    pub(crate) fn gpu(self) -> bool {
        matches!(self, Self::Gpu)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum AtmosphereVariant {
    Absent,
    Clear,
    Shadowed,
    ViewVolume,
}

impl AtmosphereVariant {
    pub(crate) fn new(present: bool, air_shadow: bool) -> Self {
        match (present, air_shadow) {
            (false, _) => Self::Absent,
            (true, false) => Self::Clear,
            (true, true) => Self::Shadowed,
        }
    }

    pub(crate) fn present(self) -> bool {
        !matches!(self, Self::Absent)
    }

    pub(crate) fn air_shadow(self) -> bool {
        matches!(self, Self::Shadowed)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SceneVariant {
    pub(crate) atmosphere: AtmosphereVariant,
    pub(crate) cloud: CloudVariant,
    pub(crate) shadows: SunShadows,
    pub(crate) sun: SunShadowMethod,
    pub(crate) traced_lamps: bool,
}

macro_rules! shader_catalog {
    ($($name:ident => $source:expr;)*) => {
        #[derive(Clone, Copy)]
        pub(crate) enum Shader {
            $($name,)*
            Scene(SceneVariant),
            SecondaryScene(SceneVariant),
            ViewVolume(bool, CloudVariant),
        }

        impl Shader {
            pub(crate) fn source(self) -> Cow<'static, str> {
                match self {
                    $(Self::$name => ($source).into(),)*
                    Self::Scene(variant) => crate::synthetic::scene_source::main_source(variant).into(),
                    Self::ViewVolume(cascades, cloud) => crate::atmosphere::gpu::view_volume::source(cascades, cloud).into(),
                    Self::SecondaryScene(variant) => crate::synthetic::secondary::composed_source(variant).into(),
                }
            }
        }

        fn fixed_sources() -> impl Iterator<Item = (&'static str, Cow<'static, str>)> {
            [$(Shader::$name,)*].into_iter().zip([$(stringify!($name),)*])
                .map(|(shader, name)| (name, shader.source()))
        }
    };
}

shader_catalog! {
    Present => wgsl!("present.wgsl");
    Sprite => format!(
        "const SPRITE_MAX_MIP: f32 = {}.0;\nconst SPRITE_GUTTER: f32 = {}.0;\nconst SPRITE_PIXEL: u32 = {}u;\nconst SPRITE_NEAREST: u32 = {}u;\nconst SPRITE_BILINEAR: u32 = {}u;\n{}",
        crate::sprite::SHEET_MAX_MIP,
        crate::sprite::SHEET_GUTTER,
        crate::sprite::SpriteSampling::Pixel as u32,
        crate::sprite::SpriteSampling::NearestControl as u32,
        crate::sprite::SpriteSampling::BilinearControl as u32,
        wgsl!("sprite/pass.wgsl")
    );
    Exposure => crate::synthetic::exposure::SHADER;
    Temporal => wgsl!("synthetic/temporal.wgsl");
    History => wgsl!("synthetic/history_gpu.wgsl");
    NativeWitness => wgsl!("synthetic/native_witness.wgsl");
    Shadow => wgsl!("synthetic/shadow.wgsl");
    ClusterProbe => format!(
        "const CLUSTER_PROBE_WORKGROUP: u32 = {}u;\n{}",
        CLUSTER_PROBE_WORKGROUP,
        wgsl!("synthetic/cluster_probe.wgsl")
    );
    Cluster => wgsl!("synthetic/cluster.wgsl");
    RayBuild => wgsl!("synthetic/raytrace/build_probe.wgsl");
    RayView => wgsl!("synthetic/raytrace/view.wgsl");
    CopyMarker => wgsl!("synthetic/readback/copy_marker.wgsl");
    Visibility => visibility_source();
    Pyramid => concat!(
        include_str!(concat!(env!("OUT_DIR"), "/pyramid.wgsl")),
        wgsl!("visibility/pyramid.wgsl")
    );
    Casters => casters_source();
    SecondaryVisibility => secondary_visibility_source();
    AirShadow => concat!(
                    wgsl!("directional_light.wgsl"),
                    "\n",
                    wgsl!("synthetic/air_shadow.wgsl")
                );
    Reconstruction => wgsl!("synthetic/taau.wgsl");
    Reference => wgsl!("synthetic/reference.wgsl");
    DisplayReferred => crate::synthetic::display::DISPLAY_REFERRED_SHADER;
    Display => crate::synthetic::display::SHADER;
    DisplayPattern => wgsl!("synthetic/display/capture_pattern.wgsl");
    Stars => crate::synthetic::stars::shader_source();
    VolumeStars => format!(
        "{}\n{}",
        crate::synthetic::stars::shader_source(),
        wgsl!("synthetic/stars_volume.wgsl")
    );
    EnvironmentProducts => crate::atmosphere::products::library(false);
    EnvironmentProductsFar => crate::atmosphere::products::library(true);
    AtmosphereLuts => crate::atmosphere::gpu::lut_source();
    AtmosphereAerosolLuts => crate::atmosphere::gpu::aerosol::lut_source();
    AtmosphereAerosolAerial => crate::atmosphere::gpu::aerosol::aerial_chart_source();
    AtmosphereVolumeSky => concat!(
        wgsl!("atmosphere/storage.wgsl"),
        wgsl!("atmosphere/geometry.wgsl"),
        wgsl!("atmosphere/common.wgsl"),
        wgsl!("atmosphere/volume_sky.wgsl")
    );
    AtmosphereDiffuse => concat!(
                concat!(
                    wgsl!("atmosphere/storage.wgsl"),
                    "\n",
                    concat!(
                        wgsl!("atmosphere/geometry.wgsl"),
                        "\n",
                        wgsl!("atmosphere/common.wgsl")
                    )
                ),
                "\n",
                wgsl!("atmosphere/irradiance.wgsl"),
                "\n",
                wgsl!("environment_light.wgsl"),
                "\n",
                wgsl!("atmosphere/diffuse.wgsl")
            );
    AtmosphereAerial => crate::atmosphere::spectrum::EARTH
                .shader(concat!(
                    wgsl!("atmosphere/storage.wgsl"),
                    "\n",
                    wgsl!("atmosphere/geometry.wgsl"),
                    "\n",
                    wgsl!("atmosphere/common.wgsl"),
                    "\n",
                    wgsl!("atmosphere/transport.wgsl"),
                    "\n",
                    wgsl!("atmosphere/night.wgsl"),
                    "\n",
                    wgsl!("atmosphere/aerial.wgsl")
                ));
    AtmosphereSky => concat!(
                concat!(
                    wgsl!("atmosphere/storage.wgsl"),
                    "\n",
                    concat!(
                        wgsl!("atmosphere/geometry.wgsl"),
                        "\n",
                        wgsl!("atmosphere/common.wgsl")
                    )
                ),
                "\n",
                wgsl!("atmosphere/sky.wgsl")
            );
    AtmosphereReflection => crate::atmosphere::reflection::cube_source();
    AtmosphereDfg => crate::atmosphere::reflection::dfg_source();
}

impl Shader {
    pub(crate) fn compile(self, device: &wgpu::Device, label: &str) -> wgpu::ShaderModule {
        compile(device, label, self.source())
    }

    pub(crate) fn compile_with_source(
        self,
        device: &wgpu::Device,
        label: &str,
        inject_failure: bool,
    ) -> (String, wgpu::ShaderModule) {
        let source = self.source().into_owned();
        let module = compile(
            device,
            label,
            if inject_failure {
                "intentionally invalid WGSL".into()
            } else {
                source.as_str().into()
            },
        );
        (source, module)
    }
}

fn compile(device: &wgpu::Device, label: &str, source: Cow<'_, str>) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source),
    })
}

fn visibility_source() -> &'static str {
    concat!(wgsl!("visibility.wgsl"), wgsl!("visibility/occlusion.wgsl"))
}

fn casters_source() -> String {
    format!(
        "{}{}",
        visibility_source(),
        wgsl!("visibility/casters.wgsl")
    )
}

fn secondary_visibility_source() -> String {
    let base = visibility_source().replace(
        "let bounds = world_bounds(slot, mesh);\n    let center = bounds.center;",
        "let bounds = world_bounds(slot, mesh);\n    if !secondary_in_view(bounds) { return false; }\n    let center = bounds.center;",
    );
    format!(
        "{base}\n{}\n{}",
        crate::contribution::SecondaryView::record_wgsl(),
        wgsl!("visibility/secondary.wgsl")
    )
}

/// Composed renderer WGSL for ABI reflection tooling.
#[doc(hidden)]
pub fn composed_sources() -> Vec<(&'static str, Cow<'static, str>)> {
    let mut sources: Vec<_> = fixed_sources().collect();
    for cascades in [false, true] {
        for cloud in [
            CloudVariant::Absent,
            CloudVariant::Sampled,
            CloudVariant::Gpu,
        ] {
            sources.push(("ViewVolume", Shader::ViewVolume(cascades, cloud).source()));
        }
    }
    for atmosphere in [
        AtmosphereVariant::Absent,
        AtmosphereVariant::Clear,
        AtmosphereVariant::Shadowed,
        AtmosphereVariant::ViewVolume,
    ] {
        for cloud in [
            CloudVariant::Absent,
            CloudVariant::Sampled,
            CloudVariant::Gpu,
        ] {
            if !atmosphere.present() && cloud.present() {
                continue;
            }
            for shadows in [SunShadows::Disabled, SunShadows::Enabled] {
                let variant = SceneVariant {
                    atmosphere,
                    cloud,
                    shadows,
                    sun: SunShadowMethod::Maps,
                    traced_lamps: false,
                };
                sources.push(("SecondaryScene", Shader::SecondaryScene(variant).source()));
                for sun in [SunShadowMethod::Maps, SunShadowMethod::Traced] {
                    if sun == SunShadowMethod::Traced && shadows == SunShadows::Disabled {
                        continue;
                    }
                    for traced_lamps in [false, true] {
                        let variant = SceneVariant {
                            sun,
                            traced_lamps,
                            ..variant
                        };
                        sources.push(("Scene", Shader::Scene(variant).source()));
                    }
                }
            }
        }
    }
    sources
}

#[cfg(test)]
mod tests {
    use super::{CLUSTER_PROBE_WORKGROUP, Shader};
    use crate::shader_constants::{
        AERIAL_WORKGROUP_EDGE, ATMOSPHERE_EXTENTS, HISTORY_WORKGROUP, LUT_WORKGROUP_EDGE,
        PYRAMID_WORKGROUP_EDGE, RECONSTRUCTION_WORKGROUP_EDGE,
    };

    #[test]
    fn sky_view_ray_reads_zenith_row_from_host_extent() {
        use wgpu::naga::{Expression, Literal};

        let module = wgpu::naga::front::wgsl::parse_str(&Shader::AtmosphereLuts.source())
            .expect("production sky LUT WGSL");
        let (zenith, constant) = module
            .constants
            .iter()
            .find(|(_, constant)| constant.name.as_deref() == Some("SKY_ZENITH_ROW"))
            .expect("zenith row derived from the sky LUT height");
        assert!(matches!(
            module.global_expressions[constant.init],
            Expression::Literal(Literal::U32(row)) if row == ATMOSPHERE_EXTENTS[2][1] - 1
        ));
        let (_, ray) = module
            .functions
            .iter()
            .find(|(_, function)| function.name.as_deref() == Some("sky_view_ray"))
            .expect("sky LUT ray mapping");
        assert!(
            ray.expressions
                .iter()
                .any(|(_, expression)| matches!(*expression, Expression::Constant(handle) if handle == zenith)),
            "sky_view_ray must use the derived zenith row"
        );
    }

    #[test]
    fn lighting_and_display_functions_read_generated_parameters() {
        for (shader, function, expected) in [
            (
                Shader::AtmosphereLuts,
                "medium_densities",
                &[
                    "RAYLEIGH_SCALE_HEIGHT",
                    "MIE_SCALE_HEIGHT",
                    "OZONE_CENTRE",
                    "OZONE_HALF_WIDTH",
                ][..],
            ),
            (
                Shader::AtmosphereLuts,
                "night_terminal",
                &["NIGHT_TERMINAL_MAX_ALTITUDE"][..],
            ),
            (
                Shader::AtmosphereAerial,
                "project_spectrum",
                &["REC709_LUMA"][..],
            ),
            (Shader::Display, "exposure_zone", &["REC709_LUMA"][..]),
            (Shader::Display, "srgb_encode", &["SRGB_SLOPE"][..]),
            (
                Shader::Exposure,
                "resolve",
                &["REC709_LUMA", "INCIDENT_METER_LUX_SCALE", "ADAPTED_LUX"][..],
            ),
        ] {
            let module =
                wgpu::naga::front::wgsl::parse_str(&shader.source()).expect("production WGSL");
            let body = module
                .functions
                .iter()
                .map(|(_, body)| body)
                .chain(module.entry_points.iter().map(|entry| &entry.function))
                .find(|body| body.name.as_deref() == Some(function))
                .expect(function);
            for name in expected {
                assert!(
                    body.expressions.iter().any(|(_, expression)| {
                        matches!(*expression, wgpu::naga::Expression::Constant(handle)
                        if module.constants[handle].name.as_deref() == Some(name))
                    }),
                    "{function} must read {name}"
                );
            }
        }
    }

    #[test]
    fn composed_compute_entries_match_host_dispatch_dimensions() {
        let square = |edge| [edge, edge, 1];
        for (shader, names, workgroup) in [
            (Shader::History, &["publish"][..], [HISTORY_WORKGROUP, 1, 1]),
            (
                Shader::ClusterProbe,
                &["copy_lists"][..],
                [CLUSTER_PROBE_WORKGROUP, 1, 1],
            ),
            (
                Shader::AtmosphereAerial,
                &["aerial_main"][..],
                square(AERIAL_WORKGROUP_EDGE),
            ),
            (
                Shader::AtmosphereLuts,
                &["transmittance_main", "multiple_main", "sky_main"][..],
                square(LUT_WORKGROUP_EDGE),
            ),
            (
                Shader::Pyramid,
                &["pyramid_mip0", "pyramid_down"][..],
                square(PYRAMID_WORKGROUP_EDGE),
            ),
            (
                Shader::Reconstruction,
                &["reconstruct"][..],
                square(RECONSTRUCTION_WORKGROUP_EDGE),
            ),
            (
                Shader::Reference,
                &["accumulate"][..],
                square(RECONSTRUCTION_WORKGROUP_EDGE),
            ),
        ] {
            let module =
                wgpu::naga::front::wgsl::parse_str(&shader.source()).expect("production WGSL");
            for name in names {
                let entry = module
                    .entry_points
                    .iter()
                    .find(|entry| entry.name == *name)
                    .expect(name);
                assert_eq!(entry.workgroup_size, workgroup, "{name}");
            }
        }
    }
}
