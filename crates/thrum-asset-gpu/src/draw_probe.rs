//! Compile-time opt-in draw qualification controls. Ordinary builds omit this module.

/// Probe-only command selection, supplied explicitly before attaching the contribution.
#[derive(Clone, Debug)]
pub enum Probe {
    /// Independently derived indexed arguments, one list per placement.
    Reference(Vec<Vec<[u32; 5]>>),
    /// Omit one placed mesh in the forward pass of the real direct encoder.
    Omit(usize),
}

impl Probe {
    pub(crate) fn validate(&self, placements: usize) -> Result<(), String> {
        match self {
            Self::Reference(commands) => {
                if commands.len() != placements {
                    return Err("reference placement coverage mismatch".into());
                }
                for placement in commands {
                    let mut next = 0;
                    for &[count, instances, first, base, instance] in placement {
                        if count == 0
                            || instances != 1
                            || first != next
                            || base != 0
                            || instance != 0
                        {
                            return Err("reference must cover contiguous triangle ranges with base vertex zero and one instance".into());
                        }
                        next = next
                            .checked_add(count)
                            .ok_or("reference index range overflow")?;
                    }
                    if next == 0 {
                        return Err("empty reference placement".into());
                    }
                }
                Ok(())
            }
            Self::Omit(index) if *index < placements => Ok(()),
            Self::Omit(_) => Err("omitted placement is out of range".into()),
        }
    }

    pub(crate) fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        placement: usize,
        resident_count: u32,
    ) -> Option<Bound> {
        match self {
            Self::Omit(index) => (*index == placement).then_some(Bound::Omit),
            Self::Reference(all) => {
                let commands = &all[placement];
                let count: u32 = commands.iter().map(|args| args[0]).sum();
                assert_eq!(
                    count, resident_count,
                    "independently derived reference coverage differs from resident index range"
                );
                let bytes = bytemuck::cast_slice(commands);
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("opt-in cooked full-range reference"),
                    size: bytes.len() as u64,
                    usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                queue.write_buffer(&buffer, 0, bytes);
                Some(Bound::Reference(
                    buffer,
                    u32::try_from(commands.len()).expect("validated cooked meshlet count fits u32"),
                    commands.clone(),
                ))
            }
        }
    }
}

pub(crate) enum Bound {
    Reference(wgpu::Buffer, u32, Vec<[u32; 5]>),
    Omit,
}

impl Bound {
    pub(crate) fn indirect_buffer(&self) -> Option<&wgpu::Buffer> {
        match self {
            Self::Reference(buffer, _, _) => Some(buffer),
            Self::Omit => None,
        }
    }

    pub(crate) fn description(&self) -> String {
        match self {
            Self::Omit => "omitted forward placement".into(),
            Self::Reference(_, _, commands) => {
                format!("draw_indexed_indirect arguments {commands:?}")
            }
        }
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Self::Reference(buffer, count, _) = self {
            for index in 0..*count {
                pass.draw_indexed_indirect(buffer, u64::from(index) * 20);
            }
        }
    }
}

/// Derive complete reference coverage directly from validated cooked products.
/// Placement order is supplied independently by the probe's scene record.
/// # Errors
/// Refuses missing products, invalid mesh references or incomplete triangle coverage.
#[cfg(not(target_arch = "wasm32"))]
pub fn reference_commands(
    store: &std::path::Path,
    meshes: &[String],
    partitioned: bool,
) -> Result<Vec<Vec<[u32; 5]>>, String> {
    use thrum_asset::{AssetStore, MeshProduct, Profile};
    let store = AssetStore::new(store);
    let head = store
        .read_head()
        .map_err(|e| e.to_string())?
        .ok_or("missing reference store HEAD")?;
    let manifest = store.load_manifest(&head).map_err(|e| e.to_string())?;
    meshes
        .iter()
        .map(|name| {
            let entry = manifest
                .resolve(name, &[Profile::Portable])
                .ok_or_else(|| format!("missing reference mesh {name}"))?;
            let mesh = MeshProduct::open(store.map(&entry.product).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            mesh.validate_references().map_err(|e| e.to_string())?;
            let mut first: u32 = 0;
            let mut commands = Vec::new();
            for meshlet in mesh.meshlets() {
                let count = meshlet
                    .triangle_count
                    .checked_mul(3)
                    .ok_or("reference triangle overflow")?;
                commands.push([count, 1, first, 0, 0]);
                first = first.checked_add(count).ok_or("reference range overflow")?;
            }
            if first != mesh.meta().index_count
                || usize::try_from(first).map_err(|e| e.to_string())? != mesh.indices().len()
            {
                return Err(format!("incomplete reference coverage for {name}"));
            }
            if partitioned {
                Ok(commands)
            } else {
                Ok(vec![[first, 1, 0, 0, 0]])
            }
        })
        .collect()
}
