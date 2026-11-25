//! The cook cycle: scan sources, plan assets, reuse or cook products, publish one manifest.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use rayon::prelude::*;
use thrum_asset::layout::{MANIFEST_MAGIC, ManifestMeta, ManifestRecord, manifest_tag};
use thrum_asset::{ContentHash, Head, Manifest, ProductKind, Profile};

use crate::cas::{Cas, Durability};
use crate::stat_cache::{ScanCounts, StatCache};
use crate::texture::Quality;
use crate::write::ContainerWriter;
use crate::{CookError, font, mesh, sheet, texture};

/// Version of every cooker's output; bump to invalidate all cached actions.
/// 3: mirrored glTF nodes keep front-face winding and outward normals.
const COOKER_VERSION: u32 = 3;
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
    /// Texture sources decoded and mipmapped for the products cooked this cycle.
    pub texture_preparations: usize,
    /// Volume sources parsed and mipmapped for the products cooked this cycle.
    pub volume_preparations: usize,
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

#[derive(Default)]
struct CookPlan {
    jobs: Vec<Job>,
    failed: Vec<(String, String)>,
    retry: bool,
    fonts: BTreeMap<PathBuf, FontBindings>,
}

/// The input a font may claim and the PNG bindings that discovery may expose.
struct FontBindings {
    input: Option<PathBuf>,
    unpublished_pngs: Vec<String>,
}

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

enum PreparedSource {
    Mesh(Vec<u8>),
    Texture(texture::TextureMips),
    Volume(texture::VolumeMips),
    Sheet(sheet::PreparedSheet),
    Font(Vec<u8>),
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
    retry: bool,
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
            retry: false,
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

    /// Exercise a save racing the real cook cycle, deterministically after scanning.
    ///
    /// # Errors
    /// As [`Cooker::cook_all`].
    #[cfg(feature = "fault-probe")]
    pub fn cook_with_save_after_scan(
        &mut self,
        save: impl FnOnce(),
    ) -> Result<CookReport, CookError> {
        let start = Instant::now();
        let scan = self.stat.scan_all(&self.config.source)?;
        let scan_time = start.elapsed();
        save();
        self.cycle(scan, scan_time)
    }

    // Only a watch sweep waits for recent writes. Startup and explicit cooks remain immediate.
    // Stamp/content changes, rather than racy rehashes, decide whether the store needs work.
    pub(crate) fn sweep(&mut self, now: SystemTime) -> Result<Option<CookReport>, CookError> {
        let start = Instant::now();
        let before = self.stat.revision();
        let scan = self
            .stat
            .scan_settled(&self.config.source, now, crate::watch::MAX_BATCH)
            .inspect_err(|_| self.retry = true)?;
        if self.stat.revision() == before && !self.retry {
            return Ok(None);
        }
        self.cycle(scan, start.elapsed()).map(Some)
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
        // A failed publication or cache write must be retried even with unchanged source hashes.
        self.retry = true;
        let mut report = CookReport {
            scan,
            scan_time,
            ..CookReport::default()
        };
        let start = Instant::now();
        let CookPlan {
            jobs,
            mut failed,
            mut retry,
            fonts,
        } = self.plan();
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
        // Planning emits all profiles of a source together. Cache hits leave
        // that order intact, and only groups with misses need preparation.
        let groups: Vec<_> = misses.chunk_by(|a, b| a.source == b.source).collect();
        let outcomes: Vec<_> = groups
            .into_par_iter()
            .map(|jobs| {
                let products = self.prepare(&jobs[0]).map(|prepared| {
                    jobs.par_iter()
                        .map(|job| self.run(job, &prepared))
                        .collect::<Vec<_>>()
                });
                (jobs, products)
            })
            .collect();
        // A source can change after hashing but before cooking. Retry even if its next stable
        // hash equals the cached hash: no new revision would otherwise trigger that job.
        let mut failed_sheets = HashSet::new();
        for (jobs, outcomes) in outcomes {
            let outcomes = match outcomes {
                Ok(outcomes) => outcomes,
                Err(error) => {
                    let job = &jobs[0];
                    retry |= self.retry_failure(job, &error);
                    failed.push((job.name.clone(), error.to_string()));
                    if job.kind == ProductKind::SpriteSheet {
                        failed_sheets.insert(job.name.clone());
                    }
                    continue;
                }
            };
            report.texture_preparations += usize::from(jobs[0].kind == ProductKind::Texture);
            report.volume_preparations += usize::from(jobs[0].kind == ProductKind::Volume);
            for (job, outcome) in jobs.iter().zip(outcomes) {
                match outcome {
                    Ok(product) => {
                        report.cooked += 1;
                        self.remember_action(job.key, product);
                        products.insert((job.name.clone(), job.profile), (job.kind, product));
                    }
                    Err(CookError::Refused(reason)) => {
                        self.refusals.insert(job.key, reason.clone());
                        refused.insert((job.name.clone(), job.profile), reason);
                    }
                    Err(error) => {
                        // A deterministic import failure has already been reported. Only unfinished
                        // I/O or inputs that moved during the import need another unchanged sweep.
                        retry |= self.retry_failure(job, &error);
                        failed.push((job.name.clone(), error.to_string()));
                        if job.kind == ProductKind::SpriteSheet {
                            failed_sheets.insert(job.name.clone());
                        }
                    }
                }
            }
        }
        // Every encoding of a sheet describes the same frame table; retain the whole previous
        // product family if one encoding failed after another profile had succeeded or hit cache.
        if !failed_sheets.is_empty() {
            products.retain(|(name, _), _| !failed_sheets.contains(name));
        }
        self.retain_refused_font_bindings(&mut products, fonts);
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
        self.retry = retry;
        Ok(report)
    }

