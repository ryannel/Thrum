//! The aerial and sky passes' shared observer record. Each observer owns one
//! buffer; all use the same packing and transport shader.

use super::{AtmosphereFrame, SyntheticView};

pub(super) use crate::records::Atmosphere as AtmosphereUniform;

impl AtmosphereUniform {
    pub(super) fn new(frame: AtmosphereFrame, view: &SyntheticView, altitude: f32) -> Self {
        let [x, y, z] = view.camera_position();
        let [sx, sy, sz] = frame.toward_sun;
        let [r, g, b] = frame.solar_colour;
        Self {
            inverse_view_projection: view.inverse_view_proj_columns(),
            camera_altitude: [x, y, z, altitude],
            sun_lux: [sx, sy, sz, frame.solar_illuminance_lux],
            colour_pre_exposure: [r, g, b, crate::render_config::PRE_EXPOSURE],
            sky_mapping: crate::atmosphere::model::sky_mapping(
                altitude.clamp(0.0, crate::shader_constants::ATMOSPHERE_TOP),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_record_matches_the_transport_shader_and_previous_bytes() {
        let module = wgpu::naga::front::wgsl::parse_str(
            &crate::atmosphere::spectrum::EARTH.shader(super::super::LUT_SHADER),
        )
        .expect("atmosphere shader");
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some("Atmosphere"))
            .expect("observer");
        let wgpu::naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("observer struct")
        };
        assert_eq!(*span as usize, size_of::<AtmosphereUniform>());
        let offsets = [
            std::mem::offset_of!(AtmosphereUniform, inverse_view_projection),
            std::mem::offset_of!(AtmosphereUniform, camera_altitude),
            std::mem::offset_of!(AtmosphereUniform, sun_lux),
            std::mem::offset_of!(AtmosphereUniform, colour_pre_exposure),
            std::mem::offset_of!(AtmosphereUniform, sky_mapping),
        ];
        for (member, offset) in members.iter().zip(offsets) {
            assert_eq!(member.offset as usize, offset);
        }
        let view = SyntheticView::perspective(
            [123.0, 48.0, -3.0],
            [120.0, 50.0, -20.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.5,
            0.1,
            10000.0,
        )
        .expect("view");
        let frame = AtmosphereFrame::new(
            [0.4, 0.8, -0.2],
            crate::shader_constants::ATMOSPHERE_TOP,
            [1.0; 3],
            48.0,
            1,
        )
        .expect("frame");
        let mut previous = [0_u8; 128];
        previous[..64].copy_from_slice(&view.pack_aux(96, 64)[..64]);
        previous[64..76].copy_from_slice(&view.pack_view(&view, 1.0)[128..140]);
        for (chunk, value) in previous[76..].as_chunks_mut::<4>().0.iter_mut().zip(
            [frame.camera_altitude_m]
                .into_iter()
                .chain(frame.toward_sun)
                .chain([frame.solar_illuminance_lux])
                .chain(frame.solar_colour)
                .chain([crate::render_config::PRE_EXPOSURE])
                .chain(crate::atmosphere::model::sky_mapping(
                    frame.camera_altitude_m,
                )),
        ) {
            *chunk = value.to_le_bytes();
        }
        assert_eq!(
            bytemuck::bytes_of(&AtmosphereUniform::new(
                frame,
                &view,
                frame.camera_altitude_m
            )),
            previous
        );
    }
}
