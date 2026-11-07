//! Loader thread: watch `HEAD`, map and validate products, create destinations, stage bytes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use thrum_asset::{
    AssetError, ContentHash, Head, Ktx2, Manifest, MeshProduct, ProductBytes, Profile,
    RemoteGeneration,
};
#[cfg(not(target_arch = "wasm32"))]
use thrum_asset::{AssetStore, HeadWatcher};

#[cfg(not(target_arch = "wasm32"))]
use crate::CookedSource;
use crate::staging::{StagedUpload, StagingPlan};
use crate::{CookedConfig, CookedStats, wgpu_format};

/// Product bytes from a mapped store or a fetched generation.
#[derive(Clone, Copy)]
enum Source<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    Store(&'a AssetStore),
    Remote(&'a RemoteGeneration),
}

impl Source<'_> {
    fn map(self, hash: &ContentHash) -> Result<ProductBytes, AssetError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Store(store) => store.map(hash),
            Self::Remote(generation) => generation
                .product(hash)
                .ok_or(AssetError::Malformed("product not fetched")),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn store_manifest(store: &AssetStore, head: &Head) -> Result<Manifest, String> {
    store.load_manifest(head).map_err(|error| error.to_string())
}

/// A mesh resident on the GPU, with one indexed draw per meshlet.
pub(crate) struct GpuMesh {
    pub(crate) positions: wgpu::Buffer,
    pub(crate) attributes: wgpu::Buffer,
    /// Meshlet triangles expanded to global vertex indices, meshlet after meshlet.
    pub(crate) indices: wgpu::Buffer,
    /// Per meshlet: first index and index count.
    pub(crate) ranges: wgpu::Buffer,
    /// Per meshlet culling bounds.
    pub(crate) bounds: wgpu::Buffer,
    pub(crate) meshlet_count: u32,
    pub(crate) index_count: u32,
    pub(crate) caster_box: Option<(glam::Vec3, glam::Vec3)>,
}

/// A sampled 2D texture or 3D volume.
pub(crate) struct GpuTexture {
    pub(crate) texture: wgpu::Texture,
}

/// Every product one publication draws.
pub(crate) struct ResidentSet {
    pub(crate) generation: u64,
    pub(crate) meshes: Vec<(Arc<GpuMesh>, Arc<GpuTexture>)>,
    pub(crate) volume: Option<Arc<GpuTexture>>,
    pub(crate) texture_profile: Option<Profile>,
    pub(crate) volume_profile: Option<Profile>,
    pub(crate) products: Vec<ContentHash>,
}

/// A set whose destinations exist but whose bytes are still in staging.
pub(crate) struct PendingSet {
    pub(crate) set: ResidentSet,
    pub(crate) upload: StagedUpload,
    pub(crate) head_seen: Instant,
    pub(crate) staged: Instant,
}

/// Resources by product hash, only for sets the render thread has confirmed resident.
#[derive(Default)]
struct Confirmed {
    meshes: HashMap<ContentHash, Arc<GpuMesh>>,
    textures: HashMap<ContentHash, Arc<GpuTexture>>,
}

pub(crate) struct Loader {
    pub(crate) device: wgpu::Device,
    pub(crate) config: CookedConfig,
    pub(crate) texture_profiles: Vec<Profile>,
    pub(crate) volume_profiles: Vec<Profile>,
    pub(crate) stats: Arc<Mutex<CookedStats>>,
}

/// Products a set needs, before upload.
struct Opened {
    meshes: Vec<(ContentHash, Option<MeshProduct>, ContentHash, Option<Ktx2>)>,
    volume: Option<(ContentHash, Option<Ktx2>)>,
}

/// Newest-value hand-over from the loader to the render thread.
#[derive(Default)]
pub(crate) struct Mailbox(Mutex<Option<PendingSet>>);

impl Mailbox {
    /// Replace any untaken set; the superseded one (staging and destinations) drops here, on the
    /// loader thread, outside the lock.
    fn post(&self, set: PendingSet) {
        let superseded = self
            .0
            .lock()
            .map(|mut slot| slot.replace(set))
            .unwrap_or_default();
        drop(superseded);
    }

    /// Take the newest set without blocking; a contended slot is tried again next frame.
    pub(crate) fn take(&self) -> Option<PendingSet> {
        self.0.try_lock().ok().and_then(|mut slot| slot.take())
    }
}

/// Apply a loader observation unless the loader was retired. Retirement ([`retire`]) sets `stop`
/// under the same lock, so a retired loader that finishes preparing after its replacement started
/// can never overwrite the replacement's observations.
fn report(stats: &Mutex<CookedStats>, stop: &AtomicBool, update: impl FnOnce(&mut CookedStats)) {
    if let Ok(mut stats) = stats.lock()
        && !stop.load(Ordering::Relaxed)
    {
        update(&mut stats);
    }
}

/// Retire a loader: after this returns it publishes no observation.
pub(crate) fn retire(stats: &Mutex<CookedStats>, stop: &AtomicBool) {
    let _guard = stats.lock();
    stop.store(true, Ordering::Relaxed);
}

impl Loader {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn run(
        self,
        mailbox: &Mailbox,
        resident: &Receiver<Arc<ResidentSet>>,
        stop: &AtomicBool,
    ) {
        let CookedSource::Store(store) = &self.config.source else {
            return;
        };
        let mut watcher = HeadWatcher::new(AssetStore::new(store));
        let mut confirmed = Confirmed::default();
        while !stop.load(Ordering::Relaxed) {
            while let Ok(set) = resident.try_recv() {
                confirmed = Self::confirm(&set);
            }
            match watcher.poll() {
                Ok(Some(head)) => {
                    let seen = Instant::now();
                    let store = watcher.store();
                    let prepared = store_manifest(store, &head).and_then(|manifest| {
                        self.prepare(Source::Store(store), &manifest, &head, &confirmed)
                    });
                    match prepared {
                        Ok((set, upload)) => {
                            report(&self.stats, stop, |stats| {
                                stats.staged = Some(head.generation);
                            });
                            let pending = PendingSet {
                                set,
                                upload,
                                head_seen: seen,
                                staged: Instant::now(),
                            };
                            mailbox.post(pending);
                        }
                        Err(error) => {
                            self.fail(stop, &format!("generation {}: {error}", head.generation));
                        }
                    }
                }
                Ok(None) => std::thread::park_timeout(self.config.poll_interval),
                Err(error) => {
                    self.fail(stop, &error.to_string());
                    std::thread::park_timeout(self.config.poll_interval);
                }
            }
        }
    }

    /// Prepare a fetched generation on this thread and post it, as the loader thread would.
    pub(crate) fn prepare_remote(
        &self,
        generation: &RemoteGeneration,
        mailbox: &Mailbox,
        stop: &AtomicBool,
    ) {
        let head = generation.head();
        let seen = Instant::now();
        match self.prepare(
            Source::Remote(generation),
            generation.manifest(),
            &head,
            &Confirmed::default(),
        ) {
            Ok((set, upload)) => {
                report(&self.stats, stop, |stats| {
                    stats.staged = Some(head.generation);
                });
                mailbox.post(PendingSet {
                    set,
                    upload,
                    head_seen: seen,
                    staged: Instant::now(),
                });
            }
            Err(error) => self.fail(stop, &format!("generation {}: {error}", head.generation)),
        }
    }

    /// Record a failure once: a repeating error (an unreadable HEAD polled every interval) is
    /// kept as one entry, and the list stays bounded.
    pub(crate) fn fail(&self, stop: &AtomicBool, reason: &str) {
        const MAX_ERRORS: usize = 64;
        report(&self.stats, stop, |stats| {
            if stats.errors.last().is_some_and(|last| last == reason) {
                return;
            }
            if stats.errors.len() == MAX_ERRORS {
                stats.errors.remove(0);
            }
            stats.errors.push(reason.to_owned());
        });
    }

    /// Keep exactly the resources the newest resident set uses; older ones retire when the
    /// render thread drops its last reference after their final submission.
    #[cfg(not(target_arch = "wasm32"))]
    fn confirm(set: &ResidentSet) -> Confirmed {
        let mut next = Confirmed::default();
        let mut products = set.products.iter();
        for (mesh, texture) in &set.meshes {
            if let (Some(mesh_hash), Some(texture_hash)) = (products.next(), products.next()) {
                next.meshes.insert(*mesh_hash, Arc::clone(mesh));
                next.textures.insert(*texture_hash, Arc::clone(texture));
            }
        }
        if let (Some(volume), Some(hash)) = (&set.volume, products.next()) {
            next.textures.insert(*hash, Arc::clone(volume));
        }
        next
    }

    fn open_texture(
        &self,
        source: Source<'_>,
        hash: &ContentHash,
        volume: bool,
    ) -> Result<Ktx2, AssetError> {
        let product = Ktx2::open(source.map(hash)?)?;
        if product.is_3d() != volume {
            return Err(AssetError::Malformed(
                "texture dimension does not match its use",
            ));
        }
        let base = product.levels()[0];
        let limits = self.device.limits();
        let limit = if volume {
            limits.max_texture_dimension_3d
        } else {
            limits.max_texture_dimension_2d
        };
        if base.width.max(base.height).max(base.depth) > limit {
            return Err(AssetError::UnsupportedTexture(
                "extent exceeds this device's texture limit",
            ));
        }
        if !self
            .device
            .features()
            .contains(wgpu_format(product.format()).required_features())
        {
            return Err(AssetError::UnsupportedTexture(
                "format not enabled on this device",
            ));
        }
        Ok(product)
    }

    fn open(
        &self,
        source: Source<'_>,
        manifest: &Manifest,
        confirmed: &Confirmed,
    ) -> Result<(Opened, Option<Profile>, Option<Profile>), AssetError> {
        let missing = || AssetError::Malformed("asset missing from manifest");
        let mut opened = Opened {
            meshes: Vec::new(),
            volume: None,
        };
        let mut texture_profile = None;
        for placement in &self.config.meshes {
            let mesh = manifest
                .resolve(&placement.mesh, &[Profile::Portable])
                .ok_or_else(missing)?
                .product;
            let texture = manifest
                .resolve(&placement.texture, &self.texture_profiles)
                .ok_or_else(missing)?;
            texture_profile = Some(texture.profile);
            let mesh_product = if confirmed.meshes.contains_key(&mesh) {
                None
            } else {
                let product = MeshProduct::open(source.map(&mesh)?)?;
                product.validate_references()?;
                Some(product)
            };
            let texture_product = if confirmed.textures.contains_key(&texture.product) {
                None
            } else {
                Some(self.open_texture(source, &texture.product, false)?)
            };
            opened
                .meshes
                .push((mesh, mesh_product, texture.product, texture_product));
        }
        let mut volume_profile = None;
        if let Some(placement) = &self.config.volume {
            let entry = manifest
                .resolve(&placement.name, &self.volume_profiles)
                .ok_or_else(missing)?;
            volume_profile = Some(entry.profile);
            let product = if confirmed.textures.contains_key(&entry.product) {
                None
            } else {
                Some(self.open_texture(source, &entry.product, true)?)
            };
            opened.volume = Some((entry.product, product));
        }
        Ok((opened, texture_profile, volume_profile))
    }

    fn prepare(
        &self,
        source: Source<'_>,
        manifest: &Manifest,
        head: &Head,
        confirmed: &Confirmed,
    ) -> Result<(ResidentSet, StagedUpload), String> {
        let (opened, texture_profile, volume_profile) = self
            .open(source, manifest, confirmed)
            .map_err(|error| error.to_string())?;
        // Expanded meshlet indices must outlive the plan that borrows them.
        let expanded: Vec<Option<MeshletTables>> = opened
            .meshes
            .iter()
            .map(|(_, mesh, _, _)| mesh.as_ref().map(MeshletTables::new))
            .collect();
        let mut plan = StagingPlan::default();
        let mut set = ResidentSet {
            generation: head.generation,
            meshes: Vec::new(),
            volume: None,
            texture_profile,
            volume_profile,
            products: Vec::new(),
        };
        for ((mesh_hash, mesh, texture_hash, texture), tables) in
            opened.meshes.iter().zip(&expanded)
        {
            let gpu_mesh = match (mesh, tables) {
                (Some(mesh), Some(tables)) => Arc::new(self.stage_mesh(&mut plan, mesh, tables)),
                _ => Arc::clone(confirmed.meshes.get(mesh_hash).ok_or("lost mesh")?),
            };
            let gpu_texture = match texture {
                Some(texture) => Arc::new(self.stage_texture(&mut plan, texture)),
                None => Arc::clone(confirmed.textures.get(texture_hash).ok_or("lost texture")?),
            };
            set.products.extend([*mesh_hash, *texture_hash]);
            set.meshes.push((gpu_mesh, gpu_texture));
        }
        if let Some((hash, product)) = &opened.volume {
            set.volume = Some(match product {
                Some(volume) => Arc::new(self.stage_texture(&mut plan, volume)),
                None => Arc::clone(confirmed.textures.get(hash).ok_or("lost volume")?),
            });
            set.products.push(*hash);
        }
        let upload = plan.finish(&self.device)?;
        Ok((set, upload))
    }

    fn buffer(&self, label: &str, size: usize, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (size as u64)
                .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT)
                .max(16),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn stage_mesh<'a>(
        &self,
        plan: &mut StagingPlan<'a>,
        mesh: &'a MeshProduct,
        tables: &'a MeshletTables,
    ) -> GpuMesh {
        let positions: &[u8] = bytemuck::cast_slice(mesh.positions());
        let attributes: &[u8] = bytemuck::cast_slice(mesh.attributes());
        let bounds: &[u8] = bytemuck::cast_slice(mesh.meshlet_bounds());
        let indices: &[u8] = bytemuck::cast_slice(&tables.indices);
        let ranges: &[u8] = bytemuck::cast_slice(&tables.ranges);
        let gpu = GpuMesh {
            positions: self.buffer(
                "cooked positions",
                positions.len(),
                wgpu::BufferUsages::VERTEX,
            ),
            attributes: self.buffer(
                "cooked attributes",
                attributes.len(),
                wgpu::BufferUsages::VERTEX,
            ),
            indices: self.buffer(
                "cooked meshlet indices",
                indices.len(),
                wgpu::BufferUsages::INDEX,
            ),
            ranges: self.buffer(
                "cooked meshlet ranges",
                ranges.len(),
                wgpu::BufferUsages::STORAGE,
            ),
            bounds: self.buffer(
                "cooked meshlet bounds",
                bounds.len(),
                wgpu::BufferUsages::STORAGE,
            ),
            meshlet_count: u32::try_from(mesh.meshlets().len()).unwrap_or(u32::MAX),
            index_count: u32::try_from(tables.indices.len()).unwrap_or(0),
            caster_box: caster_box(mesh.positions()),
        };
        plan.buffer(positions, gpu.positions.clone());
        plan.buffer(attributes, gpu.attributes.clone());
        plan.buffer(indices, gpu.indices.clone());
        plan.buffer(ranges, gpu.ranges.clone());
        plan.buffer(bounds, gpu.bounds.clone());
        gpu
    }

    fn stage_texture<'a>(&self, plan: &mut StagingPlan<'a>, product: &'a Ktx2) -> GpuTexture {
        let base = product.levels()[0];
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cooked texture"),
            size: wgpu::Extent3d {
                width: base.width,
                height: base.height,
                depth_or_array_layers: base.depth,
            },
            mip_level_count: u32::try_from(product.levels().len()).unwrap_or(1),
            sample_count: 1,
            dimension: if product.is_3d() {
                wgpu::TextureDimension::D3
            } else {
                wgpu::TextureDimension::D2
            },
            format: wgpu_format(product.format()),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        plan.texture(product, texture.clone());
        GpuTexture { texture }
    }
}