    fn retain_refused_font_bindings(
        &self,
        products: &mut BTreeMap<Binding, (ProductKind, Product)>,
        fonts: BTreeMap<PathBuf, FontBindings>,
    ) {
        for (folder, bindings) in fonts {
            if products
                .get(&(logical_name(&folder), Profile::Portable))
                .is_some_and(|(kind, _)| *kind == ProductKind::Font)
            {
                continue;
            }
            // HEAD records which PNGs were public. A newly discovered PNG could be the
            // old font input, so defer exposing it until the new font can take ownership.
            for name in bindings.unpublished_pngs {
                for &profile in &self.config.profiles {
                    let binding = (name.clone(), profile);
                    if products
                        .get(&binding)
                        .is_some_and(|(kind, _)| *kind == ProductKind::Texture)
                    {
                        products.remove(&binding);
                    }
                }
            }
            if let Some(input) = bindings.input
                && let Some((name, ProductKind::Texture, _)) = classify(&input)
            {
                // Discovery hid the proposed input. Only a successful font may take its
                // previously public bindings away, even if the input bytes also changed.
                for (binding, previous) in self
                    .published
                    .range((name.clone(), Profile::Portable)..)
                    .take_while(|((n, _), _)| n == &name)
                    .filter(|(_, (kind, _))| *kind == ProductKind::Texture)
                {
                    products.insert(binding.clone(), *previous);
                }
            }
        }
    }

