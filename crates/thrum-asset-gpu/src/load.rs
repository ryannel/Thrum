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
    AssetError, ContentHash, Head, Ktx2, ManifestEntry, MeshProduct, ProductBytes, Profile,
    RemoteGeneration,
};
#[cfg(not(target_arch = "wasm32"))]
use thrum_asset::{AssetStore, HeadWatcher, Manifest};

#[cfg(not(target_arch = "wasm32"))]
use crate::CookedSource;
#[cfg(not(target_arch = "wasm32"))]
use crate::staging::Progress;
use crate::staging::{Bytes, Filler, StagedUpload, StagingPlan, StagingRing};
use crate::{CookedConfig, CookedStats, wgpu_format};

/// Product bytes from a mapped store or a fetched generation.
#[derive(Clone, Copy)]
enum Source<'a> {
    #[cfg(not(target_arch = "wasm32"))]
    Store(&'a AssetStore, &'a Manifest),
    Remote(&'a RemoteGeneration),
}

impl<'a> Source<'a> {
    fn portable_hash(self, name: &str) -> Option<ContentHash> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Store(_, manifest) => manifest.resolve(name, &[Profile::Portable]),
            Self::Remote(generation) => generation.manifest().resolve(name, &[Profile::Portable]),
        }
        .map(|entry| entry.product)
    }
    fn resolve(self, name: &str, preferences: &[Profile]) -> Option<ManifestEntry<'a>> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Store(_, manifest) => manifest.resolve(name, preferences),
            Self::Remote(generation) => generation.resolve(name, preferences),
        }
    }

    fn map(self, hash: &ContentHash) -> Result<ProductBytes, AssetError> {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Store(store, _) => store.map(hash),
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
    pub(crate) ranges: Option<wgpu::Buffer>,
    /// Per meshlet culling bounds.
    pub(crate) bounds: Option<wgpu::Buffer>,
    pub(crate) meshlet_count: u32,
    pub(crate) index_count: u32,
    pub(crate) caster_box: Option<(glam::Vec3, glam::Vec3)>,
    /// Points whose lowest one, once placed, is the mesh's lowest point: every vertex of a
    /// mesh of at most [`REST_VERTEX_LIMIT`] vertices, else the corners of `caster_box`.
    pub(crate) rest_points: Arc<[glam::Vec3]>,
}

/// Meshes up to this many vertices keep a CPU copy of their positions (at most 768 KiB) so a
/// resting placement touches the ground exactly; larger ones rest on their bounding box, which
/// can leave a turned mesh slightly above the ground.
pub(crate) const REST_VERTEX_LIMIT: usize = 65_536;

/// A sampled 2D texture or 3D volume.
pub(crate) struct GpuTexture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

/// Every product one publication draws.
pub(crate) struct ResidentSet {
    pub(crate) leased: Vec<(ContentHash, ContentHash, Profile, Arc<GpuTexture>)>,
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
pub(crate) struct Confirmed {
    meshes: HashMap<ContentHash, Arc<GpuMesh>>,
    textures: HashMap<ContentHash, Arc<GpuTexture>>,
}

pub(crate) struct Loader {
    pub(crate) indirect: bool,
    pub(crate) device: wgpu::Device,
    /// This device generation's bounded staging.
    pub(crate) ring: StagingRing,
    pub(crate) config: CookedConfig,
    pub(crate) texture_profiles: Vec<Profile>,
    pub(crate) volume_profiles: Vec<Profile>,
    pub(crate) stats: Arc<Mutex<CookedStats>>,
}

/// Products a set needs, before upload.
struct Opened {
    leased: Vec<(ContentHash, ContentHash, Profile, Option<Ktx2>)>,
    meshes: Vec<(ContentHash, Option<MeshProduct>, ContentHash, Option<Ktx2>)>,
    volume: Option<(ContentHash, Option<Ktx2>)>,
}

/// Newest-value hand-over from the loader to the render thread.
#[derive(Default)]
pub(crate) struct Mailbox(Mutex<Option<Result<PendingSet, ()>>>);

impl Mailbox {
    /// Replace any untaken set; the superseded one (staging and destinations) drops here, on the
    /// loader thread, outside the lock.
    fn post(&self, set: PendingSet) {
        let superseded = self
            .0
            .lock()
            .map(|mut slot| slot.replace(Ok(set)))
            .unwrap_or_default();
        drop(superseded);
    }

