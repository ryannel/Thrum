//! On-demand read-only bindings for independent scene contributions.

use super::gpu::AtmospherePrepared;
use crate::binding;
use crate::shaders::wgsl;

/// Read-only atmosphere products, created on the contribution's request.
///
/// The WGSL uses group 0. A consumer appends its entry points and binds `group`
/// against `layout`. All returned light is un-pre-exposed scene-linear Rec.709.
/// The five operations are `sun_irradiance(world_point)` in RGB lux,
/// `environment_sky_radiance(direction)` in RGB radiance,
/// `environment_diffuse(normal)` in RGB lux, `environment_aerial(world_point, uv)`
/// returning dimensionless transmission and RGB in-scatter, and
/// `environment_reflection(direction, roughness, view_cosine)` returning filtered
/// RGB radiance and the dimensionless split-sum pair.
/// `environment_sun_direction()` returns the same direction toward the Sun
/// that the forward shader reads from the scene's directional record.
///
/// Sky, diffuse and reflection belong to the active camera. Aerial starts at
/// that camera, with top-left normalized screen coordinates. Without a cloud
/// provider it clamps to 32 km. With one, sky-ending rays extend to the air's
/// boundary using chart-ratio transmission and optical-depth-apportioned sky
/// in-scatter. Points at 32 km and nearer retain the finite-volume arithmetic.
/// Reflection clamps roughness to 0.045–1 and view cosine to 0–1. Night support
/// is 0–15 km camera altitude, with the existing Sun-only fallback outside it.
/// Sun light supports arbitrary world points, clamps below the first 6.15 m
/// chart row, and is exactly zero behind the local planet horizon.
///
/// Call `environment_storage_scale()` and multiply the finished scene colour
/// once when writing the scene target. The lighting operations never apply it.
/// Resources remain owned by the atmosphere and must be rebuilt on replacement.
/// No chart dimensions, storage gains or mip choice belong to the consumer.
#[derive(Debug)]
pub struct EnvironmentProducts {
    /// Read-only group 0 layout for this library.
    pub layout: wgpu::BindGroupLayout,
    /// WGSL library to prepend to the consumer's own entry points.
    pub shader_source: String,
    groups: [wgpu::BindGroup; 2],
    exposure: [wgpu::Buffer; 2],
}

impl EnvironmentProducts {
    /// Resolved exposure uniforms, in the same side order as the environment groups.
    /// A pre-tone diagnostic can bind the first four floats to encode a fixed display
    /// colour in the scene's storage units without bringing in the environment textures.
    #[must_use]
    pub fn exposure_buffers(&self) -> &[wgpu::Buffer; 2] {
        &self.exposure
    }

    /// Select the same resolved exposure side as the scene's forward pass.
    /// The runtime supplies that side with the contribution frame.
    #[must_use]
    pub fn group(&self, exposure_side: usize) -> Option<&wgpu::BindGroup> {
        self.groups.get(exposure_side)
    }
}

pub(crate) struct ProductSelection<'a> {
    pub(crate) far_aerial: bool,
    pub(crate) cloud: Option<&'a super::CloudPrepared>,
    pub(crate) reflection: bool,
}

