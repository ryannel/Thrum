//! Background product loader: `HEAD` poll, map, validate, upload, hand off.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use thrum_asset::{
    AssetError, AssetStore, ContentHash, Head, HeadWatcher, Ktx2, MeshProduct, Profile,
};

use crate::upload::{GpuMesh, GpuTexture, upload_ktx2, upload_mesh};
use crate::{Records, ViewerConfig};

/// A complete resident set for one publication.
pub(crate) struct Prepared {
    pub(crate) generation: u64,
    pub(crate) head_seen: Instant,
    pub(crate) ready: Instant,
    pub(crate) uploaded_bytes: u64,
    pub(crate) mapped: bool,
    pub(crate) texture_profile: Profile,
    pub(crate) volume_profile: Profile,
    pub(crate) mesh: Arc<GpuMesh>,
    pub(crate) bind_group: wgpu::BindGroup,
}

pub(crate) struct Loader {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) texture_profiles: Vec<Profile>,
    pub(crate) volume_profiles: Vec<Profile>,
    pub(crate) config: ViewerConfig,
    pub(crate) records: Arc<Mutex<Records>>,
}

/// GPU resources by content hash; a product shared by consecutive publications uploads once.
#[derive(Default)]
struct Cache {
    meshes: HashMap<ContentHash, Arc<GpuMesh>>,
    textures: HashMap<ContentHash, Arc<GpuTexture>>,
}

impl Loader {
    pub(crate) fn run(self, sender: &Sender<Prepared>, stop: &AtomicBool) {
        let mut watcher = HeadWatcher::new(AssetStore::new(&self.config.store));
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("asset-loop linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let mut cache = Cache::default();
        while !stop.load(Ordering::Relaxed) {
            match watcher.poll() {
                Ok(Some(head)) => {
                    let seen = Instant::now();
                    match self.prepare(watcher.store(), &head, seen, &mut cache, &sampler) {
                        Ok(prepared) => {
                            if sender.send(prepared).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            self.fail(&format!("generation {}: {error}", head.generation));
                        }
                    }
                }
                Ok(None) => std::thread::sleep(self.config.poll_interval),
                Err(error) => {
                    self.fail(&error.to_string());
                    std::thread::sleep(self.config.poll_interval);
                }
            }
        }
    }

    fn fail(&self, reason: &str) {
        if let Ok(mut records) = self.records.lock() {
            records.errors.push(reason.to_owned());
        }
    }

    fn prepare(
        &self,
        store: &AssetStore,
        head: &Head,
        seen: Instant,
        cache: &mut Cache,
        sampler: &wgpu::Sampler,
    ) -> Result<Prepared, AssetError> {
        let manifest = store.load_manifest(head)?;
        let missing = |name: &str| {
            AssetError::Malformed(if name.is_empty() {
                "asset"
            } else {
                "asset missing from manifest"
            })
        };
        let mesh_entry = manifest
            .resolve(&self.config.mesh, &[Profile::Portable])
            .ok_or_else(|| missing(&self.config.mesh))?;
        let texture_entry = manifest
            .resolve(&self.config.texture, &self.texture_profiles)
            .ok_or_else(|| missing(&self.config.texture))?;
        let volume_entry = manifest
            .resolve(&self.config.volume, &self.volume_profiles)
            .ok_or_else(|| missing(&self.config.volume))?;
        let chunk = self.config.upload_chunk;
        let mut uploaded_bytes = 0;
        let mut mapped = true;
        let mesh = if let Some(mesh) = cache.meshes.get(&mesh_entry.product) {
            Arc::clone(mesh)
        } else {
            let bytes = store.map(&mesh_entry.product)?;
            mapped &= bytes.is_mapped();
            let product = MeshProduct::open(bytes)?;
            product.validate_references()?;
            let (gpu, bytes) = upload_mesh(&self.device, &self.queue, &product, chunk);
            uploaded_bytes += bytes;
            Arc::new(gpu)
        };
        let mut texture_for =
            |hash: ContentHash, volume: bool| -> Result<Arc<GpuTexture>, AssetError> {
                if let Some(texture) = cache.textures.get(&hash) {
                    return Ok(Arc::clone(texture));
                }
                let bytes = store.map(&hash)?;
                mapped &= bytes.is_mapped();
                let product = Ktx2::open(bytes)?;
                if product.is_3d() != volume {
                    return Err(AssetError::Malformed(
                        "texture dimension does not match its binding",
                    ));
                }
                // Refuse what this device cannot create rather than letting validation fail on upload.
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
                    .contains(crate::upload::wgpu_format(product.format()).required_features())
                {
                    return Err(AssetError::UnsupportedTexture(
                        "format not enabled on this device",
                    ));
                }
                let (gpu, bytes) = upload_ktx2(&self.device, &self.queue, &product, chunk);
                uploaded_bytes += bytes;
                Ok(Arc::new(gpu))
            };
        let texture = texture_for(texture_entry.product, false)?;
        let volume = texture_for(volume_entry.product, true)?;
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("asset-loop products"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&volume.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        // Keep only what the newest set uses; older resources retire when the render thread lets go.
        *cache = Cache {
            meshes: HashMap::from([(mesh_entry.product, Arc::clone(&mesh))]),
            textures: HashMap::from([
                (texture_entry.product, texture),
                (volume_entry.product, volume),
            ]),
        };
        Ok(Prepared {
            generation: head.generation,
            head_seen: seen,
            ready: Instant::now(),
            uploaded_bytes,
            mapped,
            texture_profile: texture_entry.profile,
            volume_profile: volume_entry.profile,
            mesh,
            bind_group,
        })
    }
}