fn caster_box(positions: &[[f32; 3]]) -> Option<(glam::Vec3, glam::Vec3)> {
    let mut min = glam::Vec3::from_array(*positions.first()?);
    let mut max = min;
    for &position in positions {
        let position = glam::Vec3::from_array(position);
        if !position.is_finite() {
            return None;
        }
        min = min.min(position);
        max = max.max(position);
    }
    Some((min, max))
}

/// Meshlets as indexed draws: global indices per meshlet, and each meshlet's index range.
pub(crate) struct MeshletTables {
    pub(crate) indices: Vec<u32>,
    pub(crate) ranges: Vec<[u32; 2]>,
}

impl MeshletTables {
    pub(crate) fn new(mesh: &MeshProduct) -> Self {
        let vertices = mesh.meshlet_vertices();
        let triangles = mesh.meshlet_triangles();
        let mut indices = Vec::with_capacity(triangles.len());
        let mut ranges = Vec::with_capacity(mesh.meshlets().len());
        for meshlet in mesh.meshlets() {
            let first = u32::try_from(indices.len()).unwrap_or(u32::MAX);
            let start = meshlet.triangle_offset as usize;
            let local = &triangles[start..start + meshlet.triangle_count as usize * 3];
            // `validate_references` proved every local index names a meshlet vertex.
            indices.extend(
                local
                    .iter()
                    .map(|&index| vertices[meshlet.vertex_offset as usize + usize::from(index)]),
            );
            ranges.push([first, meshlet.triangle_count * 3]);
        }
        Self { indices, ranges }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retired_loader_publishes_no_observation() {
        let stats = Mutex::new(CookedStats::default());
        let stop = AtomicBool::new(false);
        report(&stats, &stop, |stats| stats.staged = Some(2));
        retire(&stats, &stop);
        // A retired loader finishing an older HEAD, or failing on its old device, after retirement.
        report(&stats, &stop, |stats| {
            stats.staged = Some(1);
            stats.errors.push("old device".into());
        });
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.staged, Some(2));
        assert_eq!(stats.errors, Vec::<String>::new());
    }
}