pub(crate) fn create(
    atmosphere: &AtmospherePrepared,
    device: &wgpu::Device,
    directional: &wgpu::Buffer,
    environment_light: &wgpu::Buffer,
    exposure: &[wgpu::Buffer; 2],
    selection: ProductSelection<'_>,
) -> EnvironmentProducts {
    let ProductSelection {
        far_aerial,
        cloud,
        reflection,
    } = selection;
    let visibility = wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE;
    let buffer = |binding, bytes| binding::uniform(binding, visibility, bytes);
    let mut entries = vec![
        buffer(0, super::gpu::UNIFORM_BYTES),
        buffer(3, crate::light::ENVIRONMENT_BYTES),
        buffer(10, size_of::<[f32; 4]>() as u64),
        buffer(11, crate::light::DIRECTIONAL_BYTES),
    ];
    for (binding, view_dimension) in [
        (1, wgpu::TextureViewDimension::D2),
        (4, wgpu::TextureViewDimension::D2),
        (5, wgpu::TextureViewDimension::D2Array),
        (6, wgpu::TextureViewDimension::D2Array),
        (7, wgpu::TextureViewDimension::Cube),
        (8, wgpu::TextureViewDimension::D2),
    ] {
        entries.push(binding::texture(
            binding,
            visibility,
            wgpu::TextureSampleType::Float { filterable: true },
            view_dimension,
        ));
    }
    for binding in [2, 9] {
        entries.push(binding::sampler(
            binding,
            visibility,
            wgpu::SamplerBindingType::Filtering,
        ));
    }
    if let Some(cloud) = cloud {
        entries.extend([
            binding::texture(
                13,
                visibility,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2,
            ),
            buffer(14, cloud.uniform.size()),
        ]);
        if cloud.band.is_some() {
            entries.extend(super::ground_band::GroundBandPrepared::layout(visibility));
        }
    }
    // Bindings 7–9 are the filtered reflection pair and its sampler.
    let selected = |binding: u32| reflection || !(7..=9).contains(&binding);
    entries.retain(|entry| selected(entry.binding));
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lent environment products"),
        entries: &entries,
    });
    let sky = atmosphere
        .textures
        .sky_view
        .create_view(&wgpu::TextureViewDescriptor::default());
    let (aerial, sampler, uniform) = atmosphere.aerial_sampled();
    let (cube, dfg, reflection_sampler) = atmosphere.reflection_sampled();
    let mut bindings = vec![
        wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(sampler),
        },
        wgpu::BindGroupEntry {
            binding: 3,
            resource: environment_light.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
            binding: 9,
            resource: wgpu::BindingResource::Sampler(reflection_sampler),
        },
        wgpu::BindGroupEntry {
            binding: 11,
            resource: directional.as_entire_binding(),
        },
    ];
    for (binding, view) in [
        (1, atmosphere.transmittance_view()),
        (4, &sky),
        (5, aerial[0]),
        (6, aerial[1]),
        (7, cube),
        (8, dfg),
    ] {
        bindings.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        });
    }
    if let Some(cloud) = cloud {
        bindings.extend([
            wgpu::BindGroupEntry {
                binding: 13,
                resource: wgpu::BindingResource::TextureView(&cloud.view),
            },
            wgpu::BindGroupEntry {
                binding: 14,
                resource: cloud.uniform.as_entire_binding(),
            },
        ]);
        if let Some(band) = &cloud.band {
            bindings.extend(band.entries());
        }
    }
    bindings.retain(|entry| selected(entry.binding));
    let groups = std::array::from_fn(|side| {
        bindings.push(wgpu::BindGroupEntry {
            binding: 10,
            resource: exposure[side].as_entire_binding(),
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lent environment read-only products"),
            layout: &layout,
            entries: &bindings,
        });
        bindings.pop();
        group
    });
    let lighting = atmosphere.overlay_lighting();
    let mut shader_source =
        if atmosphere.view_volume.is_some() || lighting != super::overlay::Lighting::Clear {
            library_with_volume(far_aerial, atmosphere.view_volume.is_some(), lighting)
        } else if far_aerial {
            crate::shaders::Shader::EnvironmentProductsFar
                .source()
                .into_owned()
        } else {
            crate::shaders::Shader::EnvironmentProducts
                .source()
                .into_owned()
        };
    if let Some(cloud) = cloud {
        shader_source.push_str(&cloud.variant().sampling_source_for(0, "atmosphere"));
    }
    EnvironmentProducts {
        layout,
        shader_source,
        groups,
        exposure: exposure.clone(),
    }
}

pub(crate) fn library(far_aerial: bool) -> String {
    library_with_volume(far_aerial, false, super::overlay::Lighting::Clear)
}

fn library_with_volume(
    far_aerial: bool,
    view_volume: bool,
    lighting: super::overlay::Lighting,
) -> String {
    let far_sky = if view_volume {
        wgsl!("atmosphere/products_terminal.wgsl")
    } else if lighting == super::overlay::Lighting::OverlayAndHaze {
        wgsl!("atmosphere/overlay_aerial_terminal.wgsl")
    } else {
        "fn environment_far_sky(direction: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
            return environment_sky_radiance(direction);
        }"
    };
    let far = if far_aerial {
        format!(
            "{}\n{far_sky}",
            wgsl!("atmosphere/products_aerial_far.wgsl")
        )
    } else {
        wgsl!("atmosphere/products_aerial.wgsl").to_owned()
    };
    let products = format!(
        "{}\n{}\n{}{}{}",
        wgsl!("directional_light.wgsl"),
        wgsl!("environment_light.wgsl"),
        wgsl!("atmosphere/products.wgsl"),
        far,
        super::overlay::reflection_product(lighting)
    );
    // Reuse the receiver Sun and cumulative aerial arithmetic verbatim. Only
    // the resource declarations and active-frame name differ in this group.
    let sun = wgsl!("synthetic_sun.wgsl")
        .split_once("\n\n")
        .expect("Sun declarations followed by function")
        .1
        .replace("aerial_frame", "atmosphere");
    let aerial = if view_volume {
        concat!(
            wgsl!(@bytes "synthetic_aerial.wgsl"),
            wgsl!(@bytes "atmosphere/volume_sample.wgsl"),
            wgsl!(@bytes "synthetic_aerial_decode.wgsl"),
        )
    } else {
        super::overlay::aerial_source(lighting)
    };
    let sample = aerial
        .split_once("// Cumulative")
        .expect("aerial sample marker")
        .1
        .split_once("fn aerial_uv")
        .expect("sample precedes screen helper")
        .0;
    let decode = aerial
        .split_once("// Decode")
        .expect("aerial decode marker")
        .1;
    let grid = if view_volume {
        super::gpu::view_volume::grid_source()
    } else {
        String::new()
    };
    let source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n// Cumulative{}\n// Decode{}\n{}",
        wgsl!("atmosphere/storage.wgsl"),
        wgsl!("atmosphere/geometry.wgsl"),
        wgsl!("atmosphere/common.wgsl"),
        wgsl!("atmosphere/irradiance.wgsl"),
        products,
        sun,
        sample,
        decode.replace("aerial_frame", "atmosphere"),
        grid
    );
    if lighting == super::overlay::Lighting::Clear {
        source
    } else {
        super::overlay::with_reflection_gain(&source)
    }
}

#[cfg(test)]
mod tests;
