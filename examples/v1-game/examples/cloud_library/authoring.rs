use std::{error::Error, fs, io, path::Path};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thrum_asset::{AssetStore, Profile};
use thrum_cloud_model::Shape;
use thrum_cook::{CookConfig, Cooker, Durability, Quality, encode_volume};

pub(super) const SIDE: usize = 128;
pub(super) const FOOTPRINT_RECORD_BYTES: usize = (5 + SIDE * SIDE * 3) * 4;

#[derive(Deserialize)]
struct Library {
    shapes: Vec<Slab>,
}

#[derive(Deserialize)]
struct GrowthTrack {
    keys: Vec<Slab>,
}

#[derive(Deserialize)]
struct Slab {
    payload: String,
    metadata: String,
    payload_sha256: String,
    voxel_sha256: String,
    metadata_sha256: Option<String>,
}

#[derive(Deserialize)]
struct Metadata {
    side: usize,
    tiles_per_row: usize,
    extent_m: f64,
    base_height_m: f64,
    downwind_axis: [f64; 3],
    payload_sha256: String,
    voxel_sha256: String,
}

pub(super) fn cook(
    library: &Path,
    work: &Path,
    store: &Path,
    profiles: Vec<Profile>,
) -> Result<AssetStore, Box<dyn Error>> {
    let shapes: Library = serde_json::from_slice(&fs::read(library.join("library.json"))?)?;
    let growth = library.join("growth");
    let track: GrowthTrack =
        serde_json::from_slice(&fs::read(growth.join("growth-track-manifest.json"))?)?;
    fs::create_dir_all(work)?;
    let mut footprints =
        Vec::with_capacity((shapes.shapes.len() + track.keys.len()) * FOOTPRINT_RECORD_BYTES);
    stack(
        library,
        &shapes.shapes,
        &work.join("cloud-library.tvol"),
        &mut footprints,
    )?;
    stack(
        &growth,
        &track.keys,
        &work.join("cloud-growth.tvol"),
        &mut footprints,
    )?;
    let mut cooker = Cooker::open(CookConfig {
        source: work.to_owned(),
        store: store.to_owned(),
        profiles,
        quality: Quality::Final,
        durability: Durability::Process,
    })?;
    let report = cooker.cook_all()?;
    if !report.failed.is_empty() {
        return Err(
            io::Error::other(format!("Cloud stack cook failed: {:?}", report.failed)).into(),
        );
    }
    fs::write(store.join("cloud-footprints.bin"), footprints)?;
    Ok(AssetStore::new(store))
}

fn stack(
    root: &Path,
    slabs: &[Slab],
    source: &Path,
    footprints: &mut Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    let mut stack = Vec::with_capacity(SIDE.pow(3) * slabs.len());
    for slab in slabs {
        let metadata_path = root.join(&slab.metadata);
        let metadata_bytes = fs::read(&metadata_path)?;
        if let Some(expected) = &slab.metadata_sha256 {
            verify(&metadata_bytes, expected, &metadata_path)?;
        }
        let metadata: Metadata = serde_json::from_slice(&metadata_bytes)?;
        if metadata.payload_sha256 != slab.payload_sha256
            || metadata.voxel_sha256 != slab.voxel_sha256
        {
            return Err(io::Error::other(format!(
                "Cloud library and sidecar disagree at {}",
                metadata_path.display()
            ))
            .into());
        }
        let bytes = decode_atlas(&root.join(&slab.payload), &metadata)?;
        let spacing = metadata.extent_m / 128.0;
        let shape = Shape::new(
            [SIDE; 3],
            spacing,
            bytes
                .iter()
                .map(|&value| f64::from(value) / 255.0)
                .collect(),
        )?;
        write_footprint(footprints, &metadata, &shape);
        stack.extend_from_slice(&bytes);
    }
    let side = u32::try_from(SIDE)?;
    let depth = u32::try_from(SIDE * slabs.len())?;
    fs::write(source, encode_volume(side, side, depth, &stack))?;
    Ok(())
}

fn decode_atlas(path: &Path, metadata: &Metadata) -> Result<Vec<u8>, Box<dyn Error>> {
    let encoded = fs::read(path)?;
    verify(&encoded, &metadata.payload_sha256, path)?;
    let atlas =
        image::load_from_memory_with_format(&encoded, image::ImageFormat::Png)?.into_rgba8();
    let columns = metadata.tiles_per_row;
    if metadata.side != SIDE
        || columns == 0
        || !SIDE.is_multiple_of(columns)
        || usize::try_from(atlas.width())? != columns * SIDE
        || usize::try_from(atlas.height())? != SIDE * (SIDE / columns)
    {
        return Err(
            io::Error::other(format!("Unsupported cloud atlas: {}", path.display())).into(),
        );
    }
    // The atlas tiles advance along +Y. Its rows run towards -Z. Red alone carries density.
    let width = columns * SIDE;
    let pixels = atlas.into_raw();
    let mut voxels = Vec::with_capacity(SIDE.pow(3));
    for z in (0..SIDE).rev() {
        for y in 0..SIDE {
            let tile_x = y % columns;
            let tile_y = y / columns;
            let start = ((tile_y * SIDE + z) * width + tile_x * SIDE) * 4;
            voxels.extend(pixels[start..start + SIDE * 4].iter().step_by(4));
        }
    }
    verify(&voxels, &metadata.voxel_sha256, path)?;
    Ok(voxels)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded metres, unit axes and column integrals in the GPU footprint precision"
)]
fn write_footprint(out: &mut Vec<u8>, metadata: &Metadata, shape: &Shape) {
    for value in [metadata.extent_m, metadata.base_height_m]
        .into_iter()
        .chain(metadata.downwind_axis)
    {
        out.extend_from_slice(&(value as f32).to_le_bytes());
    }
    // The sidecar measures the condensation plane from the cube centre.
    let floor = -metadata.base_height_m - metadata.extent_m * 0.5;
    for &[integral, bottom, top] in shape.footprint().columns() {
        let column = if integral > 0.0 {
            [integral, bottom + floor, top + floor]
        } else {
            [0.0; 3]
        };
        for value in column {
            out.extend_from_slice(&(value as f32).to_le_bytes());
        }
    }
}

fn verify(bytes: &[u8], expected: &str, path: &Path) -> Result<(), io::Error> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{}: SHA-256 was {actual}, expected {expected}",
            path.display()
        )))
    }
}
