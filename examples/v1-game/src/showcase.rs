//! Authored scene data consumed once by the native game, through G5's existing residency path.

use serde::Deserialize;
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedMaterial, CookedSource, MeshPlacement, VolumePlacement,
};

use crate::SceneCamera;

/// The valley's three stable entity rows; repeated products share publication identity.
pub(crate) fn entity_mesh_names(content: crate::GameContent) -> Vec<String> {
    let names = match content {
        crate::GameContent::Exit => crate::exit_scene::MESH_NAMES.as_slice(),
        _ => &["rock", "sphere", "rock"],
    };
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    camera: Camera,
    placements: Vec<Placement>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Camera {
    position_m: [f32; 3],
    target_m: [f32; 3],
    vertical_fov_degrees: f32,
    near_m: f32,
    far_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Placement {
    mesh: String,
    texture: Option<String>,
    translation_m: [f32; 3],
    scale: [f32; 3],
    material: Material,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Material {
    #[serde(rename = "baseColorFactor")]
    base_colour: [f32; 4],
    #[serde(rename = "metallicFactor")]
    metalness: f32,
    #[serde(rename = "roughnessFactor")]
    roughness: f32,
}

/// Load the embedded dry-valley placement recipe against cooked products.
/// Camera and atmosphere share world metres, with sea level at Y = zero.
/// This prepares game policy; the renderer retains its device, sky and submitter.
///
/// # Errors
/// Returns an invalid embedded recipe or camera description.
pub(crate) fn prepare_showcase(
    source: CookedSource,
) -> Result<(SceneCamera, CookedAssets), String> {
    prepare_showcase_with_path(source, thrum_asset_gpu::CookedDrawPath::Auto)
}

pub(crate) fn prepare_showcase_with_path(
    source: CookedSource,
    path: thrum_asset_gpu::CookedDrawPath,
) -> Result<(SceneCamera, CookedAssets), String> {
    prepare_showcase_with_volume(source, None, path)
}

#[cfg(all(feature = "proving", any(target_os = "macos", target_arch = "wasm32")))]
pub(crate) fn prepare_content_preview(
    source: CookedSource,
) -> Result<(SceneCamera, CookedAssets), String> {
    let (camera, mut config) = valley_config(source).map_err(|error| error.to_string())?;
    config.leased = crate::content_preview::requests();
    let assets = CookedAssets::new(config).map_err(|error| error.to_string())?;
    Ok((camera, assets))
}

/// The authored valley recipe or contribution could not be prepared.
#[derive(Debug)]
pub struct CookedValleyError(String);

impl std::fmt::Display for CookedValleyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CookedValleyError {}

/// Prepare the embedded valley recipe from a checked cooked source.
///
/// # Errors
/// Returns the recipe or contribution preparation refusal.
#[doc(hidden)]
pub fn prepare_cooked_valley(
    source: CookedSource,
) -> Result<(SceneCamera, CookedAssets), CookedValleyError> {
    prepare_showcase(source).map_err(CookedValleyError)
}

/// Opt-in M2 fixture; the ordinary authored valley keeps no volume.
#[cfg(all(target_arch = "wasm32", feature = "proving"))]
pub(crate) fn prepare_temporal_showcase(
    source: CookedSource,
) -> Result<(SceneCamera, CookedAssets), String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Volume {
        name: String,
        transform: [[f32; 4]; 4],
        extinction: f32,
        steps: u32,
        light_steps: u32,
        #[serde(rename = "extent")]
        _extent: [u32; 3],
        #[serde(rename = "density")]
        _density: u8,
        #[serde(rename = "expected_central_byte")]
        _expected: u8,
    }
    let definition: serde_json::Value =
        serde_json::from_str(thrum_render::synthetic::TEMPORAL_WITNESS_JSON)
            .map_err(|e| e.to_string())?;
    let v: Volume =
        serde_json::from_value(definition["volume"].clone()).map_err(|e| e.to_string())?;
    prepare_showcase_with_volume(
        source,
        Some(VolumePlacement {
            name: v.name,
            transform: v.transform,
            extinction: v.extinction,
            steps: v.steps,
            light_steps: v.light_steps,
        }),
        thrum_asset_gpu::CookedDrawPath::Auto,
    )
}

fn prepare_showcase_with_volume(
    source: CookedSource,
    volume: Option<VolumePlacement>,
    path: thrum_asset_gpu::CookedDrawPath,
) -> Result<(SceneCamera, CookedAssets), String> {
    let (camera, mut config) = valley_config(source).map_err(|error| error.to_string())?;
    config.volume = volume;
    config.draw_path = path;
    let assets = CookedAssets::new(config).map_err(|error| format!("showcase: {error}"))?;
    Ok((camera, assets))
}

/// Authored valley camera and cooked contribution configuration.
///
/// # Errors
/// Returns an invalid authored camera or placement.
#[doc(hidden)]
pub(crate) fn valley_config(
    source: CookedSource,
) -> Result<(SceneCamera, CookedConfig), CookedValleyError> {
    let recipe: Recipe = serde_json::from_str(include_str!("../assets/showcase/scene.json"))
        .map_err(|error| CookedValleyError(format!("showcase recipe: {error}")))?;
    let camera = SceneCamera::perspective(
        recipe.camera.position_m,
        recipe.camera.target_m,
        recipe.camera.vertical_fov_degrees.to_radians(),
        recipe.camera.near_m,
        recipe.camera.far_m,
    )
    .map_err(CookedValleyError)?;
    let meshes = recipe
        .placements
        .into_iter()
        .map(Placement::checked)
        .collect::<Result<_, _>>()
        .map_err(CookedValleyError)?;
    let mut config = CookedConfig::new(source, meshes);
    config.poll_interval = std::time::Duration::from_millis(2);
    Ok((camera, config))
}

impl Placement {
    fn checked(self) -> Result<MeshPlacement, String> {
        let material = self.material;
        let factors = material
            .base_colour
            .into_iter()
            .chain([material.metalness, material.roughness]);
        if factors.into_iter().any(|v| !(0.0..=1.0).contains(&v))
            || !self.translation_m.into_iter().all(f32::is_finite)
            || !self.scale.into_iter().all(|v| v.is_finite() && v > 0.0)
            || self.scale.iter().any(|v| (*v - self.scale[0]).abs() > 1e-6)
        {
            return Err(format!(
                "showcase placement {} has invalid factors",
                self.mesh
            ));
        }
        let mesh = self
            .mesh
            .strip_suffix(".gltf")
            .ok_or("showcase mesh must name a glTF source")?;
        let mut placement = MeshPlacement::new(mesh, self.texture.as_deref().unwrap_or("white"));
        placement.transform = [
            [self.scale[0], 0.0, 0.0, 0.0],
            [0.0, self.scale[1], 0.0, 0.0],
            [0.0, 0.0, self.scale[2], 0.0],
            [
                self.translation_m[0],
                self.translation_m[1],
                self.translation_m[2],
                1.0,
            ],
        ];
        placement.material = CookedMaterial {
            base_colour: material.base_colour,
            roughness: material.roughness,
            metalness: material.metalness,
        };
        Ok(placement)
    }
}
