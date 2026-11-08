//! The cook cycle: scan sources, plan assets, reuse or cook products, publish one manifest.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use rayon::prelude::*;
use thrum_asset::layout::{MANIFEST_MAGIC, ManifestMeta, ManifestRecord, manifest_tag};
use thrum_asset::{ContentHash, Head, Manifest, ProductKind, Profile};

use crate::cas::{Cas, Durability};
use crate::stat_cache::{ScanCounts, StatCache};
use crate::texture::Quality;
use crate::write::ContainerWriter;
use crate::{CookError, mesh, texture};

/// Version of every cooker's output; bump to invalidate all cached actions.
const COOKER_VERSION: u32 = 2;
/// Encoder libraries and meshlet limits that shape product bytes. Cached actions persist across
/// builds, so any change here must change every action key. Keep in step with Cargo.toml.
const ENCODERS: &str =
    "astcenc-rs=0.2.0 intel_tex_2=0.5.0 meshopt=0.7.0 image=0.25.10 gltf=1.4.1 meshlet=64/124/0.25";

/// What to cook and where to publish it.
#[derive(Clone, Debug)]
pub struct CookConfig {
    /// Source root (authoring files).
    pub source: PathBuf,
    /// Product store root.
    pub store: PathBuf,
    /// Profiles to cook textures and volumes for. Meshes are profile-independent.
    pub profiles: Vec<Profile>,
    /// Encoder effort.
    pub quality: Quality,
    /// Write durability.
    pub durability: Durability,
}

/// Outcome and timing of one cook cycle.
#[derive(Clone, Debug, Default)]
pub struct CookReport {
    /// Stat-cache counts.
    pub scan: ScanCounts,
    /// Time spent scanning sources.
    pub scan_time: Duration,
    /// Products produced by running a cooker.
    pub cooked: usize,
    /// Products reused from the action cache.
    pub reused: usize,
    /// Assets that failed, with the reason; their previous product (if any) stays published.
    pub failed: Vec<(String, String)>,
    /// Products a profile refused for its source, with the reason. The asset publishes without
    /// them, withdrawing any earlier product for that profile, so loaders select its portable
    /// product.
    pub refused: Vec<(String, Profile, String)>,
    /// Time spent cooking.
    pub cook_time: Duration,
    /// The new publication, when the manifest changed.
    pub published: Option<Head>,
    /// Time spent building and publishing the manifest.
    pub publish_time: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Product {
    hash: ContentHash,
    len: u64,
}

type Binding = (String, Profile);

/// One cook job: produce `profile`'s product for an asset from these inputs.
struct Job {
    name: String,
    kind: ProductKind,
    profile: Profile,
    key: ContentHash,
    source: PathBuf,
    /// Relative input paths and the hashes the key was built from.
    inputs: Vec<(PathBuf, ContentHash)>,
}

/// Incremental cooker owning one store.
pub struct Cooker {
    config: CookConfig,
    cas: Cas,
    stat: StatCache,
    actions: HashMap<ContentHash, Product>,
    /// Action keys a profile refused, with the reason, so unchanged sources are not re-decoded.
    refusals: HashMap<ContentHash, String>,
    gltf_buffers: HashMap<ContentHash, Vec<String>>,
    published: BTreeMap<Binding, (ProductKind, Product)>,
    /// Products whose bytes this process has verified against their names.
    verified: HashSet<ContentHash>,
    generation: u64,
}

impl Cooker {
    /// Open a store for cooking. The published closure is re-verified: a product whose bytes do
    /// not match its name (for example after power loss under [`Durability::Process`]) is removed
    /// so the next cycle recooks it.
    ///
    /// # Errors
    /// I/O failure opening the store, or another cooker already writes it.
    pub fn open(mut config: CookConfig) -> Result<Self, CookError> {
        // Every device must find a loadable product: the portable fallback is always cooked.
        if !config.profiles.contains(&Profile::Portable) {
            config.profiles.push(Profile::Portable);
        }
        let cas = Cas::open(&config.store, config.durability)?;
        std::fs::create_dir_all(config.store.join("actions"))?;
        let stat = StatCache::load(&config.store.join("stat-cache.bin"));
        let mut cooker = Self {
            config,
            cas,
            stat,
            actions: HashMap::new(),
            refusals: HashMap::new(),
            gltf_buffers: HashMap::new(),
            published: BTreeMap::new(),
            verified: HashSet::new(),
            generation: 0,
        };
        cooker.recover();
        Ok(cooker)
    }

