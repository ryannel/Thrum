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

/// The input a font may claim and the private PNG bindings discovery may expose.
struct FontBindings {
    input: Option<PathBuf>,
    private_pngs: Vec<String>,
}

/// A font's last published private texture binding, or None for a non-texture input.
/// A font missing from the map was published by a cooker that did not record this information.
type FontInputs = BTreeMap<String, Option<String>>;

type Products = BTreeMap<Binding, (ProductKind, Product)>;

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
    published: Products,
    published_font_inputs: FontInputs,
    /// Possible private PNGs from the last legacy scan. None means no saved scan was available
    /// for that font. Keep the snapshot until a manifest records the committed input.
    legacy_grid_candidates: HashMap<PathBuf, Option<HashSet<String>>>,
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
            published_font_inputs: BTreeMap::new(),
            legacy_grid_candidates: HashMap::new(),
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
        self.published_font_inputs = manifest
            .font_inputs()
            .and_then(|bytes| serde_json::from_slice(bytes).ok())
            .unwrap_or_default();
        self.recover_legacy_font_inputs();
    }

    fn recover_legacy_font_inputs(&mut self) {
        let mut files = None;
        for ((name, _), (_, product)) in self
            .published
            .iter()
            .filter(|(_, (kind, _))| *kind == ProductKind::Font)
        {
            if self.published_font_inputs.contains_key(name) {
                continue;
            }
            let Some(font) = self
                .store()
                .map(&product.hash)
                .ok()
                .and_then(|bytes| thrum_asset::FontProduct::open(bytes).ok())
            else {
                continue;
            };
            if font.sizes()[0].pixel == 0 {
                self.published_font_inputs.insert(name.clone(), None);
            } else {
                let files = files.get_or_insert_with(|| self.stat.files());
                let description = Path::new(name).join("font.json");
                let candidates = files.contains_key(&description).then(|| {
                    let folder = Path::new(name);
                    let pngs = files
                        .range(folder.to_path_buf()..)
                        .take_while(|(path, _)| path.starts_with(folder))
                        .filter(|(path, _)| path.parent() == Some(folder) && sheet::is_png(path));
                    pngs.filter_map(|(path, _)| classify(path).map(|(name, _, _)| name))
                        .collect()
                });
                // A refused legacy cook can overwrite the description hash while leaving HEAD
                // alone. At most a PNG known to that scan could be the committed private grid.
                self.legacy_grid_candidates
                    .insert(PathBuf::from(name), candidates);
                if let Some(input) = self.legacy_grid_input(name, *product, files) {
                    self.published_font_inputs.insert(name.clone(), Some(input));
                }
            }
        }
    }

    fn legacy_grid_input(
        &self,
        name: &str,
        product: Product,
        files: &BTreeMap<PathBuf, ContentHash>,
    ) -> Option<String> {
        // The saved stat cache may predate a refused edit. Accept a candidate only if
        // its exact description and PNG paths and hashes reproduce the published action.
        let description = Path::new(name).join("font.json");
        let folder = description.parent()?;
        let description_hash = *files.get(&description)?;
        let mut inputs = [
            (description.clone(), description_hash),
            (PathBuf::new(), description_hash),
        ];
        let mut matches = files
            .range(folder.to_path_buf()..)
            .take_while(|(path, _)| path.starts_with(folder))
            .filter(|(path, _)| path.parent() == Some(folder) && sheet::is_png(path))
            .filter(|(path, hash)| {
                inputs[1] = ((*path).clone(), **hash);
                [Quality::Fast, Quality::Final].into_iter().any(|quality| {
                    let key = action_key(
                        ProductKind::Font,
                        Profile::Portable,
                        quality,
                        false,
                        &inputs,
                    );
                    self.read_action(&key) == Some(product)
                })
            });
        let (path, _) = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        classify(path).map(|(name, _, _)| name)
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
        let mut font_inputs = self.resolve_font_bindings(&mut products, fonts);
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
        font_inputs.retain(|name, _| has_font(&products, name));
        report.cook_time = start.elapsed();
        let start = Instant::now();
        if products != self.published || font_inputs != self.published_font_inputs {
            report.published = Some(self.publish(&products, &font_inputs)?);
            self.published = products;
            self.published_font_inputs = font_inputs;
            self.legacy_grid_candidates.retain(|folder, _| {
                let name = logical_name(folder);
                has_font(&self.published, &name) && !self.published_font_inputs.contains_key(&name)
            });
        }
        // A refusal must leave HEAD alone. Keep the legacy snapshot across unrelated publishes
        // and reopens, so newly scanned PNGs cannot become candidates for an older private grid.
        let stat_path = self.config.store.join("stat-cache.bin");
        if self.legacy_grid_candidates.is_empty() {
            self.stat.save(&stat_path)?;
        } else {
            self.stat.save_preserving(&stat_path, |path| {
                (font::is_description(path) || sheet::is_png(path))
                    && path
                        .parent()
                        .is_some_and(|folder| self.legacy_grid_candidates.contains_key(folder))
            })?;
        }
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

    fn resolve_font_bindings(
        &self,
        products: &mut Products,
        fonts: BTreeMap<PathBuf, FontBindings>,
    ) -> FontInputs {
        let mut font_inputs = self.published_font_inputs.clone();
        for (folder, bindings) in fonts {
            let input = bindings
                .input
                .as_deref()
                .and_then(classify)
                .and_then(|(name, kind, _)| (kind == ProductKind::Texture).then_some(name));
            let name = logical_name(&folder);
            if has_font(products, &name) {
                font_inputs.insert(name, input);
                continue;
            }
            for name in bindings.private_pngs {
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
            if let Some(name) = input {
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
        font_inputs
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
                            private_pngs: Vec::new(),
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
                    && let Some(parent) = path.parent()
                    && let Some(font) = plan.fonts.get_mut(parent)
                    && self.could_be_private_font_input(parent, &name)
                    && !self
                        .published
                        .range((name.clone(), Profile::Portable)..)
                        .take_while(|((published, _), _)| published == &name)
                        .any(|(_, (kind, _))| *kind == ProductKind::Texture)
                {
                    font.private_pngs.push(name.clone());
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

    fn could_be_private_font_input(&self, folder: &Path, texture: &str) -> bool {
        let font = logical_name(folder);
        match self.published_font_inputs.get(&font) {
            Some(input) => input.as_deref() == Some(texture),
            None => self.legacy_grid_candidates.get(folder).map_or_else(
                || has_font(&self.published, &font),
                |candidates| {
                    candidates
                        .as_ref()
                        .is_none_or(|candidates| candidates.contains(texture))
                },
            ),
        }
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
        let product = self
            .actions
            .get(key)
            .copied()
            .or_else(|| self.read_action(key))?;
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

    fn read_action(&self, key: &ContentHash) -> Option<Product> {
        let bytes = std::fs::read(self.config.store.join("actions").join(key.to_hex())).ok()?;
        let hash = ContentHash::from_bytes(bytes.get(..32)?.try_into().ok()?);
        let len = u64::from_le_bytes(bytes.get(32..40)?.try_into().ok()?);
        Some(Product { hash, len })
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
        products: &Products,
        font_inputs: &FontInputs,
    ) -> Result<Head, CookError> {
        let generation = self.generation + 1;
        let bytes = encode_manifest(generation, products, font_inputs)?;
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

fn has_font(products: &Products, name: &str) -> bool {
    products
        .get(&(name.to_owned(), Profile::Portable))
        .is_some_and(|(kind, _)| *kind == ProductKind::Font)
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
    products: &Products,
    font_inputs: &FontInputs,
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
    let mut writer = ContainerWriter::new(MANIFEST_MAGIC)
        .section(manifest_tag::META, &[meta])
        .section(manifest_tag::ENTRIES, &records)
        .section(manifest_tag::STRINGS, &strings);
    if !font_inputs.is_empty() {
        let bytes = serde_json::to_vec(font_inputs)
            .map_err(|error| CookError::Source(format!("font inputs: {error}")))?;
        writer = writer.section(manifest_tag::FONT_INPUTS, &bytes);
    }
    Ok(writer.finish())
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
    fn legacy_grid_refusal_keeps_the_font_private_and_publishes_new_siblings() {
        for (rename, legacy_refused_first) in [(false, false), (true, false), (false, true)] {
            let root = thrum_test_support::Scratch::new("legacy-grid");
            let source = root.join("source");
            let folder = source.join("hud");
            std::fs::create_dir_all(&folder).expect("font folder");
            let grid = folder.join("grid.png");
            let icon = folder.join("icon.png");
            crate::demo::write_checker_png(&grid, 8, [200, 20, 40]).expect("grid");
            crate::demo::write_checker_png(&icon, 4, [20, 200, 40]).expect("public icon");
            let description = folder.join("font.json");
            let original = r#"{"kind":"pixel","file":"grid.png","cell":[4,4],"baseline":3,"characters":[{"character":"A","cell":[0,0],"advance":4}]}"#;
            std::fs::write(&description, original).expect("font description");
            let open = || {
                Cooker::open(CookConfig {
                    source: source.clone(),
                    store: root.join("store"),
                    profiles: Profile::ALL.to_vec(),
                    quality: Quality::Fast,
                    durability: Durability::Process,
                })
                .expect("open store")
            };
            let mut cooker = open();
            assert_eq!(cooker.cook_all().expect("initial cook").failed, []);
            // Select a legacy manifest with the same products and original cook records.
            let products = cooker.published.clone();
            let legacy = cooker.publish(&products, &FontInputs::new()).unwrap();
            let reader = cooker.store().clone();
            let before = reader.load_manifest(&legacy).unwrap();
            assert!(before.font_inputs().is_none());
            drop(cooker);

            let edit = if rename {
                original
                    .replace("grid.png", "icon.png")
                    .replace("[4,4]", "[3,3]")
            } else {
                original.replace("[4,4]", "[3,3]")
            };
            std::fs::write(&description, edit).expect("refused edit");
            if rename {
                crate::demo::write_checker_png(&icon, 4, [40, 200, 20])
                    .expect("edit proposed input");
            }
            if legacy_refused_first {
                // The old cooker writes the edited hashes after refusing the font, without
                // changing the published manifest or creating a successful action for the edit.
                let mut stat = StatCache::default();
                stat.scan_all(&source).expect("legacy scan of refused edit");
                stat.save(&root.join("store/stat-cache.bin"))
                    .expect("legacy stat save");
            }
            let refused = |report: &CookReport| {
                assert!(report.failed.iter().any(|(name, reason)| {
                    name == "hud" && reason.contains("pixel font cells must divide the image")
                }));
            };
            if !legacy_refused_first {
                let report = open().cook_all().expect("refuse font");
                refused(&report);
                assert!(report.published.is_none());
            }
            assert_eq!(reader.read_head().unwrap(), Some(legacy));

            crate::demo::write_checker_png(&folder.join("badge.png"), 4, [20, 40, 200])
                .expect("add an independent sibling in a later save");
            let notes = Path::new("notes.txt");
            std::fs::write(source.join(notes), "unrelated edit").expect("unrelated source");
            let report = open().cook_all().expect("reopen and cook the badge");
            refused(&report);
            let next = report.published.expect("badge publishes during refusal");
            let after = reader.load_manifest(&next).unwrap();
            assert_eq!(
                after.resolve("hud", &[Profile::Portable]),
                before.resolve("hud", &[Profile::Portable])
            );
            if legacy_refused_first {
                assert!(after.font_inputs().is_none());
                let saved = StatCache::load(&root.join("store/stat-cache.bin"));
                assert_eq!(
                    saved.hash(notes),
                    Some(ContentHash::of(b"unrelated edit")),
                    "unrelated scan entries still advance while ownership is unresolved"
                );
            } else {
                assert_eq!(
                    after.font_inputs(),
                    Some(br#"{"hud":"hud/grid"}"#.as_slice())
                );
            }
            assert!(!after.entries().any(|entry| entry.name == "hud/grid"));
            for profile in Profile::ALL {
                assert!(
                    after
                        .entries()
                        .any(|entry| entry.name == "hud/badge" && entry.profile == profile)
                );
                assert_eq!(
                    after.resolve("hud/icon", &[profile]),
                    before.resolve("hud/icon", &[profile])
                );
            }
            let report = open()
                .cook_all()
                .expect("reopen after the badge publication");
            refused(&report);
            assert!(report.published.is_none());
            if legacy_refused_first {
                // Another refused scan or unrelated publication must never broaden the
                // legacy candidate set, including for a previously unreadable new sibling.
                let mark = folder.join("mark.png");
                std::fs::write(&mark, b"invalid PNG").expect("add unfinished sibling");
                let report = open().cook_all().expect("refuse the unfinished sibling");
                assert!(report.failed.iter().any(|(name, _)| name == "hud/mark"));
                crate::demo::write_checker_png(&mark, 4, [40, 20, 200]).expect("repair sibling");
                let report = open().cook_all().expect("publish the repaired sibling");
                refused(&report);
                let after = reader.load_manifest(&report.published.unwrap()).unwrap();
                for profile in Profile::ALL {
                    assert!(
                        after
                            .entries()
                            .any(|entry| { entry.name == "hud/mark" && entry.profile == profile })
                    );
                }
                assert!(!after.entries().any(|entry| entry.name == "hud/grid"));
                std::fs::write(&description, original).expect("repair font");
                let report = open().cook_all().expect("publish recovered ownership");
                assert!(report.failed.is_empty(), "{report:?}");
                let after = reader.load_manifest(&report.published.unwrap()).unwrap();
                assert_eq!(
                    after.font_inputs(),
                    Some(br#"{"hud":"hud/grid"}"#.as_slice())
                );
            }
        }
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
