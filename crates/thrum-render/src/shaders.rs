//! Production WGSL recipes. The catalog also enumerates the sources reflected by the ABI checks.
use std::borrow::Cow;

use crate::{capabilities::SunShadowMethod, synthetic::scene_source::SunShadows};

/// A source marker gives reflected declarations a file identity across compositions.
macro_rules! wgsl {
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
            include_str!(concat!(env!("OUT_DIR"), "/sun.wgsl")),
            $crate::shaders::wgsl!(@file "atmosphere/sky.wgsl"),
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
            include_str!(concat!(env!("OUT_DIR"), "/cluster.wgsl")),
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
    ("synthetic/display/legacy_display.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/display/legacy_display.wgsl"),
        )
    };
    ("synthetic/exposure.wgsl") => {
        concat!(
            $crate::shaders::wgsl!(@file "records/serials.wgsl"),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/exposure.wgsl"),
        )
    };
    ("synthetic/history_gpu.wgsl") => {
        concat!(
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
            $crate::shaders::wgsl!(@file "records/observer.wgsl"),
            $crate::shaders::wgsl!(@file "records/exposure.wgsl"),
            $crate::shaders::wgsl!(@file "synthetic/stars.wgsl"),
        )
    };
    ("synthetic/taau.wgsl") => {
        concat!(
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
            $crate::shaders::wgsl!(@file "synthetic_aerial.wgsl"),
        )
    };
    ("synthetic_display.wgsl") => {
        concat!(
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
        }

        impl Shader {
            pub(crate) fn source(self) -> Cow<'static, str> {
                match self {
                    $(Self::$name => ($source).into(),)*
                    Self::Scene(variant) => crate::synthetic::scene_source::main_source(variant).into(),
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
    Sprite => wgsl!("sprite/pass.wgsl");
    Exposure => crate::synthetic::exposure::SHADER;
    Temporal => wgsl!("synthetic/temporal.wgsl");
    History => wgsl!("synthetic/history_gpu.wgsl");
    NativeWitness => wgsl!("synthetic/native_witness.wgsl");
    Shadow => wgsl!("synthetic/shadow.wgsl");
    ClusterProbe => wgsl!("synthetic/cluster_probe.wgsl");
    Cluster => wgsl!("synthetic/cluster.wgsl");
    RayBuild => wgsl!("synthetic/raytrace/build_probe.wgsl");
    RayView => wgsl!("synthetic/raytrace/view.wgsl");
    CopyMarker => wgsl!("synthetic/readback/copy_marker.wgsl");
    Visibility => visibility_source();
    Pyramid => wgsl!("visibility/pyramid.wgsl");
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
    EnvironmentProducts => crate::atmosphere::products::library(false);
    EnvironmentProductsFar => crate::atmosphere::products::library(true);
    AtmosphereLuts => crate::atmosphere::gpu::lut_source();
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
    for atmosphere in [
        AtmosphereVariant::Absent,
        AtmosphereVariant::Clear,
        AtmosphereVariant::Shadowed,
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
