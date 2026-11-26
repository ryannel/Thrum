//! Read the V1 Metal preparation launch inputs before opening the device.

use std::{env, io::Write as _, path::PathBuf};
use thrum_render::capabilities::{
    HardwareFeature, HardwareRequest,
    metallib::{ArchiveRequest, Preparation, PreparationPath},
};

#[expect(
    unsafe_code,
    reason = "the local cook's native output is trusted like the game executable"
)]
pub(super) fn options(hardware: &mut Vec<HardwareRequest>) -> Result<Option<Preparation>, String> {
    let path = |key| env::var_os(key).map(PathBuf::from);
    let store = path("THRUM_METALLIB_STORE");
    let manifest = path("THRUM_PIPELINE_MANIFEST");
    let archive = path("THRUM_PIPELINE_ARCHIVE");
    #[cfg(all(feature = "proving", feature = "creation-counters"))]
    let harvest = path("THRUM_PIPELINE_ARCHIVE_HARVEST");
    #[cfg(not(all(feature = "proving", feature = "creation-counters")))]
    let harvest: Option<PathBuf> = None;
    let forced = env::var("THRUM_PIPELINE_PREPARATION").ok();
    let strict = forced.as_deref() == Some("strict-archive");

    if let Some(output) = path("THRUM_METALLIB_HARVEST") {
        return Ok(Some(Preparation::harvest(output)));
    }
    let selected = match forced.as_deref() {
        Some("wgsl") => PreparationPath::Wgsl,
        Some("parallel") => PreparationPath::Parallel,
        Some("archive" | "strict-archive") => PreparationPath::Archive,
        Some(_) => return Err("unknown THRUM_PIPELINE_PREPARATION path".to_owned()),
        None if harvest.is_some() => PreparationPath::Archive,
        None if manifest.is_none() && archive.is_none() => {
            if let Some(store) = store {
                let parallel = env::var_os("THRUM_METALLIB_PARALLEL").is_some();
                // SAFETY: Only the explicitly selected local build store is used.
                return Ok(Some(unsafe { Preparation::trusted_store(store, parallel) }));
            }
            return Ok(env::var_os("THRUM_METALLIB_MANIFEST").map(|_| Preparation::default()));
        }
        None => {
            PreparationPath::preferred(store.is_some(), archive.is_some() && manifest.is_some())
        }
    };
    if cfg!(feature = "creation-counters") {
        let selected = match selected {
            PreparationPath::Wgsl => "wgsl",
            PreparationPath::Parallel => "parallel",
            PreparationPath::Archive => "archive",
        };
        drop(writeln!(
            std::io::stderr().lock(),
            "THRUM_METAL_PREPARATION {selected}"
        ));
    }
    let archive = if selected == PreparationPath::Archive {
        let manifest = manifest.ok_or("pipeline archive needs THRUM_PIPELINE_MANIFEST")?;
        let request = if let Some(output) = harvest {
            if env::var_os("THRUM_CAPTURE_SCENE").is_none()
                || env::var("THRUM_CREATION_PROOF").as_deref() != Ok("launch")
            {
                return Err("pipeline archive harvest needs the separate launch capture".to_owned());
            }
            ArchiveRequest::Harvest {
                path: output,
                manifest,
                omit: env::var("THRUM_PIPELINE_ARCHIVE_OMIT").ok(),
            }
        } else {
            ArchiveRequest::Read {
                path: archive.ok_or("archive mode needs THRUM_PIPELINE_ARCHIVE")?,
                manifest,
                strict,
            }
        };
        request_feature(
            hardware,
            HardwareFeature::MetalPipelineArchive,
            forced.is_some(),
        );
        Some(request)
    } else {
        None
    };
    let prepared = match selected {
        PreparationPath::Wgsl => Preparation::default(),
        PreparationPath::Parallel | PreparationPath::Archive => {
            let store = store.ok_or("cooked preparation needs THRUM_METALLIB_STORE")?;
            request_feature(
                hardware,
                HardwareFeature::MetalShaderPassthrough,
                forced.is_some(),
            );
            // SAFETY: Only the explicitly selected local build store is used.
            unsafe { Preparation::trusted_store(store, true) }
        }
    };
    Ok(Some(prepared.with_archive(archive)))
}

fn request_feature(hardware: &mut Vec<HardwareRequest>, feature: HardwareFeature, required: bool) {
    if !hardware.iter().any(|request| request.feature == feature) {
        hardware.push(HardwareRequest { feature, required });
    }
}
