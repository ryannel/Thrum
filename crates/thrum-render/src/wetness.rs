//! Optional scene-owned inputs to the forward wetness function.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const BINDING: u32 = 26;

/// Prepared field data and the WGSL that interprets it. The shader defines
/// `wet_material(position, normal, base, roughness, metalness)` and returns
/// the adjusted base and roughness in a `vec4<f32>`.
#[derive(Clone, Debug)]
pub struct WetnessProvider {
    source: &'static str,
    width: u32,
    static_rows: Arc<[[f32; 4]]>,
    state: Arc<Mutex<State>>,
}

#[derive(Debug)]
struct State {
    rows: Vec<[f32; 4]>,
    revision: u64,
    active: bool,
}

impl WetnessProvider {
    /// Create a field made of contiguous RGBA32F rows. The initial dynamic
    /// rows precede the static rows. Both slices must contain whole rows.
    #[must_use]
    pub fn new(
        source: &'static str,
        width: u32,
        dynamic_rows: Vec<[f32; 4]>,
        static_rows: Vec<[f32; 4]>,
    ) -> Option<Self> {
        let columns = width as usize;
        if width == 0
            || dynamic_rows.is_empty()
            || !dynamic_rows.len().is_multiple_of(columns)
            || !static_rows.len().is_multiple_of(columns)
            || u32::try_from((dynamic_rows.len() + static_rows.len()) / columns).is_err()
        {
            return None;
        }
        Some(Self {
            source,
            width,
            static_rows: static_rows.into(),
            state: Arc::new(Mutex::new(State {
                rows: dynamic_rows,
                revision: 0,
                active: true,
            })),
        })
    }

    /// Publish the latest tick's dynamic rows. Returns false for an invalid
    /// row count. Repeating the same values does not request another upload.
    ///
    /// # Panics
    /// Panics if the publication mutex is poisoned.
    pub fn publish(&self, rows: &[[f32; 4]]) -> bool {
        self.publish_state(rows, true)
    }

    /// Publish a tick and whether its field can change forward shading.
    /// A dry field draws with the original material pipeline.
    ///
    /// # Panics
    /// Panics if the publication mutex is poisoned.
    pub fn publish_state(&self, rows: &[[f32; 4]], active: bool) -> bool {
        let mut state = self.state.lock().expect("wetness state");
        if rows.len() != state.rows.len() {
            return false;
        }
        if rows != state.rows || active != state.active {
            state.rows.copy_from_slice(rows);
            state.active = active;
            state.revision = state.revision.wrapping_add(1);
        }
        true
    }

    pub(crate) fn source(&self) -> &'static str {
        self.source
    }
}

/// The field texture lent to passes that evaluate the same ground wetness.
#[derive(Clone, Debug)]
pub struct PreparedWetness {
    field: WetnessProvider,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    revision: u64,
    active: Arc<AtomicBool>,
}