    fn recover(&mut self) {
        let store = self.cas.store().clone();
        let head = match store.read_head() {
            Ok(Some(head)) => head,
            Ok(None) => return,
            Err(_) => {
                // A damaged HEAD must not restart generations below what readers have already
                // seen; continue after the newest manifest in the store.
                self.generation = newest_manifest_generation(&store);
                return;
            }
        };
        self.generation = head.generation;
        let Ok(manifest) = store.load_manifest(&head) else {
            return;
        };
        let entries: Vec<_> = manifest
            .entries()
            .map(|entry| {
                (
                    (entry.name.to_owned(), entry.profile),
                    (
                        entry.kind,
                        Product {
                            hash: entry.product,
                            len: entry.byte_len,
                        },
                    ),
                )
            })
            .collect();
        let damaged: Vec<ContentHash> = entries
            .par_iter()
            .filter(|(_, (_, product))| {
                store
                    .map(&product.hash)
                    .map_or(true, |bytes| product.hash.verify(&bytes).is_err())
            })
            .map(|(_, (_, product))| product.hash)
            .collect();
        for hash in &damaged {
            std::fs::remove_file(store.product_path(hash)).ok();
        }
        self.published = entries
            .into_iter()
            .filter(|(_, (_, product))| !damaged.contains(&product.hash))
            .collect();
        self.verified
            .extend(self.published.values().map(|(_, product)| product.hash));
    }

    /// Current publication generation (zero before the first publish).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Store this cooker writes.
    pub fn store(&self) -> &thrum_asset::AssetStore {
        self.cas.store()
    }

    /// Scan the whole source tree, then cook and publish.
    ///
    /// # Errors
    /// I/O failure scanning or publishing; per-asset cook failures are reported, not returned.
    pub fn cook_all(&mut self) -> Result<CookReport, CookError> {
        let start = Instant::now();
        let scan = self.stat.scan_all(&self.config.source)?;
        self.cycle(scan, start.elapsed())
    }

    /// Re-stat only `paths` (absolute, or relative to the source root), then cook and publish.
    ///
    /// # Errors
    /// As [`Cooker::cook_all`].
    pub fn cook_changed(&mut self, paths: &[PathBuf]) -> Result<CookReport, CookError> {
        let start = Instant::now();
        let relative: Vec<PathBuf> = paths
            .iter()
            .filter_map(|path| {
                if path.is_absolute() {
                    path.strip_prefix(&self.config.source)
                        .ok()
                        .map(Path::to_path_buf)
                } else {
                    Some(path.clone())
                }
            })
            .collect();
        let scan = self.stat.scan_paths(&self.config.source, &relative)?;
        self.cycle(scan, start.elapsed())
    }

    fn cycle(&mut self, scan: ScanCounts, scan_time: Duration) -> Result<CookReport, CookError> {
        let mut report = CookReport {
            scan,
            scan_time,
            ..CookReport::default()
        };
        let start = Instant::now();
        let (jobs, mut failed) = self.plan();
        let mut products = BTreeMap::new();
        let mut misses = Vec::new();
        let mut refused = BTreeMap::new();
        for job in jobs {
            if let Some(reason) = self.refusals.get(&job.key) {
                refused.insert((job.name, job.profile), reason.clone());
                continue;
            }
            match self.cached_action(&job.key) {
                Some(product) => {
                    report.reused += 1;
                    products.insert((job.name, job.profile), (job.kind, product));
                }
                None => misses.push(job),
            }
        }
        let outcomes: Vec<(Job, Result<Product, CookError>)> = misses
            .into_par_iter()
            .map(|job| {
                let outcome = self.run(&job);
                (job, outcome)
            })
            .collect();
        for (job, outcome) in outcomes {
            match outcome {
                Ok(product) => {
                    report.cooked += 1;
                    self.remember_action(job.key, product);
                    products.insert((job.name, job.profile), (job.kind, product));
                }
                Err(CookError::Refused(reason)) => {
                    self.refusals.insert(job.key, reason.clone());
                    refused.insert((job.name, job.profile), reason);
                }
                Err(error) => failed.push((job.name, error.to_string())),
            }
        }
        // Keep serving the last good products of any asset that failed to plan or cook, except
        // those its current source refuses.
        for (name, _) in &failed {
            for (binding, previous) in self
                .published
                .range((name.clone(), Profile::Portable)..)
                .take_while(|((n, _), _)| n == name)
            {
                if !refused.contains_key(binding) {
                    products.entry(binding.clone()).or_insert(*previous);
                }
            }
        }
        report.cook_time = start.elapsed();
        let start = Instant::now();
        if products != self.published {
            report.published = Some(self.publish(&products)?);
            self.published = products;
        }
        self.stat.save(&self.config.store.join("stat-cache.bin"))?;
        report.publish_time = start.elapsed();
        failed.sort();
        failed.dedup();
        report.failed = failed;
        report.refused = refused
            .into_iter()
            .map(|((name, profile), reason)| (name, profile, reason))
            .collect();
        Ok(report)
    }

