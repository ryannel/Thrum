//! `thrum-cook`: authoring command line for the G5 content-addressed loop.
//!
//! - `thrum-cook demo <source>` writes the deterministic demo sources;
//! - `thrum-cook cook <source> <store> [--final]` runs one incremental cycle;
//! - `thrum-cook watch <source> <store> [--serve <address>]` cooks on every change and optionally
//!   serves the store over HTTP (use a loopback address unless exposure is intended).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use thrum_asset::{AssetStore, Profile};
use thrum_cook::{
    CookConfig, CookReport, Cooker, DevServer, Durability, Publications, Quality, demo, watch,
};

const USAGE: &str = "usage: thrum-cook demo <source> | cook <source> <store> [--final] | watch <source> <store> [--serve <address>]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |index: usize| args.get(index).map(PathBuf::from).ok_or(USAGE);
    let flag = |name: &str| args.iter().position(|value| value == name);
    let config = || -> Result<CookConfig, &'static str> {
        Ok(CookConfig {
            source: arg(1)?,
            store: arg(2)?,
            profiles: Profile::ALL.to_vec(),
            quality: if flag("--final").is_some() {
                Quality::Final
            } else {
                Quality::Fast
            },
            durability: Durability::Process,
        })
    };
    let mut out = std::io::stdout().lock();
    match args.first().map(String::as_str) {
        Some("demo") => {
            let source = arg(1)?;
            std::fs::create_dir_all(source.join("meshes"))?;
            std::fs::create_dir_all(source.join("textures"))?;
            demo::write_grid_gltf(&source.join("meshes"), "terrain", 64, 0.3)?;
            demo::write_checker_png(&source.join("textures/checker.png"), 256, [200, 60, 40])?;
            demo::write_cloud_volume(&source.join("cloud.tvol"), 64, 1)?;
            writeln!(out, "demo sources written to {}", source.display())?;
        }
        Some("cook") => {
            let mut cooker = Cooker::open(config()?)?;
            let report = cooker.cook_all()?;
            describe(&mut out, &report)?;
        }
        Some("watch") => {
            let config = config()?;
            let publications = Arc::new(Publications::default());
            let _server = match flag("--serve").and_then(|index| args.get(index + 1)) {
                Some(address) => {
                    let server = DevServer::start(
                        address,
                        AssetStore::new(&config.store),
                        Arc::clone(&publications),
                    )?;
                    writeln!(out, "serving http://{}", server.address())?;
                    Some(server)
                }
                None => None,
            };
            let source = config.source.clone();
            let mut cooker = Cooker::open(config)?;
            publications.announce(cooker.generation());
            let stop = AtomicBool::new(false);
            watch(&mut cooker, &source, &stop, |event| {
                let mut out = std::io::stdout().lock();
                match &event.report {
                    Ok(report) => {
                        if let Some(head) = report.published {
                            publications.announce(head.generation);
                        }
                        describe(&mut out, report).ok();
                        writeln!(
                            out,
                            "  {} paths, event to publish {:?}",
                            event.paths,
                            event.first_event.elapsed()
                        )
                        .ok();
                    }
                    Err(error) => {
                        writeln!(out, "cycle failed: {error}").ok();
                    }
                }
            })?;
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn describe(out: &mut impl Write, report: &CookReport) -> std::io::Result<()> {
    writeln!(
        out,
        "scan {:?} (reused {} hashed {} unstable {}), cooked {} reused {} in {:?}, publish {:?} -> {}",
        report.scan_time,
        report.scan.reused,
        report.scan.hashed,
        report.scan.unstable,
        report.cooked,
        report.reused,
        report.cook_time,
        report.publish_time,
        report.published.map_or_else(
            || "unchanged".to_owned(),
            |head| format!("generation {}", head.generation)
        ),
    )?;
    for (asset, reason) in &report.failed {
        writeln!(out, "  failed {asset}: {reason}")?;
    }
    for (asset, profile, reason) in &report.refused {
        writeln!(out, "  refused {asset} ({}): {reason}", profile.name())?;
    }
    Ok(())
}
