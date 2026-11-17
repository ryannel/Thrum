//! Cooked products adapted to the renderer's admitted whole-mesh rows.

use std::sync::{Arc, Mutex};
use thrum_asset::{AssetError, ContentHash, MeshProduct, Profile, RemoteGeneration};
use thrum_render::synthetic::mesh_rows::{MeshRowError, MeshRowGeometry, MeshRows};
use thrum_render::synthetic::{SyntheticMesh, SyntheticVertex};

/// Failure to prepare a cooked mesh for an entity row.
#[derive(Debug)]
pub enum CookedRowError {
    /// The product could not be read or validated.
    Product(AssetError),
    /// The arena would refuse the geometry.
    Admission(MeshRowError),
    /// A requested mesh is absent from this generation.
    Missing(String),
}

impl std::fmt::Display for CookedRowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Product(error) => error.fmt(f),
            Self::Admission(error) => error.fmt(f),
            Self::Missing(name) => write!(f, "cooked mesh row unavailable: {name}"),
        }
    }
}

impl std::error::Error for CookedRowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Product(error) => Some(error),
            Self::Admission(error) => Some(error),
            Self::Missing(_) => None,
        }
    }
}

/// Decode the product's octahedral normal, matching the cooked vertex shader.
fn normal(encoded: [i16; 2]) -> [f32; 3] {
    let [x, y] = encoded.map(|v| (f32::from(v) / 32767.0).max(-1.0));
    let mut n = glam::Vec3::new(x, y, 1.0 - x.abs() - y.abs());
    let t = (-n.z).max(0.0);
    n.x += if n.x >= 0.0 { -t } else { t };
    n.y += if n.y >= 0.0 { -t } else { t };
    n.normalize().to_array()
}

/// Convert and pack on the loader thread (or between browser frames).
/// Positions, UVs and global triangle indices are copied exactly.
/// # Errors
/// Refuses malformed references and every mesh the renderer's arena refuses.
pub fn mesh_row(
    product: &MeshProduct,
    max_binding: u64,
) -> Result<MeshRowGeometry, CookedRowError> {
    product
        .validate_references()
        .map_err(CookedRowError::Product)?;
    let vertices = product
        .positions()
        .iter()
        .zip(product.attributes())
        .map(|(&position, attributes)| {
            let normal = normal(attributes.normal_oct);
            let n = glam::Vec3::from_array(normal);
            let axis = if n.z.abs() < 0.9 {
                glam::Vec3::Z
            } else {
                glam::Vec3::Y
            };
            let tangent = axis.cross(n).normalize();
            SyntheticVertex {
                position,
                normal,
                uv: attributes.uv,
                tangent: [tangent.x, tangent.y, tangent.z, 1.0],
            }
        })
        .collect();
    MeshRowGeometry::new(
        SyntheticMesh {
            vertices,
            indices: product.indices().to_vec(),
        },
        max_binding,
    )
    .map_err(CookedRowError::Admission)
}

/// Named rows sharing residency's publication and loader.
#[derive(Clone, Debug)]
pub struct CookedRows {
    pub(crate) shown: Arc<Mutex<Vec<(ContentHash, MeshRowGeometry)>>>,
    pub(crate) names: Vec<String>,
    pub(crate) initial: Vec<(ContentHash, MeshRowGeometry)>,
    pub(crate) updates: MeshRows,
}

impl CookedRows {
    /// Prepare the scene's initial rows from the current mapped generation.
    /// # Errors
    /// Refuses an absent publication, missing product or arena admission failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_store(
        store: &thrum_asset::AssetStore,
        names: Vec<String>,
        max_binding: u64,
    ) -> Result<Self, CookedRowError> {
        let head = store
            .read_head()
            .map_err(CookedRowError::Product)?
            .ok_or_else(|| CookedRowError::Missing("HEAD".into()))?;
        let manifest = store
            .load_manifest(&head)
            .map_err(CookedRowError::Product)?;
        let initial = names
            .iter()
            .map(|name| {
                let hash = manifest
                    .resolve(name, &[Profile::Portable])
                    .ok_or_else(|| CookedRowError::Missing(name.clone()))?
                    .product;
                let product = MeshProduct::open(store.map(&hash).map_err(CookedRowError::Product)?)
                    .map_err(CookedRowError::Product)?;
                Ok((hash, mesh_row(&product, max_binding)?))
            })
            .collect::<Result<Vec<_>, CookedRowError>>()?;
        Ok(Self::new(names, initial, max_binding))
    }

    /// Prepare the same initial selection from a fetched generation.
    /// # Errors
    /// Refuses a missing or invalid product and arena admission failures.
    pub fn from_generation(
        generation: &RemoteGeneration,
        names: Vec<String>,
        max_binding: u64,
    ) -> Result<Self, CookedRowError> {
        let initial = names
            .iter()
            .map(|name| {
                let hash = generation
                    .resolve(name, &[Profile::Portable])
                    .ok_or_else(|| CookedRowError::Missing(name.clone()))?
                    .product;
                let product = MeshProduct::open(
                    generation
                        .product(&hash)
                        .ok_or_else(|| CookedRowError::Missing(name.clone()))?,
                )
                .map_err(CookedRowError::Product)?;
                Ok((hash, mesh_row(&product, max_binding)?))
            })
            .collect::<Result<Vec<_>, CookedRowError>>()?;
        Ok(Self::new(names, initial, max_binding))
    }

    fn new(
        names: Vec<String>,
        initial: Vec<(ContentHash, MeshRowGeometry)>,
        max_binding: u64,
    ) -> Self {
        let updates = MeshRows::from_geometry(
            &initial
                .iter()
                .map(|(_, geometry)| geometry.clone())
                .collect::<Vec<_>>(),
        )
        .with_binding_limit(max_binding);
        Self {
            shown: Arc::new(Mutex::new(initial.clone())),
            names,
            initial,
            updates,
        }
    }

    /// Connect a browser host prepared before its first store fetch.
    /// That first publication replaces its loading geometry through the same handshake.
    #[must_use]
    pub fn with_reader(mut self, updates: MeshRows) -> Self {
        self.updates = updates;
        self.shown = Arc::new(Mutex::new(Vec::new()));
        self
    }

    /// Build the entity selection with the game's stable slot choices.
    pub fn selection(
        &self,
        slots: Vec<thrum_render::synthetic::SyntheticMeshChoice>,
    ) -> Arc<thrum_render::synthetic::SyntheticMeshes> {
        Arc::new(thrum_render::synthetic::SyntheticMeshes {
            meshes: self
                .initial
                .iter()
                .map(|(_, geometry)| geometry.mesh().clone())
                .collect(),
            slots,
            updates: Some(self.updates.clone()),
        })
    }

    /// Shared host reader; the publication owner retains the preparation policy.
    pub fn updates(&self) -> MeshRows {
        self.updates.clone()
    }
}
