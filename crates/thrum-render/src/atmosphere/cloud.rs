//! Optional Sun-path optical depth. The provider owns its immutable contents.
use std::sync::Arc;

use crate::records::CloudField;
use crate::synthetic::RenderError;

const CLOUD_FIELD_BYTES: usize = size_of::<CloudField>();

/// GPU provider mapping. Plain fields use `far[1] = 0`. An atlas uses
/// `far[1] = 1`, with near, middle and outer windows in the top-left,
/// top-right and bottom-left quadrants of the texture, respectively.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSunMapping {
    /// Outer world XZ rectangle, minimum then maximum.
    pub region: [f32; 4],
    /// Plane height, XZ displacement and validity (zero or one).
    pub plane_drift: [f32; 4],
    /// Unit path direction and outer border width in metres.
    pub path_border: [f32; 4],
    /// Far optical depth, atlas flag, and padding.
    pub far: [f32; 4],
    /// Near window minimum XZ, width in metres, and padding.
    pub near: [f32; 4],
    /// Middle window minimum XZ, width in metres, and padding.
    pub middle: [f32; 4],
}

/// A provider's sampled optical depth along the current true Sun ray.
///
/// The field applies below one horizontal plane. World XZ is projected along
/// the Sun to that plane, then displaced into the field's declared region.
/// In the CPU-fed form, samples outside that region, at/above the plane,
/// marked invalid, or below the minimum Sun elevation receive exactly unit
/// transmission. Its provider should fade optical depth to zero at those limits.
/// GPU-fed fields instead supply their own path direction and continuous border
/// blend to a far transmittance; they remain sampled below their direction's floor.
/// Values are stored as filtered fp16 optical depth, independently of exposure.
#[derive(Clone, Debug)]
pub struct SunOpticalDepth {
    gpu: Option<(wgpu::Texture, wgpu::Buffer)>,
    allocate_gpu: bool,
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
    /// Declare a GPU target prepared with each scene/device generation. Its
    /// provider borrows the texture and [`GpuSunMapping`] through
    /// [`crate::contribution::SceneSurface::sun_path_target`] during `rebuild`.
    /// The scene allocates the shared target; only the provider writes it.
    /// This keeps a replaced device from inheriting retired GPU handles.
    /// # Errors
    /// Rejects an empty extent; device limits are checked at preparation.
    pub fn gpu_target(extent: [u32; 2], source_epoch: u64) -> Result<Self, RenderError> {
        if extent.contains(&0) {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            gpu: None,
            allocate_gpu: true,
            width: extent[0],
            height: extent[1],
            pixels: Arc::from([]),
            plane_m: 0.0,
            region: [0.0; 4],
            displacement_m: [0.0; 2],
            drift_metres_per_second: [0.0; 2],
            minimum_sun_sine: 0.0,
            source_epoch,
            valid: true,
        })
    }
    /// Register a provider's GPU-written field and [`GpuSunMapping`]. The provider records
    /// all writes in the scene's encoder and declares them and opaque reads in
    /// its effect graph. The true Sun and CPU-fed diagnostic are unchanged.
    ///
    /// # Errors
    /// Rejects textures or buffers without the required sampled/uniform usage,
    /// incompatible dimensions/formats, or an undersized mapping buffer.
    pub fn from_gpu(
        texture: wgpu::Texture,
        mapping: wgpu::Buffer,
        source_epoch: u64,
    ) -> Result<Self, RenderError> {
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.depth_or_array_layers() != 1
            || texture.mip_level_count() != 1
            || texture.sample_count() != 1
            || !matches!(
                texture.format(),
                wgpu::TextureFormat::R16Float | wgpu::TextureFormat::Rgba16Float
            )
            || !texture.usage().contains(
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            )
            || mapping.size() != size_of::<GpuSunMapping>() as u64
            || !mapping
                .usage()
                .contains(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::STORAGE)
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            width: texture.width(),
            height: texture.height(),
            pixels: Arc::from([]),
            plane_m: 0.0,
            region: [0.0; 4],
            displacement_m: [0.0; 2],
            drift_metres_per_second: [0.0; 2],
            minimum_sun_sine: 0.0,
            source_epoch,
            valid: true,
            gpu: Some((texture, mapping)),
            allocate_gpu: false,
        })
    }

    pub(crate) fn gpu_written(&self) -> bool {
        self.gpu.is_some() || self.allocate_gpu
    }

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
            || optical_depth.iter().any(|x| {
                !x.is_finite() || !(0.0..=crate::shader_constants::MAX_FINITE_HALF).contains(x)
            })
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            gpu: None,
            allocate_gpu: false,
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
    fn pack(&self, seconds: f64) -> [u8; CLOUD_FIELD_BYTES] {
        let drift: [f32; 2] = std::array::from_fn(|i| {
            (f64::from(self.displacement_m[i])
                + f64::from(self.drift_metres_per_second[i]) * seconds) as f32
        });
        let supported = drift.iter().all(|x| x.is_finite());
        let drift = if supported { drift } else { [0.0; 2] };
        bytemuck::cast(CloudField {
            region: self.region,
            plane_drift: [self.plane_m, drift[0], drift[1], self.minimum_sun_sine],
            validity: [f32::from(self.valid && supported), 0.0, 0.0, 0.0],
        })
    }
}

#[derive(Clone)]
pub(crate) struct CloudPrepared {
    packed: [u8; CLOUD_FIELD_BYTES],
    field: SunOpticalDepth,
    texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) uniform: wgpu::Buffer,
}

impl CloudPrepared {
    pub(crate) fn gpu_target(&self) -> Option<(wgpu::Texture, wgpu::Buffer)> {
        self.field
            .gpu_written()
            .then(|| (self.texture.clone(), self.uniform.clone()))
    }
    pub(crate) fn append_reads(
        &self,
        graph: &mut crate::graph::FrameGraph,
        accesses: &mut Vec<crate::graph::Access>,
    ) -> Result<(), crate::graph::GraphError> {
        use crate::graph::{Access, AccessKind, ResourceDescriptor, ResourceScope};
        let texture = graph.resource(
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Imported { initialized: true },
        )?;
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: self.uniform.size(),
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.extend([
            Access::new(texture, 0..1, AccessKind::SampledRead),
            Access::new(uniform, 0..self.uniform.size(), AccessKind::UniformRead),
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
        if field.allocate_gpu {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("provider Sun-path optical depth"),
                size: wgpu::Extent3d {
                    width: field.width,
                    height: field.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("provider Sun-path mapping"),
                size: size_of::<GpuSunMapping>() as u64,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            return Ok(Self {
                packed: [0; CLOUD_FIELD_BYTES],
                field: field.clone(),
                view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
                texture,
                uniform,
            });
        }
        if let Some((texture, uniform)) = &field.gpu {
            return Ok(Self {
                packed: [0; CLOUD_FIELD_BYTES],
                field: field.clone(),
                texture: texture.clone(),
                view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
                uniform: uniform.clone(),
            });
        }
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
            size: CLOUD_FIELD_BYTES as u64,
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
        self.field.width == field.width
            && self.field.height == field.height
            && self.field.gpu_written() == field.gpu_written()
            && match (&self.field.gpu, &field.gpu) {
                (Some((a, u)), Some((b, v))) => a == b && u == v,
                (None, None) => true,
                _ => false,
            }
    }

    pub(crate) fn update(&mut self, queue: &wgpu::Queue, field: &SunOpticalDepth, seconds: f64) {
        if field.gpu_written() {
            return;
        }
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