    /// Take the newest set without blocking; a contended slot is tried again next frame.
    pub(crate) fn take(&self) -> Option<Result<PendingSet, ()>> {
        self.0.try_lock().ok().and_then(|mut slot| slot.take())
    }

    fn refuse(&self) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(Err(()));
        }
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
        // The newest posted upload still being filled. HEAD is polled between bounded fill
        // slices, so a newer publication supersedes it instead of waiting for it to finish.
        let mut filling: Option<Filler> = None;
        while !stop.load(Ordering::Relaxed) {
            while let Ok(set) = resident.try_recv() {
                confirmed = Self::confirm(&set);
            }
            match watcher.poll() {
                Ok(Some(head)) => {
                    let seen = Instant::now();
                    let store = watcher.store();
                    let prepared = store_manifest(store, &head).and_then(|manifest| {
                        self.prepare(Source::Store(store, &manifest), &head, &confirmed)
                    });
                    match prepared {
                        Ok((set, upload, filler)) => {
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
                            filling = Some(filler);
                        }
                        Err(error) => {
                            mailbox.refuse();
                            filling = None;
                            self.fail(stop, &format!("generation {}: {error}", head.generation));
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => self.fail(stop, &error.to_string()),
            }
            // Fill the bounded staging as the render thread frees it, for at most one poll
            // interval before looking at HEAD again.
            let Some(filler) = filling.as_mut() else {
                std::thread::park_timeout(self.config.poll_interval);
                continue;
            };
            match filler.fill_until(stop, Instant::now() + self.config.poll_interval) {
                Progress::Waiting => {}
                Progress::Failed(reason) => {
                    self.fail(stop, &reason);
                    filling = None;
                }
                Progress::Done | Progress::Stopped => filling = None,
            }
        }
    }

    /// Prepare a fetched generation on this thread and post it, as the loader thread would.
    /// Products of `confirmed` (the drawn set) are reused rather than staged again. The caller
    /// fills the returned staging without waiting, as chunks free up.
    pub(crate) fn prepare_remote(
        &self,
        generation: &RemoteGeneration,
        confirmed: &Confirmed,
        mailbox: &Mailbox,
        stop: &AtomicBool,
    ) -> Option<Filler> {
        let head = generation.head();
        let seen = Instant::now();
        match self.prepare(Source::Remote(generation), &head, confirmed) {
            Ok((set, upload, filler)) => {
                report(&self.stats, stop, |stats| {
                    stats.staged = Some(head.generation);
                });
                mailbox.post(PendingSet {
                    set,
                    upload,
                    head_seen: seen,
                    staged: Instant::now(),
                });
                Some(filler)
            }
            Err(error) => {
                mailbox.refuse();
                self.fail(stop, &format!("generation {}: {error}", head.generation));
                None
            }
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
            stats.failures += 1;
        });
    }

    /// Keep exactly the resources the newest resident set uses; older ones retire when the
    /// render thread drops its last reference after their final submission.
    pub(crate) fn confirm(set: &ResidentSet) -> Confirmed {
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
        for (hash, _, _, texture) in &set.leased {
            next.textures.insert(*hash, Arc::clone(texture));
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
        confirmed: &Confirmed,
    ) -> Result<(Opened, Option<Profile>, Option<Profile>), AssetError> {
        let missing = || AssetError::Malformed("asset unavailable in the requested profiles");
        let mut opened = Opened {
            leased: Vec::new(),
            meshes: Vec::new(),
            volume: None,
        };
        let mut texture_profile = None;
        for placement in &self.config.meshes {
            let mesh = source
                .resolve(&placement.mesh, &[Profile::Portable])
                .ok_or_else(missing)?
                .product;
            let texture = source
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
            let entry = source
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
        for request in &self.config.leased {
            let volume = match request.dimension {
                wgpu::TextureDimension::D2 => false,
                wgpu::TextureDimension::D3 => true,
                wgpu::TextureDimension::D1 => {
                    return Err(AssetError::UnsupportedTexture("leases require D2 or D3"));
                }
            };
            let profiles = if volume {
                &self.volume_profiles
            } else {
                &self.texture_profiles
            };
            let entry = source.resolve(&request.name, profiles).ok_or_else(|| {
                AssetError::Io(std::io::Error::other(format!(
                    "lease {}: unavailable in requested profiles",
                    request.name
                )))
            })?;
            let product = if confirmed.textures.contains_key(&entry.product) {
                None
            } else {
                Some(
                    self.open_texture(source, &entry.product, volume)
                        .map_err(|error| {
                            AssetError::Io(std::io::Error::other(format!(
                                "lease {}: {error}",
                                request.name
                            )))
                        })?,
                )
            };
            let portable_hash = source.portable_hash(&request.name).ok_or_else(|| {
                AssetError::Io(std::io::Error::other(format!(
                    "lease {}: portable product unavailable",
                    request.name
                )))
            })?;
            opened
                .leased
                .push((entry.product, portable_hash, entry.profile, product));
        }
        Ok((opened, texture_profile, volume_profile))
    }

    fn prepare(
        &self,
        source: Source<'_>,
        head: &Head,
        confirmed: &Confirmed,
    ) -> Result<(ResidentSet, StagedUpload, Filler), String> {
        let (opened, texture_profile, volume_profile) = self
            .open(source, confirmed)
            .map_err(|error| error.to_string())?;
        let mut plan = StagingPlan::default();
        let mut set = ResidentSet {
            leased: Vec::new(),
            generation: head.generation,
            meshes: Vec::new(),
            volume: None,
            texture_profile,
            volume_profile,
            products: Vec::new(),
        };
        for (mesh_hash, mesh, texture_hash, texture) in opened.meshes {
            let gpu_mesh = match mesh {
                Some(mesh) => Arc::new(self.stage_mesh(&mut plan, &Arc::new(mesh))),
                None => Arc::clone(confirmed.meshes.get(&mesh_hash).ok_or("lost mesh")?),
            };
            let gpu_texture = match texture {
                Some(texture) => Arc::new(self.stage_texture(&mut plan, Arc::new(texture))),
                None => Arc::clone(
                    confirmed
                        .textures
                        .get(&texture_hash)
                        .ok_or("lost texture")?,
                ),
            };
            set.products.extend([mesh_hash, texture_hash]);
            set.meshes.push((gpu_mesh, gpu_texture));
        }
        if let Some((hash, product)) = opened.volume {
            set.volume = Some(match product {
                Some(volume) => Arc::new(self.stage_texture(&mut plan, Arc::new(volume))),
                None => Arc::clone(confirmed.textures.get(&hash).ok_or("lost volume")?),
            });
            set.products.push(hash);
        }
        // Reuse placement textures and repeated lease names within this publication too.
        let mut textures = Self::confirm(&set).textures;
        for ((hash, portable_hash, profile, product), request) in
            opened.leased.into_iter().zip(&self.config.leased)
        {
            let texture = if let Some(texture) = textures
                .get(&hash)
                .or_else(|| confirmed.textures.get(&hash))
            {
                Arc::clone(texture)
            } else {
                Arc::new(
                    self.stage_texture(&mut plan, Arc::new(product.ok_or("lost leased texture")?)),
                )
            };
            if texture.texture.dimension() != request.dimension {
                return Err(format!(
                    "lease {}: texture dimension does not match its use",
                    request.name
                ));
            }
            textures.insert(hash, Arc::clone(&texture));
            set.leased.push((hash, portable_hash, profile, texture));
        }
        let (upload, filler) = plan.finish(&self.ring)?;
        Ok((set, upload, filler))
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

    fn stage_mesh(&self, plan: &mut StagingPlan, mesh: &Arc<MeshProduct>) -> GpuMesh {
        let tables = Arc::new(MeshletTables::new(mesh));
        let part = |part| -> Arc<dyn Bytes> {
            Arc::new(MeshPart {
                mesh: Arc::clone(mesh),
                tables: Arc::clone(&tables),
                part,
            })
        };
        let (positions, attributes, indices, ranges, bounds) = (
            part(Part::Positions),
            part(Part::Attributes),
            part(Part::Indices),
            part(Part::Ranges),
            part(Part::Bounds),
        );
        let gpu = GpuMesh {
            positions: self.buffer(
                "cooked positions",
                positions.bytes().len(),
                wgpu::BufferUsages::VERTEX,
            ),
            attributes: self.buffer(
                "cooked attributes",
                attributes.bytes().len(),
                wgpu::BufferUsages::VERTEX,
            ),
            indices: self.buffer(
                "cooked meshlet indices",
                indices.bytes().len(),
                wgpu::BufferUsages::INDEX,
            ),
            ranges: self.indirect.then(|| {
                self.buffer(
                    "cooked meshlet ranges",
                    ranges.bytes().len(),
                    wgpu::BufferUsages::STORAGE,
                )
            }),
            bounds: self.indirect.then(|| {
                self.buffer(
                    "cooked meshlet bounds",
                    bounds.bytes().len(),
                    wgpu::BufferUsages::STORAGE,
                )
            }),
            meshlet_count: u32::try_from(mesh.meshlets().len()).unwrap_or(u32::MAX),
            index_count: u32::try_from(tables.indices.len()).unwrap_or(0),
            caster_box: caster_box(mesh.positions()),
            rest_points: rest_points(mesh.positions()),
        };
        plan.buffer(positions, gpu.positions.clone());
        plan.buffer(attributes, gpu.attributes.clone());
        plan.buffer(indices, gpu.indices.clone());
        if let Some(buffer) = &gpu.ranges {
            plan.buffer(ranges, buffer.clone());
        }
        if let Some(buffer) = &gpu.bounds {
            plan.buffer(bounds, buffer.clone());
        }
        gpu
    }

    fn stage_texture(&self, plan: &mut StagingPlan, product: Arc<Ktx2>) -> GpuTexture {
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
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        GpuTexture { texture, view }
    }
}

fn rest_points(positions: &[[f32; 3]]) -> Arc<[glam::Vec3]> {
    if positions.len() <= REST_VERTEX_LIMIT {
        let points: Arc<[glam::Vec3]> = positions
            .iter()
            .map(|&p| glam::Vec3::from_array(p))
            .collect();
        if points.iter().all(|point| point.is_finite()) {
            return points;
        }
    }
    caster_box(positions).map_or_else(
        || Arc::from([]),
        |(min, max)| {
            (0..8)
                .map(|corner| {
                    glam::Vec3::new(
                        if corner & 1 == 0 { min.x } else { max.x },
                        if corner & 2 == 0 { min.y } else { max.y },
                        if corner & 4 == 0 { min.z } else { max.z },
                    )
                })
                .collect()
        },
    )
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

/// Which array of a mesh a staged buffer reads.
#[derive(Clone, Copy)]
enum Part {
    Positions,
    Attributes,
    Indices,
    Ranges,
    Bounds,
}

/// One staged array of a mesh: the product's own bytes, or its expanded meshlet tables.
struct MeshPart {
    mesh: Arc<MeshProduct>,
    tables: Arc<MeshletTables>,
    part: Part,
}

impl Bytes for MeshPart {
    fn bytes(&self) -> &[u8] {
        match self.part {
            Part::Positions => bytemuck::cast_slice(self.mesh.positions()),
            Part::Attributes => bytemuck::cast_slice(self.mesh.attributes()),
            Part::Indices => bytemuck::cast_slice(&self.tables.indices),
            Part::Ranges => bytemuck::cast_slice(&self.tables.ranges),
            Part::Bounds => bytemuck::cast_slice(self.mesh.meshlet_bounds()),
        }
    }
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