    fn plan(&mut self) -> (Vec<Job>, Vec<(String, String)>) {
        let files = self.stat.files();
        let mut jobs = Vec::new();
        let mut failed = Vec::new();
        let mut sources: BTreeMap<String, Vec<&PathBuf>> = BTreeMap::new();
        for path in files.keys() {
            if let Some((name, _, _)) = classify(path) {
                sources.entry(name).or_default().push(path);
            }
        }
        for (path, hash) in &files {
            let Some((name, kind, linear)) = classify(path) else {
                continue;
            };
            if let Some(paths) = sources.get(&name).filter(|paths| paths.len() > 1) {
                let list: Vec<String> = paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();
                failed.push((
                    name,
                    format!(
                        "several sources map to one logical name: {}",
                        list.join(", ")
                    ),
                ));
                continue;
            }
            let mut inputs = vec![(path.clone(), *hash)];
            if kind == ProductKind::Mesh {
                match self.mesh_buffers(path, *hash) {
                    Ok(buffers) => {
                        let mut missing = None;
                        for buffer in buffers {
                            match files.get(&buffer) {
                                Some(buffer_hash) => inputs.push((buffer, *buffer_hash)),
                                None => missing = Some(buffer),
                            }
                        }
                        if let Some(buffer) = missing {
                            failed
                                .push((name, format!("missing glTF buffer {}", buffer.display())));
                            continue;
                        }
                    }
                    Err(error) => {
                        failed.push((name, error.to_string()));
                        continue;
                    }
                }
            }
            let profiles = if kind == ProductKind::Mesh {
                vec![Profile::Portable]
            } else {
                self.config.profiles.clone()
            };
            for profile in profiles {
                let key = action_key(kind, profile, self.config.quality, linear, &inputs);
                jobs.push(Job {
                    name: name.clone(),
                    kind,
                    profile,
                    key,
                    source: self.config.source.join(path),
                    inputs: inputs.clone(),
                });
            }
        }
        (jobs, failed)
    }

    fn mesh_buffers(&mut self, path: &Path, hash: ContentHash) -> Result<Vec<PathBuf>, CookError> {
        if !self.gltf_buffers.contains_key(&hash) {
            let is_glb = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("glb"));
            let buffers = if is_glb {
                Vec::new()
            } else {
                mesh::external_buffers(&std::fs::read(self.config.source.join(path))?)?
            };
            self.gltf_buffers.insert(hash, buffers);
        }
        let directory = path.parent().unwrap_or(Path::new(""));
        let mut resolved = Vec::new();
        for uri in &self.gltf_buffers[&hash] {
            let relative = directory.join(uri);
            if relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            {
                return Err(CookError::Source(format!(
                    "glTF buffer outside the source tree: {uri}"
                )));
            }
            resolved.push(relative);
        }
        Ok(resolved)
    }

    fn cached_action(&mut self, key: &ContentHash) -> Option<Product> {
        let product = if let Some(product) = self.actions.get(key) {
            *product
        } else {
            let bytes = std::fs::read(self.config.store.join("actions").join(key.to_hex())).ok()?;
            let hash = ContentHash::from_bytes(bytes.get(..32)?.try_into().ok()?);
            let len = u64::from_le_bytes(bytes.get(32..40)?.try_into().ok()?);
            Product { hash, len }
        };
        if !self.verified.contains(&product.hash) {
            // A record can outlive damage to its product (power loss under process durability);
            // check the bytes once per process before trusting the record.
            let intact = self
                .cas
                .store()
                .map(&product.hash)
                .is_ok_and(|bytes| product.hash.verify(&bytes).is_ok());
            if !intact {
                std::fs::remove_file(self.cas.store().product_path(&product.hash)).ok();
                std::fs::remove_file(self.config.store.join("actions").join(key.to_hex())).ok();
                self.actions.remove(key);
                return None;
            }
            self.verified.insert(product.hash);
        }
        self.actions.insert(*key, product);
        Some(product)
    }

