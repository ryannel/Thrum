//! CPU/WGSL revision 2: explicit little-endian scalar packing, column-major
//! matrices, storage-buffer material/instance tables and four typed D2 arrays.
//! No native Rust layout or unsafe cast is part of the GPU ABI.

/// Shared material/instance ABI revision.
pub const ABI_REVISION: u32 = 2;
/// Packed material-table stride.
pub const MATERIAL_BYTES: usize = 80;
/// Packed instance-table stride.
pub const INSTANCE_BYTES: usize = 192;
/// Base colour, emissive, surface factors, semantic flags, texture-layer indices.
pub const MATERIAL_OFFSETS: [usize; 5] = [0, 16, 32, 48, 64];
/// Current/previous model, padded normal columns and instance/material/skin/source IDs.
pub const INSTANCE_OFFSETS: [usize; 6] = [0, 64, 128, 144, 160, 176];

/// The four fixed, separately typed sampled texture arrays. Array layers share
/// dimensions/format/mips within one class; this requires no binding-array feature.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureClass {
    /// sRGB RGB, linear coverage alpha.
    BaseColor,
    /// Linear tangent-space normals.
    Normal,
    /// Linear occlusion/roughness/metalness.
    Orm,
    /// sRGB emission multiplier.
    Emissive,
}
impl TextureClass {
    /// Fixed slot in CPU records and group 2 bindings 1 through 4.
    pub const fn index(self) -> usize {
        match self {
            Self::BaseColor => 0,
            Self::Normal => 1,
            Self::Orm => 2,
            Self::Emissive => 3,
        }
    }
    /// Portable uncompressed format for the first profile.
    pub const fn format(self) -> wgpu::TextureFormat {
        match self {
            Self::BaseColor | Self::Emissive => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Normal | Self::Orm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
    /// Default identity texel, at reserved layer zero.
    pub const fn default_texel(self) -> [u8; 4] {
        match self {
            Self::Normal => [128, 128, 255, 255],
            _ => [255; 4],
        }
    }
}

/// One immutable texture class allocation, including its reserved default layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureArrayDescriptor {
    /// Semantics/transfer/format for this array.
    pub class: TextureClass,
    /// Shared base width.
    pub width: u32,
    /// Shared base height.
    pub height: u32,
    /// Nonzero layers, including multiplicative default layer zero.
    pub layers: u32,
    /// Complete authored/cooked mip count, at least one.
    pub mips: u32,
}

/// A prepared texture-class policy. Four arrays consume four sampled bindings,
/// irrespective of material count; no global unbounded texture heap is implied.
#[derive(Clone, Debug)]
pub struct TextureClasses {
    arrays: [TextureArrayDescriptor; 4],
}
impl TextureClasses {
    /// Check order, dimensions, mips and queried device limits before allocation.
    ///
    /// # Errors
    ///
    /// Returns an ABI error for an invalid class or insufficient device limits.
    pub fn new(
        arrays: [TextureArrayDescriptor; 4],
        limits: &wgpu::Limits,
    ) -> Result<Self, AbiError> {
        for (index, array) in arrays.iter().enumerate() {
            if array.class.index() != index
                || array.width == 0
                || array.height == 0
                || array.layers == 0
                || array.mips == 0
                || array.mips > 32 - array.width.max(array.height).leading_zeros()
                || array.width > limits.max_texture_dimension_2d
                || array.height > limits.max_texture_dimension_2d
                || array.layers > limits.max_texture_array_layers
            {
                return Err(AbiError::TextureClass);
            }
        }
        if limits.max_sampled_textures_per_shader_stage < 4
            || limits.max_bind_groups < 4
            || limits.max_storage_buffers_per_shader_stage < 2
        {
            return Err(AbiError::DeviceLimits);
        }
        Ok(Self { arrays })
    }
    /// Four 1×1 default-only arrays used by the synthetic diagnostic.
    ///
    /// # Errors
    ///
    /// Returns an ABI error if the device cannot support the default classes.
    pub fn defaults(limits: &wgpu::Limits) -> Result<Self, AbiError> {
        Self::new(
            [
                TextureClass::BaseColor,
                TextureClass::Normal,
                TextureClass::Orm,
                TextureClass::Emissive,
            ]
            .map(|class| TextureArrayDescriptor {
                class,
                width: 1,
                height: 1,
                layers: 1,
                mips: 1,
            }),
            limits,
        )
    }
    /// Fixed class order: base, normal, ORM, emissive.
    pub fn arrays(&self) -> &[TextureArrayDescriptor; 4] {
        &self.arrays
    }
    fn contains(&self, layers: [u32; 4]) -> bool {
        layers
            .iter()
            .zip(self.arrays)
            .all(|(layer, array)| *layer < array.layers)
    }
}

/// Native metallic-roughness factors. Values remain linear; display conversion
/// belongs to V4. Factor validation does not certify a BRDF implementation.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialRecord {
    /// RGBA factors, finite in [0,1].
    pub base_rgba: [f32; 4],
    /// Nonnegative RGB radiance and alpha cutoff in [0,1].
    pub emissive_cutoff: [f32; 4],
    /// Occlusion strength, roughness, metallic and normal scale.
    pub surface: [f32; 4],
    /// Versioned semantic flags/IDs; synthetic profile requires all zero.
    pub flags: [u32; 4],
    /// Base, normal, ORM and emissive layer, all validated against their class.
    pub texture_layers: [u32; 4],
}
impl Default for MaterialRecord {
    fn default() -> Self {
        Self {
            base_rgba: [1.0; 4],
            emissive_cutoff: [0.0; 4],
            surface: [1.0, 1.0, 0.0, 1.0],
            flags: [0; 4],
            texture_layers: [0; 4],
        }
    }
}
impl MaterialRecord {
    /// Validate semantics and encode the exact shared 80-byte record.
    ///
    /// # Errors
    ///
    /// Returns an ABI error if a factor or texture layer is invalid.
    pub fn pack(&self, textures: &TextureClasses) -> Result<[u8; MATERIAL_BYTES], AbiError> {
        if !self.base_rgba.iter().all(|x| unit(*x))
            || !self.emissive_cutoff[..3]
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0)
            || !unit(self.emissive_cutoff[3])
            || !self.surface[..3].iter().all(|x| unit(*x))
            || !self.surface[3].is_finite()
            || self.surface[3] < 0.0
        {
            return Err(AbiError::InvalidFactor);
        }
        if !textures.contains(self.texture_layers) {
            return Err(AbiError::TextureLayer);
        }
        let mut bytes = [0; MATERIAL_BYTES];
        floats(&mut bytes, 0, &self.base_rgba);
        floats(&mut bytes, 16, &self.emissive_cutoff);
        floats(&mut bytes, 32, &self.surface);
        uints(&mut bytes, 48, &self.flags);
        uints(&mut bytes, 64, &self.texture_layers);
        Ok(bytes)
    }
}

