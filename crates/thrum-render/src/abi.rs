//! CPU/WGSL revision 2: column-major matrices, typed storage-buffer records and D2 arrays.

/// Shared material/instance ABI revision.
pub const ABI_REVISION: u32 = 2;
/// Packed material-table stride.
pub const MATERIAL_BYTES: usize = size_of::<MaterialRecord>();
/// Packed instance-table stride.
pub const INSTANCE_BYTES: usize = size_of::<InstanceRecord>();
/// Base colour, emissive, surface factors, semantic flags, texture-layer indices.
pub const MATERIAL_OFFSETS: [usize; 5] = [
    std::mem::offset_of!(MaterialRecord, base_rgba),
    std::mem::offset_of!(MaterialRecord, emissive_cutoff),
    std::mem::offset_of!(MaterialRecord, surface),
    std::mem::offset_of!(MaterialRecord, flags),
    std::mem::offset_of!(MaterialRecord, texture_layers),
];

/// The scene's actual material declaration, for consumers that reflect their upload ABI.
/// Returns `None` if the renderer no longer declares this public material record.
pub fn material_wgsl_declaration() -> Option<&'static str> {
    let source = crate::shaders::wgsl!("synthetic.wgsl");
    let start = source.find("struct MaterialRecord {")?;
    let end = start + source[start..].find('}')? + 1;
    source.get(start..end)
}
/// Current/previous model, padded normal columns and instance/material/skin/source IDs.
pub const INSTANCE_OFFSETS: [usize; 6] = {
    let normal = std::mem::offset_of!(InstanceRecord, normal);
    let column = size_of::<[f32; 4]>();
    [
        std::mem::offset_of!(InstanceRecord, current),
        std::mem::offset_of!(InstanceRecord, previous),
        normal,
        normal + column,
        normal + 2 * column,
        std::mem::offset_of!(InstanceRecord, ids),
    ]
};

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
#[repr(C)]
#[derive(Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Copy)]
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
        Ok(bytemuck::cast(*self))
    }
}

/// Static/instanced geometry record. Prior transforms are supplied by the actual
/// history sample owner; copying the current matrix does not certify history.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Copy)]
pub struct InstanceRecord {
    /// Column-major current object-to-view-origin-relative-world matrix.
    pub current: [[f32; 4]; 4],
    /// Column-major previous simulation endpoint for presentation interpolation.
    /// Submitted motion history is retained separately by the renderer.
    pub previous: [[f32; 4]; 4],
    /// Three padded inverse-transpose normal matrix columns.
    pub normal: [[f32; 4]; 3],
    /// Logical GPU slot (never a StableId value), material row, skin offset,
    /// source generation. The renderer validates the slot on packing.
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

    /// Logical GPU slot (`ids` lane 0). Never a `StableId` value.
    #[must_use]
    pub fn logical_slot(&self) -> u32 {
        self.ids[0]
    }

    /// Set the logical GPU slot (`ids` lane 0).
    pub fn set_logical_slot(&mut self, slot: u32) {
        self.ids[0] = slot;
    }

    /// Material row (`ids` lane 1).
    #[must_use]
    pub fn material(&self) -> u32 {
        self.ids[1]
    }

    /// Set the material row (`ids` lane 1).
    pub fn set_material(&mut self, material: u32) {
        self.ids[1] = material;
    }

    /// Skin offset (`ids` lane 2); nonzero is not supported by the synthetic path.
    #[must_use]
    pub fn skin_offset(&self) -> u32 {
        self.ids[2]
    }

    /// Source generation (`ids` lane 3). Zero marks the row inactive.
    #[must_use]
    pub fn generation(&self) -> u32 {
        self.ids[3]
    }

    /// Set the source generation (`ids` lane 3); zero deactivates the row.
    pub fn set_generation(&mut self, generation: u32) {
        self.ids[3] = generation;
    }

    /// Whether the row is drawn: its source generation (`ids` lane 3) is nonzero.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.ids[3] != 0
    }

    /// Hide the row by zeroing its source generation (`ids` lane 3).
    pub fn deactivate(&mut self) {
        self.ids[3] = 0;
    }
    /// Check finite transforms and indexed material bounds before packing.
    ///
    /// # Errors
    ///
    /// Returns an ABI error for a nonfinite transform or invalid material index.
    pub fn pack(&self, materials: u32) -> Result<[u8; INSTANCE_BYTES], AbiError> {
        self.validate(materials)?;
        Ok(bytemuck::cast(*self))
    }

    pub(crate) fn validate(&self, materials: u32) -> Result<(), AbiError> {
        if self.material() >= materials {
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
        Ok(())
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
