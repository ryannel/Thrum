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
}

impl EnvironmentProducts {
    /// Select the same resolved exposure side as the scene's forward pass.
    /// The runtime supplies that side with the contribution frame.
    #[must_use]
    pub fn group(&self, exposure_side: usize) -> Option<&wgpu::BindGroup> {
        self.groups.get(exposure_side)
    }
}

pub(super) fn create(
    atmosphere: &AtmospherePrepared,
    device: &wgpu::Device,
    directional: &wgpu::Buffer,
    environment_light: &wgpu::Buffer,
    exposure: &[wgpu::Buffer; 2],
    far_aerial: bool,
) -> EnvironmentProducts {
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
    EnvironmentProducts {
        layout,
        shader_source: if atmosphere.view_volume.is_some() {
            library_with_volume(far_aerial, true)
        } else if far_aerial {
            crate::shaders::Shader::EnvironmentProductsFar
                .source()
                .into_owned()
        } else {
            crate::shaders::Shader::EnvironmentProducts
                .source()
                .into_owned()
        },
        groups,
    }
}

pub(crate) fn library(far_aerial: bool) -> String {
    library_with_volume(far_aerial, false)
}

fn library_with_volume(far_aerial: bool, view_volume: bool) -> String {
    let products = format!(
        "{}\n{}\n{}{}{}",
        wgsl!("directional_light.wgsl"),
        wgsl!("environment_light.wgsl"),
        wgsl!("atmosphere/products.wgsl"),
        if far_aerial {
            wgsl!("atmosphere/products_aerial_far.wgsl")
        } else {
            wgsl!("atmosphere/products_aerial.wgsl")
        },
        wgsl!("atmosphere/products_reflection.wgsl")
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
        wgsl!("synthetic_aerial.wgsl")
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
    format!(
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
    )
}

#[cfg(test)]
mod tests;