    fn remember_action(&mut self, key: ContentHash, product: Product) {
        self.actions.insert(key, product);
        self.verified.insert(product.hash);
        let mut record = product.hash.as_bytes().to_vec();
        record.extend_from_slice(&product.len.to_le_bytes());
        // The action cache is an accelerator: failing to persist it only costs a later recook.
        std::fs::write(self.config.store.join("actions").join(key.to_hex()), record).ok();
    }

    fn run(&self, job: &Job) -> Result<Product, CookError> {
        // The action key names the hashed input bytes; a product cooked from anything else would
        // poison the persistent action cache. Single-file sources are cooked from bytes checked
        // against their hash; multi-file meshes are re-verified after the importer reads them.
        let changed = || {
            CookError::Source("source changed during cook; it is retried on the next change".into())
        };
        let read_verified = || -> Result<Vec<u8>, CookError> {
            let bytes = std::fs::read(&job.source)?;
            if ContentHash::of(&bytes) == job.inputs[0].1 {
                Ok(bytes)
            } else {
                Err(changed())
            }
        };
        let bytes = match job.kind {
            ProductKind::Mesh => {
                let bytes = mesh::cook(&job.source)?;
                for (path, hash) in &job.inputs {
                    if ContentHash::of(&std::fs::read(self.config.source.join(path))?) != *hash {
                        return Err(changed());
                    }
                }
                bytes
            }
            ProductKind::Texture => {
                let linear = classify(&job.source).is_some_and(|(_, _, linear)| linear);
                texture::cook_texture(&read_verified()?, linear, job.profile, self.config.quality)?
            }
            ProductKind::Volume => texture::cook_volume(&read_verified()?, job.profile)?,
        };
        validate(job.kind, &bytes)?;
        let hash = self.cas.put(&bytes)?;
        Ok(Product {
            hash,
            len: bytes.len() as u64,
        })
    }

    fn publish(
        &mut self,
        products: &BTreeMap<Binding, (ProductKind, Product)>,
    ) -> Result<Head, CookError> {
        let generation = self.generation + 1;
        let bytes = encode_manifest(generation, products)?;
        // Prove the runtime accepts it before anyone can select it.
        Manifest::parse(thrum_asset::ProductBytes::copy_from(&bytes))?;
        let manifest = self.cas.put(&bytes)?;
        let head = Head {
            epoch: self.cas.epoch(),
            generation,
            manifest,
        };
        self.cas.publish(&head)?;
        self.generation = generation;
        Ok(head)
    }
}

/// Logical name, kind and linear flag for a source path, or `None` if it is not an asset root.
fn classify(path: &Path) -> Option<(String, ProductKind, bool)> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let kind = match extension.as_str() {
        "gltf" | "glb" => ProductKind::Mesh,
        "png" | "jpg" | "jpeg" => ProductKind::Texture,
        "tvol" => ProductKind::Volume,
        _ => return None,
    };
    let stem = path.with_extension("");
    let (stem, linear) = match stem.extension() {
        Some(infix) if infix == "linear" => (stem.with_extension(""), true),
        _ => (stem, false),
    };
    let name = stem
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Some((name, kind, linear))
}

