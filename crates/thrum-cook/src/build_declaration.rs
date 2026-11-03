//! Fixed first-profile Cargo build metadata from an accepted source pin.

use thrum_source::{
    AcceptedSourceRoot, JsonError, PathError, PinnedSource, SourcePath, SourceStoreError,
    parse_record_json,
};

use crate::{SceneProduct, SceneProductRevision, SceneTarget};

const DECLARATION_PATH: &str = "build/headless.json";
const MAX_DECLARATION_BYTES: usize = 64 * 1024;
const FIELDS: [&str; 7] = [
    "format",
    "cargoManifest",
    "cargoLock",
    "rustToolchain",
    "package",
    "ruleSource",
    "entryScene",
];

/// Parsed, checked metadata for a future trusted native compiler adapter.
/// This is not a target-ready `BuildRevision` or a compile attestation.
pub struct BuildDeclaration {
    root: AcceptedSourceRoot,
    scene_product: SceneProductRevision,
    cargo_manifest: SourcePath,
    cargo_lock: SourcePath,
    rust_toolchain: SourcePath,
    package: String,
    rule_source: SourcePath,
}

impl BuildDeclaration {
    /// Exact admitted root owning this declaration and all named files.
    pub fn source_root(&self) -> AcceptedSourceRoot {
        self.root
    }

    /// Scene product identity checked against the declaration's entry scene.
    pub fn scene_product(&self) -> SceneProductRevision {
        self.scene_product
    }

    /// Portable path of the Cargo manifest in the accepted tree.
    pub fn cargo_manifest(&self) -> &SourcePath {
        &self.cargo_manifest
    }

    /// Portable path of the Cargo lockfile in the accepted tree.
    pub fn cargo_lock(&self) -> &SourcePath {
        &self.cargo_lock
    }

    /// Portable path of the pinned Rust toolchain declaration.
    pub fn rust_toolchain(&self) -> &SourcePath {
        &self.rust_toolchain
    }

    /// Selected local Cargo package name; dependency closure is not yet checked.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Selected rule-source path for later compiler/binary provenance checks.
    pub fn rule_source(&self) -> &SourcePath {
        &self.rule_source
    }
}

/// The accepted tree cannot supply this first headless build declaration.
#[derive(Debug)]
pub enum BuildDeclarationError {
    /// The scene product was derived from another accepted source root/target.
    ProductMismatch,
    /// No declaration file exists at the fixed first-profile path.
    MissingDeclaration,
    /// The declaration exceeds the fixed metadata-format bound.
    TooLarge,
    /// Strict JSON parsing failed, including duplicate keys.
    Syntax(JsonError),
    /// The root must be a JSON object.
    RootType,
    /// An unknown declaration key is unsupported.
    UnknownField(String),
    /// A required declaration key is absent.
    MissingField(&'static str),
    /// A known field has an invalid type or value.
    InvalidField(&'static str),
    /// One named input path is not portable.
    InvalidPath {
        /// Declaration field naming the path.
        field: &'static str,
        /// Portable-path validation failure.
        source: PathError,
    },
    /// The declared entry scene differs from the cooked scene.
    EntryMismatch,
    /// A named build input is missing from the accepted tree.
    MissingInput(SourcePath),
    /// Verified source bytes could not be read.
    Source(SourceStoreError),
}

/// Read and validate a fixed headless build declaration from accepted bytes.
///
/// This validates membership and identity only. T2 must still parse Cargo and
/// toolchain files, close dependencies, isolate compiler execution and verify
/// the binary before any build can be called ready.
pub fn read_pinned_build_declaration(
    pin: &mut PinnedSource,
    product: &SceneProduct,
) -> Result<BuildDeclaration, BuildDeclarationError> {
    if product.source_root() != pin.root() || product.target() != SceneTarget::Headless {
        return Err(BuildDeclarationError::ProductMismatch);
    }
    let path = SourcePath::parse(DECLARATION_PATH).expect("fixed portable path");
    let index = pin
        .manifest()
        .files()
        .binary_search_by(|file| file.path().cmp(&path))
        .map_err(|_| BuildDeclarationError::MissingDeclaration)?;
    let (length, _) = pin.manifest().files()[index].content_identity();
    if length > MAX_DECLARATION_BYTES as u64 {
        return Err(BuildDeclarationError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(length as usize);
    pin.copy_file_to(&path, &mut bytes)
        .map_err(BuildDeclarationError::Source)?;
    let value = parse_record_json(&bytes).map_err(BuildDeclarationError::Syntax)?;
    let fields = value.as_object().ok_or(BuildDeclarationError::RootType)?;
    for key in fields.keys() {
        if !FIELDS.contains(&key.as_str()) {
            return Err(BuildDeclarationError::UnknownField(key.clone()));
        }
    }
    for name in FIELDS {
        if !fields.contains_key(name) {
            return Err(BuildDeclarationError::MissingField(name));
        }
    }
    if fields.get("format").and_then(|value| value.as_u64()) != Some(1) {
        return Err(BuildDeclarationError::InvalidField("format"));
    }
    let cargo_manifest = path_field(fields, "cargoManifest")?;
    let cargo_lock = path_field(fields, "cargoLock")?;
    let rust_toolchain = path_field(fields, "rustToolchain")?;
    let rule_source = path_field(fields, "ruleSource")?;
    let package = fields
        .get("package")
        .and_then(|value| value.as_str())
        .ok_or(BuildDeclarationError::InvalidField("package"))?;
    if package.is_empty()
        || package.len() > 128
        || !package
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(BuildDeclarationError::InvalidField("package"));
    }
    let entry = fields
        .get("entryScene")
        .and_then(|value| value.as_str())
        .ok_or(BuildDeclarationError::InvalidField("entryScene"))?;
    if entry != product.scene_id() {
        return Err(BuildDeclarationError::EntryMismatch);
    }
    for path in [&cargo_manifest, &cargo_lock, &rust_toolchain, &rule_source] {
        if pin
            .manifest()
            .files()
            .binary_search_by(|file| file.path().cmp(path))
            .is_err()
        {
            return Err(BuildDeclarationError::MissingInput(path.clone()));
        }
    }
    Ok(BuildDeclaration {
        root: pin.root(),
        scene_product: product.revision(),
        cargo_manifest,
        cargo_lock,
        rust_toolchain,
        package: package.to_owned(),
        rule_source,
    })
}

fn path_field(
    fields: &serde_json::Map<String, serde_json::Value>,
    name: &'static str,
) -> Result<SourcePath, BuildDeclarationError> {
    let text = fields
        .get(name)
        .and_then(|value| value.as_str())
        .ok_or(BuildDeclarationError::InvalidField(name))?;
    SourcePath::parse(text).map_err(|source| BuildDeclarationError::InvalidPath {
        field: name,
        source,
    })
}