    fn plan(&mut self) -> CookPlan {
        let files = self.stat.files();
        let mut plan = CookPlan::default();
        let mut sheet_images = BTreeMap::<&Path, Vec<(PathBuf, ContentHash)>>::new();
        for (path, hash) in &files {
            if let Some(parent) = path.parent() {
                if sheet::is_description(path) {
                    sheet_images.insert(parent, Vec::new());
                } else if font::is_description(path) {
                    let bytes = std::fs::read(self.config.source.join(path));
                    let input = match bytes {
                        Ok(bytes) if ContentHash::of(&bytes) == *hash => {
                            font::input_path(path, &bytes)
                        }
                        _ => {
                            plan.retry = true;
                            None
                        }
                    };
                    plan.fonts.insert(
                        parent.to_path_buf(),
                        FontBindings {
                            input,
                            unpublished_pngs: Vec::new(),
                        },
                    );
                }
            }
        }
        let mut sources = BTreeMap::<String, Vec<_>>::new();
        for (path, hash) in &files {
            if let Some(parent) = path.parent() {
                if plan
                    .fonts
                    .get(parent)
                    .is_some_and(|font| font.input.as_ref() == Some(path))
                {
                    continue;
                }
                if let Some(images) = sheet_images.get_mut(parent)
                    && sheet::is_png(path)
                {
                    images.push((path.clone(), *hash));
                    continue;
                }
            }
            if let Some((name, kind, linear)) = classify(path) {
                if kind == ProductKind::Texture
                    && sheet::is_png(path)
                    && let Some(font) = path.parent().and_then(|parent| plan.fonts.get_mut(parent))
                    && !self
                        .published
                        .range((name.clone(), Profile::Portable)..)
                        .take_while(|((published, _), _)| published == &name)
                        .any(|(_, (kind, _))| *kind == ProductKind::Texture)
                {
                    // A malformed description has no proposed input and cannot cook at all.
                    if font.input.is_none() {
                        continue;
                    }
                    font.unpublished_pngs.push(name.clone());
                }
                sources
                    .entry(name)
                    .or_default()
                    .push((path, hash, kind, linear));
            }
        }
        for (name, paths) in sources {
            let [(path, hash, kind, linear)] = paths.as_slice() else {
                let list: Vec<String> = paths
                    .iter()
                    .map(|(path, ..)| path.display().to_string())
                    .collect();
                plan.failed.push((
                    name,
                    format!(
                        "several sources map to one logical name: {}",
                        list.join(", ")
                    ),
                ));
                continue;
            };
            let (path, hash, kind, linear) = (*path, **hash, *kind, *linear);
            let mut inputs = vec![(path.clone(), hash)];
            if let Some(parent) = path.parent() {
                if kind == ProductKind::SpriteSheet
                    && let Some(images) = sheet_images.get(parent)
                {
                    inputs.extend(images.iter().cloned());
                }
                if kind == ProductKind::Font
                    && let Some(input) = plan.fonts.get(parent).and_then(|font| font.input.as_ref())
                    && let Some(input_hash) = files.get(input)
                {
                    inputs.push((input.clone(), *input_hash));
                }
            }
            if kind == ProductKind::Mesh {
                match self.mesh_buffers(path, hash) {
                    Ok(buffers) => {
                        let mut missing = None;
                        for buffer in buffers {
                            match files.get(&buffer) {
                                Some(buffer_hash) => inputs.push((buffer, *buffer_hash)),
                                None => missing = Some(buffer),
                            }
                        }
                        if let Some(buffer) = missing {
                            plan.failed
                                .push((name, format!("missing glTF buffer {}", buffer.display())));
                            continue;
                        }
                    }
                    Err(error) => {
                        plan.failed.push((name, error.to_string()));
                        continue;
                    }
                }
            }
            let profiles = if matches!(kind, ProductKind::Mesh | ProductKind::Font) {
                vec![Profile::Portable]
            } else {
                self.config.profiles.clone()
            };
            for profile in profiles {
                let key = action_key(kind, profile, self.config.quality, linear, &inputs);
                plan.jobs.push(Job {
                    name: name.clone(),
                    kind,
                    profile,
                    key,
                    source: self.config.source.join(path),
                    inputs: inputs.clone(),
                });
            }
        }
        plan
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

    fn retry_failure(&self, job: &Job, error: &CookError) -> bool {
        matches!(error, CookError::Io(_))
            || job.inputs.iter().any(|(path, hash)| {
                std::fs::read(self.config.source.join(path))
                    .map_or(true, |bytes| ContentHash::of(&bytes) != *hash)
            })
    }

    fn prepare(&self, job: &Job) -> Result<PreparedSource, CookError> {
        // The action key names the hashed input bytes; a product cooked from anything else would
        // poison the persistent action cache. Single-file sources are cooked from bytes checked
        // against their hash; multi-file meshes are re-verified after the importer reads them.
        let changed = || CookError::SourceChanged;
        let read_verified = || -> Result<Vec<u8>, CookError> {
            let bytes = std::fs::read(&job.source)?;
            if ContentHash::of(&bytes) == job.inputs[0].1 {
                Ok(bytes)
            } else {
                Err(changed())
            }
        };
        match job.kind {
            ProductKind::Mesh => {
                let bytes = mesh::cook(&job.source)?;
                for (path, hash) in &job.inputs {
                    if ContentHash::of(&std::fs::read(self.config.source.join(path))?) != *hash {
                        return Err(changed());
                    }
                }
                Ok(PreparedSource::Mesh(bytes))
            }
            ProductKind::Texture => {
                let linear = classify(&job.source).is_some_and(|(_, _, linear)| linear);
                texture::prepare_texture(&read_verified()?, linear).map(PreparedSource::Texture)
            }
            ProductKind::Volume => {
                texture::prepare_volume(&read_verified()?).map(PreparedSource::Volume)
            }
            ProductKind::SpriteSheet | ProductKind::Font => {
                let description = read_verified()?;
                let images = job.inputs[1..].iter().map(|(path, hash)| {
                    let bytes = std::fs::read(self.config.source.join(path))?;
                    if ContentHash::of(&bytes) != *hash {
                        return Err(changed());
                    }
                    Ok((path.as_path(), bytes))
                });
                if job.kind == ProductKind::Font {
                    font::cook(&description, images).map(PreparedSource::Font)
                } else {
                    sheet::prepare(&description, images).map(PreparedSource::Sheet)
                }
            }
        }
    }

    fn run(&self, job: &Job, prepared: &PreparedSource) -> Result<Product, CookError> {
        let encoded;
        let bytes = match prepared {
            PreparedSource::Mesh(bytes) | PreparedSource::Font(bytes) => bytes.as_slice(),
            PreparedSource::Texture(texture) => {
                encoded = texture.encode(job.profile, self.config.quality)?;
                &encoded
            }
            PreparedSource::Sheet(sheet) => {
                encoded = sheet.encode(job.profile, self.config.quality)?;
                &encoded
            }
            PreparedSource::Volume(volume) => {
                encoded = volume.encode(job.profile)?;
                &encoded
            }
        };
        validate(job.kind, bytes)?;
        let hash = self.cas.put(bytes)?;
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
    let folder_kind = if sheet::is_description(path) {
        Some(ProductKind::SpriteSheet)
    } else if font::is_description(path) {
        Some(ProductKind::Font)
    } else {
        None
    };
    if let Some(kind) = folder_kind {
        return Some((logical_name(path.parent()?), kind, false));
    }
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
    Some((logical_name(&stem), kind, linear))
}

fn logical_name(stem: &Path) -> String {
    stem.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
        ProductKind::SpriteSheet => {
            thrum_asset::SheetProduct::open(bytes)?;
        }
        ProductKind::Font => {
            thrum_asset::FontProduct::open(bytes)?;
        }
        ProductKind::Texture | ProductKind::Volume => {
            let texture = thrum_asset::Ktx2::open(bytes)?;
            if texture.is_3d() != (kind == ProductKind::Volume) || texture.array_layers() > 1 {
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
    fn a_restored_source_retries_an_interrupted_plan_without_another_notice() {
        let root = thrum_test_support::Scratch::new("restored-plan");
        let source = root.join("source");
        std::fs::create_dir_all(&source).expect("source");
        crate::demo::write_grid_gltf(&source, "a", 2, 0.1).expect("mesh");
        let path = source.join("a.gltf");
        let original = std::fs::read(&path).expect("original source");
        let mut cooker = Cooker::open(CookConfig {
            source: source.clone(),
            store: root.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("open");
        let scan = cooker.stat.scan_all(&source).expect("scan valid bytes");
        // The writer interrupts planning after the scan, then finishes with exactly the
        // original bytes. Hash equality alone cannot mean the failed plan is now published.
        std::fs::write(&path, b"{").expect("interrupted save");
        let failed = cooker
            .cycle(scan, Duration::ZERO)
            .expect("failed plan report");
        assert_eq!(failed.failed.len(), 1);
        assert!(failed.published.is_none());
        std::fs::write(&path, original).expect("finished save");
        let now = SystemTime::now() + Duration::from_secs(2);
        let repaired = cooker
            .sweep(now)
            .expect("sweep")
            .expect("restoration is a change");
        assert!(repaired.failed.is_empty(), "{repaired:?}");
        assert!(repaired.published.is_some());
        assert!(cooker.sweep(now).expect("unchanged sweep").is_none());
        drop(cooker);
    }

    #[test]
    fn a_source_edited_during_its_cook_is_not_cached_under_the_old_key() {
        let root = thrum_test_support::Scratch::new("cook-race");
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
        let first = cooker.cook_all().expect("publish the last good texture");
        let first_head = first.published.expect("initial publication");
        let store = thrum_asset::AssetStore::new(root.join("store"));
        assert_eq!(store.read_head().expect("initial HEAD"), Some(first_head));
        crate::demo::write_checker_png(&source.join("a.png"), 16, [0, 200, 0])
            .expect("save that starts the next cook");
        let scan = cooker.stat.scan_all(&source).expect("scan green bytes");
        // The next save lands after the scan hashed the green bytes and before the cooker reads them.
        crate::demo::write_checker_png(&source.join("a.png"), 16, [0, 0, 200]).expect("edit");
        let raced = cooker.cycle(scan, Duration::ZERO).expect("raced cook");
        assert_eq!(
            raced.failed,
            vec![("a".into(), CookError::SourceChanged.to_string())]
        );
        assert!(raced.published.is_none());
        assert_eq!(store.read_head().expect("retained HEAD"), Some(first_head));
        let corrected = cooker.cook_all().expect("cook latest stable bytes");
        assert!(corrected.failed.is_empty(), "{corrected:?}");
        let head = corrected.published.expect("corrected publication");
        assert_eq!(head.generation, first_head.generation + 1);
        assert_ne!(head.manifest, first_head.manifest);
        drop(cooker);
    }
}
