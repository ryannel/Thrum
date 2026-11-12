//! Authored scene data consumed once by the native game, through G5's existing residency path.

use serde::Deserialize;
use thrum_asset_gpu::{CookedAssets, CookedMaterial, CookedSource, MeshPlacement, VolumePlacement};

use crate::SceneCamera;

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
    prepare_showcase_with_volume(source, None)
}

/// Opt-in M2 fixture; the ordinary authored valley keeps no volume.
#[cfg(target_arch = "wasm32")]
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
    let definition: serde_json::Value = serde_json::from_str(include_str!(
        "../../../crates/thrum-render/src/synthetic/temporal/witness.json"
    ))
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
    )
}

fn prepare_showcase_with_volume(
    source: CookedSource,
    volume: Option<VolumePlacement>,
) -> Result<(SceneCamera, CookedAssets), String> {
    let recipe: Recipe = serde_json::from_str(include_str!("../assets/showcase/scene.json"))
        .map_err(|error| format!("showcase recipe: {error}"))?;
    let camera = SceneCamera::perspective(
        recipe.camera.position_m,
        recipe.camera.target_m,
        recipe.camera.vertical_fov_degrees.to_radians(),
        recipe.camera.near_m,
        recipe.camera.far_m,
    )?;
    let mut config = crate::cooked::cooked_config(source);
    config.volume = volume;
    config.meshes = recipe
        .placements
        .into_iter()
        .map(Placement::checked)
        .collect::<Result<_, _>>()?;
    let assets = CookedAssets::new(config).map_err(|error| format!("showcase: {error}"))?;
    Ok((camera, assets))
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
