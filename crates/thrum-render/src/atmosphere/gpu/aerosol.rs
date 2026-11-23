use crate::{
    atmosphere::Aerosol,
    binding,
    shaders::{Shader, wgsl},
};

use super::EARTH;

const BINDING: u32 = 8;
pub(in crate::atmosphere) const BYTES: u64 = size_of::<AerosolUniform>() as u64;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct AerosolUniform {
    extinction: [[f32; 4]; 2],
    scattering: [[f32; 4]; 2],
    surface_extinction: [f32; 4],
}

impl AerosolUniform {
    fn new(aerosol: Aerosol) -> Self {
        let extinction = aerosol.band_extinction();
        let scattering = if aerosol == Aerosol::default() {
            [EARTH.mie_scattering; 8]
        } else {
            extinction.map(|sigma| sigma * (EARTH.mie_scattering / EARTH.mie_extinction))
        };
        Self {
            extinction: bytemuck::cast(extinction),
            scattering: bytemuck::cast(scattering),
            surface_extinction: [680.0, 550.0, 440.0, 550.0]
                .map(|wavelength| aerosol.extinction_at(wavelength)),
        }
    }
}

pub(super) fn write(queue: &wgpu::Queue, buffer: &wgpu::Buffer, aerosol: Aerosol) {
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&AerosolUniform::new(aerosol)));
}

pub(super) fn layout() -> wgpu::BindGroupLayoutEntry {
    binding::uniform(BINDING, wgpu::ShaderStages::COMPUTE, BYTES)
}

pub(super) fn entry(buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding: BINDING,
        resource: buffer.as_entire_binding(),
    }
}

pub(super) fn compile(
    device: &wgpu::Device,
    shader: Shader,
    label: &str,
    aerosol: Option<&wgpu::Buffer>,
) -> wgpu::ShaderModule {
    if aerosol.is_none() {
        return shader.compile(device, label);
    }
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source(shader).into()),
    })
}

pub(super) fn source(shader: Shader) -> String {
    let entry = match shader {
        Shader::AtmosphereLuts => concat!(
            wgsl!(@lut "atmosphere/aerosol_multiple.wgsl", "atmosphere/aerosol_sky_view.wgsl"),
            wgsl!("atmosphere/ground.wgsl")
        ),
        Shader::AtmosphereAerial => wgsl!("atmosphere/aerosol_aerial.wgsl"),
        _ => unreachable!("aerosol is a variant of the atmosphere compute stages"),
    };
    if matches!(shader, Shader::AtmosphereLuts) {
        assemble(entry)
    } else {
        aerial_source(entry)
    }
}

pub(super) fn aerial_source(entry: &str) -> String {
    assemble(&format!(
        "{}\n{entry}",
        wgsl!(@bytes "atmosphere/aerial.wgsl")
    ))
}

fn assemble(entry: &str) -> String {
    let body = format!(
        "{}{}{}{}{}{}\n{entry}",
        wgsl!("atmosphere/storage.wgsl"),
        wgsl!("atmosphere/geometry.wgsl"),
        wgsl!("atmosphere/common.wgsl"),
        wgsl!(@transport "atmosphere/aerosol_extinction.wgsl", "atmosphere/aerosol_scattering.wgsl"),
        wgsl!("atmosphere/night.wgsl"),
        wgsl!("atmosphere/night_aerosol.wgsl"),
    );
    let transport = EARTH.shader(&body);
    format!(
        "struct AerosolUniform {{
            extinction: array<vec4<f32>, 2>,
            scattering: array<vec4<f32>, 2>,
            surface_extinction: vec4<f32>,
        }}
        @group(0) @binding({BINDING}) var<uniform> aerosol: AerosolUniform;
        {transport}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_aerosol_record_matches_the_compiled_shaders() {
        for shader in [Shader::AtmosphereLuts, Shader::AtmosphereAerial] {
            let module = wgpu::naga::front::wgsl::parse_str(&source(shader)).expect("WGSL");
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .expect("portable module");
            let ty = module
                .types
                .iter()
                .find_map(|(_, ty)| (ty.name.as_deref() == Some("AerosolUniform")).then_some(ty))
                .expect("aerosol record");
            let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
                panic!("aerosol struct");
            };
            assert_eq!(u64::from(*span), BYTES);
            assert_eq!(
                members
                    .iter()
                    .map(|member| member.offset as usize)
                    .collect::<Vec<_>>(),
                [
                    std::mem::offset_of!(AerosolUniform, extinction),
                    std::mem::offset_of!(AerosolUniform, scattering),
                    std::mem::offset_of!(AerosolUniform, surface_extinction),
                ]
            );
        }
    }
}
