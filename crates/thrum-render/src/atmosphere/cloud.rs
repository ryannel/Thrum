//! Optional Sun-path optical depth. The provider owns its immutable contents.
use std::sync::Arc;

use crate::synthetic::RenderError;

/// A provider's sampled optical depth along the current true Sun ray.
///
/// The field applies below one horizontal plane. World XZ is projected along
/// the Sun to that plane, then displaced into the field's declared region.
/// Samples outside that region, at/above the plane, marked invalid, or below
/// the minimum Sun elevation receive exactly unit transmission. The provider
/// owns continuity at these limits and should fade its field to zero there.
/// Values are stored as filtered fp16 optical depth, independently of exposure.
#[derive(Clone, Debug)]
pub struct SunOpticalDepth {
    width: u32,
    height: u32,
    pixels: Arc<[u16]>,
    plane_m: f32,
    region: [f32; 4],
    displacement_m: [f32; 2],
    drift_metres_per_second: [f32; 2],
    minimum_sun_sine: f32,
    source_epoch: u64,
    valid: bool,
}

impl SunOpticalDepth {
    /// Admit a finite nonnegative row-major optical-depth field. Region is
    /// `[min_x, min_z, max_x, max_z]`, in metres. The minimum sine must be
    /// positive, making horizontal and below-horizon rays unsupported.
    ///
    /// # Errors
    /// Returns invalid lighting for inconsistent dimensions, values beyond
    /// fp16's finite range, nonfinite metadata or an empty region.
    pub fn new(
        extent: [u32; 2],
        optical_depth: &[f32],
        plane_m: f32,
        region: [f32; 4],
        minimum_sun_sine: f32,
        source_epoch: u64,
    ) -> Result<Self, RenderError> {
        let [width, height] = extent;
        let count = u64::from(width) * u64::from(height);
        if width == 0
            || height == 0
            || usize::try_from(count).ok() != Some(optical_depth.len())
            || !plane_m.is_finite()
            || !region.iter().all(|x| x.is_finite())
            || region[0] >= region[2]
            || region[1] >= region[3]
            || !minimum_sun_sine.is_finite()
            || !(0.0..1.0).contains(&minimum_sun_sine)
            || minimum_sun_sine == 0.0
            || optical_depth
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=65_504.0).contains(x))
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            width,
            height,
            pixels: optical_depth
                .iter()
                .map(|x| half::f16::from_f32(*x).to_bits())
                .collect(),
            plane_m,
            region,
            displacement_m: [0.0; 2],
            drift_metres_per_second: [0.0; 2],
            minimum_sun_sine,
            source_epoch,
            valid: true,
        })
    }

    /// Advect the immutable field by a finite world XZ displacement in metres.
    ///
    /// # Errors
    /// Returns invalid lighting for nonfinite displacement.
    pub fn with_displacement(mut self, displacement_m: [f32; 2]) -> Result<Self, RenderError> {
        if !displacement_m.iter().all(|x| x.is_finite()) {
            return Err(RenderError::InvalidLighting);
        }
        self.displacement_m = displacement_m;
        Ok(self)
    }

    /// Advect with the atmosphere's simulation time, in world XZ metres per second.
    ///
    /// # Errors
    /// Returns invalid lighting for nonfinite velocity.
    pub fn with_drift(mut self, metres_per_second: [f32; 2]) -> Result<Self, RenderError> {
        if !metres_per_second.iter().all(|x| x.is_finite()) {
            return Err(RenderError::InvalidLighting);
        }
        self.drift_metres_per_second = metres_per_second;
        Ok(self)
    }

    /// Mark this provider's current sample supported or unsupported.
    #[must_use]
    pub fn with_validity(mut self, valid: bool) -> Self {
        self.valid = valid;
        self
    }

    /// Provider's source revision, independent of advection and exposure.
    #[must_use]
    pub fn source_epoch(&self) -> u64 {
        self.source_epoch
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite f64 simulation displacement rounds once into the GPU mapping, overflow disables this sample"
    )]
    fn pack(&self, seconds: f64) -> [u8; 48] {
        let drift: [f32; 2] = std::array::from_fn(|i| {
            (f64::from(self.displacement_m[i])
                + f64::from(self.drift_metres_per_second[i]) * seconds) as f32
        });
        let supported = drift.iter().all(|x| x.is_finite());
        let drift = if supported { drift } else { [0.0; 2] };
        let words = [
            self.region[0],
            self.region[1],
            self.region[2],
            self.region[3],
            self.plane_m,
            drift[0],
            drift[1],
            self.minimum_sun_sine,
            f32::from(u8::from(self.valid && supported)),
            0.0,
            0.0,
            0.0,
        ];
        let mut bytes = [0; 48];
        for (destination, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
            *destination = value.to_le_bytes();
        }
        bytes
    }
}

pub(crate) struct CloudPrepared {
    packed: [u8; 48],
    field: SunOpticalDepth,
    texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) uniform: wgpu::Buffer,
}

impl CloudPrepared {
    pub(crate) fn append_reads(
        &self,
        graph: &mut crate::graph::FrameGraph,
        accesses: &mut Vec<crate::graph::Access>,
    ) -> Result<(), crate::graph::GraphError> {
        use crate::graph::{Access, AccessKind, ResourceDescriptor, ResourceScope};
        let texture = graph.resource(
            ResourceDescriptor::Texture {
                width: self.texture.width(),
                height: self.texture.height(),
                layers: 1,
                mips: 1,
                samples: 1,
                format: self.texture.format(),
                usage: self.texture.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: 48,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.extend([
            Access {
                resource: texture,
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            },
            Access {
                resource: uniform,
                range: 0..48,
                kind: AccessKind::UniformRead,
                store: true,
            },
        ]);
        Ok(())
    }

    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        field: &SunOpticalDepth,
        seconds: f64,
    ) -> Result<Self, RenderError> {
        if field.width.max(field.height) > device.limits().max_texture_dimension_2d {
            return Err(RenderError::SceneCapacity);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Sun-path optical depth"),
            size: wgpu::Extent3d {
                width: field.width,
                height: field.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sun-path optical depth support"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let result = Self {
            packed: field.pack(seconds),
            field: field.clone(),
            texture,
            view,
            uniform,
        };
        result.write_pixels(queue, field);
        queue.write_buffer(&result.uniform, 0, &result.packed);
        Ok(result)
    }

    pub(crate) fn accepts(&self, field: &SunOpticalDepth) -> bool {
        self.field.width == field.width && self.field.height == field.height
    }

    pub(crate) fn update(&mut self, queue: &wgpu::Queue, field: &SunOpticalDepth, seconds: f64) {
        if !Arc::ptr_eq(&self.field.pixels, &field.pixels) {
            self.write_pixels(queue, field);
        }
        let packed = field.pack(seconds);
        if packed != self.packed {
            queue.write_buffer(&self.uniform, 0, &packed);
        }
        self.packed = packed;
        self.field = field.clone();
    }

    fn write_pixels(&self, queue: &wgpu::Queue, field: &SunOpticalDepth) {
        let bytes: Vec<_> = field
            .pixels
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        queue.write_texture(
            self.texture.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(field.width * 2),
                rows_per_image: Some(field.height),
            },
            self.texture.size(),
        );
    }
}

#[cfg(test)]
mod tests;