/// Static/instanced geometry record. Prior transforms are supplied by the actual
/// history sample owner; copying the current matrix does not certify history.
#[derive(Clone, Debug, PartialEq)]
pub struct InstanceRecord {
    /// Column-major current object-to-view-origin-relative-world matrix.
    pub current: [[f32; 4]; 4],
    /// Column-major matrix for the explicitly retained history sample.
    pub previous: [[f32; 4]; 4],
    /// Three padded inverse-transpose normal matrix columns.
    pub normal: [[f32; 4]; 3],
    /// Stable instance index, material row, skin offset, source generation.
    pub ids: [u32; 4],
}
impl InstanceRecord {
    /// Identity instance referencing an existing material row. Source generation
    /// is diagnostic revision 1; history validity remains a separate contract.
    pub fn identity(instance: u32, material: u32) -> Self {
        let matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Self {
            current: matrix,
            previous: matrix,
            normal: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
            ],
            ids: [instance, material, 0, 1],
        }
    }
    /// Check finite transforms and indexed material bounds before packing.
    ///
    /// # Errors
    ///
    /// Returns an ABI error for a nonfinite transform or invalid material index.
    pub fn pack(&self, materials: u32) -> Result<[u8; INSTANCE_BYTES], AbiError> {
        if self.ids[1] >= materials {
            return Err(AbiError::MaterialIndex);
        }
        if !self
            .current
            .iter()
            .chain(&self.previous)
            .chain(&self.normal)
            .flatten()
            .all(|x| x.is_finite())
        {
            return Err(AbiError::InvalidTransform);
        }
        let mut bytes = [0; INSTANCE_BYTES];
        for (column, values) in self.current.iter().enumerate() {
            floats(&mut bytes, column * 16, values);
        }
        for (column, values) in self.previous.iter().enumerate() {
            floats(&mut bytes, 64 + column * 16, values);
        }
        for (column, values) in self.normal.iter().enumerate() {
            floats(&mut bytes, 128 + column * 16, values);
        }
        uints(&mut bytes, 176, &self.ids);
        Ok(bytes)
    }
}

/// Admission failure before GPU resources/readiness are published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiError {
    /// Invalid native finite factor domain.
    InvalidFactor,
    /// Nonfinite model or normal matrix.
    InvalidTransform,
    /// Material row outside the admitted table.
    MaterialIndex,
    /// Layer outside its typed texture class.
    TextureLayer,
    /// Class order, dimension, layer or mip policy violation.
    TextureClass,
    /// Insufficient queried binding/storage limits.
    DeviceLimits,
    /// Empty table or storage table exceeds queried binding/buffer limits.
    TableCapacity,
}

/// Validate storage-table sizes before allocation, including the queried binding
/// size limit. No truncation, implicit chunking or per-object uniform fallback.
///
/// # Errors
///
/// Returns an ABI error if either table cannot fit the device limits.
pub fn table_bytes(
    materials: usize,
    instances: usize,
    limits: &wgpu::Limits,
) -> Result<(u64, u64), AbiError> {
    let material_bytes = materials
        .checked_mul(MATERIAL_BYTES)
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(AbiError::TableCapacity)?;
    let instance_bytes = instances
        .checked_mul(INSTANCE_BYTES)
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(AbiError::TableCapacity)?;
    if materials == 0
        || instances == 0
        || materials > u32::MAX as usize
        || instances > u32::MAX as usize
        || [material_bytes, instance_bytes].iter().any(|size| {
            *size > limits.max_buffer_size || *size > limits.max_storage_buffer_binding_size
        })
    {
        return Err(AbiError::TableCapacity);
    }
    Ok((material_bytes, instance_bytes))
}
fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
fn floats(bytes: &mut [u8], offset: usize, values: &[f32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}
fn uints(bytes: &mut [u8], offset: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[offset + i * 4..offset + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
}