impl PreparedWetness {
    /// Prepare the field for a device.
    /// # Errors
    /// Refuses a field beyond the device limits.
    ///
    /// # Panics
    /// Panics if the publication mutex is poisoned.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        field: &WetnessProvider,
    ) -> Result<Self, crate::abi::AbiError> {
        let state = field.state.lock().expect("wetness state");
        let size = wgpu::Extent3d {
            width: field.width,
            height: u32::try_from(
                (state.rows.len() + field.static_rows.len()) / field.width as usize,
            )
            .map_err(|_| crate::abi::AbiError::DeviceLimits)?,
            depth_or_array_layers: 1,
        };
        let limits = device.limits();
        if size.width.max(size.height) > limits.max_texture_dimension_2d
            || limits.max_bindings_per_bind_group <= BINDING
        {
            return Err(crate::abi::AbiError::DeviceLimits);
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("forward ground wetness"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        write_rows(queue, &texture, 0, &state.rows);
        write_rows(
            queue,
            &texture,
            u32::try_from(state.rows.len() / field.width as usize)
                .map_err(|_| crate::abi::AbiError::DeviceLimits)?,
            &field.static_rows,
        );
        Ok(Self {
            field: field.clone(),
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture,
            revision: state.revision,
            active: Arc::new(AtomicBool::new(state.active)),
        })
    }

    pub(crate) fn source(&self) -> &'static str {
        self.field.source()
    }

    /// Whether this field uses the given evaluator.
    #[must_use]
    pub fn uses(&self, evaluator: &'static str) -> bool {
        self.source() == evaluator
    }

    pub(crate) fn accepts(&self, field: &WetnessProvider) -> bool {
        Arc::ptr_eq(&self.field.state, &field.state)
    }

    /// Read the published tick's dynamic rows for a contribution's CPU admission.
    ///
    /// # Panics
    /// Panics if the publication mutex is poisoned.
    pub fn read_rows<T>(&self, read: impl FnOnce(&[[f32; 4]]) -> T) -> T {
        let state = self.field.state.lock().expect("wetness state");
        read(&state.rows)
    }

    /// Upload a newly published tick to this device.
    ///
    /// # Panics
    /// Panics if the publication mutex is poisoned.
    pub fn upload(&mut self, queue: &wgpu::Queue) {
        let state = self.field.state.lock().expect("wetness state");
        if self.revision != state.revision {
            write_rows(queue, &self.texture, 0, &state.rows);
            self.revision = state.revision;
            self.active.store(state.active, Ordering::Release);
        }
    }

    /// Whether the uploaded field changes forward shading.
    #[must_use]
    pub fn active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Layout entry for reading the field at the pass's chosen binding.
    #[must_use]
    pub fn layout(binding: u32) -> wgpu::BindGroupLayoutEntry {
        crate::binding::texture(
            binding,
            wgpu::ShaderStages::FRAGMENT,
            wgpu::TextureSampleType::Float { filterable: false },
            wgpu::TextureViewDimension::D2,
        )
    }

    /// Bind the same prepared field at the pass's chosen binding.
    #[must_use]
    pub fn entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }

    pub(crate) fn forward_layout() -> wgpu::BindGroupLayoutEntry {
        Self::layout(BINDING)
    }

    pub(crate) fn forward_entry(&self) -> wgpu::BindGroupEntry<'_> {
        self.entry(BINDING)
    }

    /// The field evaluator using a pass's chosen resource location.
    #[must_use]
    pub fn shader(&self, group: u32, binding: u32) -> String {
        field_source(self.field.source(), group, binding)
    }
}

fn write_rows(queue: &wgpu::Queue, texture: &wgpu::Texture, y: u32, rows: &[[f32; 4]]) {
    if rows.is_empty() {
        return;
    }
    let width = texture.width();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(rows),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 16),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width,
            height: u32::try_from(rows.len() / width as usize).expect("admitted atlas rows"),
            depth_or_array_layers: 1,
        },
    );
}

fn field_source(source: &str, group: u32, binding: u32) -> String {
    format!("@group({group}) @binding({binding}) var wetness_field: texture_2d<f32>;\n{source}")
}

pub(crate) fn compose(scene: String, source: Option<&str>) -> String {
    const MATERIAL: &str = "    let surface_base = material.base.rgb*base.rgb;\n    let metalness = material.surface.z*orm.b;\n    let roughness = material.surface.y*orm.g;";
    let Some(source) = source else {
        return scene;
    };
    let (before, after) = scene
        .split_once(MATERIAL)
        .expect("forward wetness material input");
    let field = field_source(source, 1, BINDING);
    format!(
        "{before}    let dry_base = material.base.rgb*base.rgb;
    let metalness = material.surface.z*orm.b;
    let dry_roughness = material.surface.y*orm.g;
    let wet = wet_material(input.world_position, normalize(input.normal),
        dry_base, dry_roughness, metalness);
    let surface_base = wet.rgb;
    let roughness = wet.a;{after}\n{field}"
    )
}