fn action_key(
    kind: ProductKind,
    profile: Profile,
    quality: Quality,
    linear: bool,
    inputs: &[(PathBuf, ContentHash)],
) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"thrum-cook action\0");
    hasher.update(&COOKER_VERSION.to_le_bytes());
    hasher.update(ENCODERS.as_bytes());
    hasher.update(&[
        kind.code(),
        profile.code(),
        u8::from(quality == Quality::Final),
        u8::from(linear),
    ]);
    for (path, hash) in inputs {
        let text = path.to_string_lossy();
        hasher.update(&(text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
        hasher.update(hash.as_bytes());
    }
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

fn validate(kind: ProductKind, bytes: &[u8]) -> Result<(), CookError> {
    let bytes = thrum_asset::ProductBytes::copy_from(bytes);
    match kind {
        ProductKind::Mesh => thrum_asset::MeshProduct::open(bytes)?.validate_references()?,
        ProductKind::Texture | ProductKind::Volume => {
            let texture = thrum_asset::Ktx2::open(bytes)?;
            if texture.is_3d() != (kind == ProductKind::Volume) {
                return Err(CookError::Source(
                    "texture dimension does not match product kind".into(),
                ));
            }
        }
    }
    Ok(())
}

fn encode_manifest(
    generation: u64,
    products: &BTreeMap<Binding, (ProductKind, Product)>,
) -> Result<Vec<u8>, CookError> {
    let mut strings = Vec::new();
    let mut offsets: HashMap<&str, (u32, u16)> = HashMap::new();
    let mut records = Vec::with_capacity(products.len());
    for ((name, profile), (kind, product)) in products {
        let (name_offset, name_len) = *offsets.entry(name.as_str()).or_insert_with(|| {
            let offset = u32::try_from(strings.len()).unwrap_or(u32::MAX);
            strings.extend_from_slice(name.as_bytes());
            (offset, u16::try_from(name.len()).unwrap_or(u16::MAX))
        });
        if usize::from(name_len) != name.len() {
            return Err(CookError::Source(format!("asset name too long: {name}")));
        }
        records.push(ManifestRecord {
            name_key: ContentHash::of(name.as_bytes()).prefix_u64(),
            product: *product.hash.as_bytes(),
            byte_len: product.len,
            name_offset,
            name_len,
            kind: kind.code(),
            profile: profile.code(),
            reserved: [0; 8],
        });
    }
    records.sort_by_key(|record| (record.name_key, record.profile));
    for pair in records.windows(2) {
        if (pair[0].name_key, pair[0].profile) == (pair[1].name_key, pair[1].profile) {
            return Err(CookError::Source(
                "two asset names share a 64-bit key; rename one".into(),
            ));
        }
    }
    let meta = ManifestMeta {
        generation,
        entry_count: u32::try_from(records.len()).unwrap_or(u32::MAX),
        reserved: [0; 13],
    };
    Ok(ContainerWriter::new(MANIFEST_MAGIC)
        .section(manifest_tag::META, &[meta])
        .section(manifest_tag::ENTRIES, &records)
        .section(manifest_tag::STRINGS, &strings)
        .finish())
}

/// Highest generation among manifests still in the store; used only when `HEAD` is unreadable.
fn newest_manifest_generation(store: &thrum_asset::AssetStore) -> u64 {
    let Ok(shards) = std::fs::read_dir(store.root().join("cas")) else {
        return 0;
    };
    let mut newest = 0;
    for shard in shards
        .flatten()
        .filter(|shard| shard.file_name().len() == 2)
    {
        for file in std::fs::read_dir(shard.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let Ok(bytes) = std::fs::read(file.path()) else {
                continue;
            };
            if bytes.starts_with(&MANIFEST_MAGIC)
                && let Ok(manifest) = Manifest::parse(thrum_asset::ProductBytes::copy_from(&bytes))
            {
                newest = newest.max(manifest.generation());
            }
        }
    }
    newest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_edited_during_its_cook_is_not_cached_under_the_old_key() {
        let root = std::env::temp_dir().join(format!("thrum-cook-race-{}", std::process::id()));
        let source = root.join("source");
        std::fs::create_dir_all(&source).expect("dir");
        crate::demo::write_checker_png(&source.join("a.png"), 16, [200, 0, 0]).expect("png");
        let mut cooker = Cooker::open(CookConfig {
            source: source.clone(),
            store: root.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("open");
        cooker.stat.scan_all(&source).expect("scan");
        let (jobs, failed) = cooker.plan();
        assert_eq!(failed, Vec::<(String, String)>::new());
        // The save lands after the scan hashed the old bytes and before the cooker reads them.
        crate::demo::write_checker_png(&source.join("a.png"), 16, [0, 0, 200]).expect("edit");
        let error = cooker.run(&jobs[0]).expect_err("stale key");
        assert!(error.to_string().contains("changed during cook"), "{error}");
        drop(cooker);
        std::fs::remove_dir_all(&root).ok();
    }
}
