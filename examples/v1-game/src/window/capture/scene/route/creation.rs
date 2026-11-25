//! Mark the route's generation readiness and confirm the creation control.
use std::{
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thrum_render::runtime::DeviceSite;

use super::CreationProof;
use crate::host::Runtime;

pub(super) struct Ready {
    arm: CreationProof,
    generation: u64,
    native_creations: u64,
}

impl Ready {
    pub(super) fn mark(arm: CreationProof, host: &Runtime) -> Result<Self, String> {
        let report = host.creation_checkpoint();
        if !report.ready || report.frames == 0 {
            return Err("route readiness requires a prepared device and a cooked draw".to_owned());
        }
        let native_creations = report.native_creations.iter().map(|row| row.count).sum();
        let native_elapsed_ns = report
            .native_creations
            .iter()
            .map(|row| row.elapsed.as_nanos())
            .sum::<u128>();
        let ready = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?;
        writeln!(
            std::io::stderr().lock(),
            "THRUM_METAL_READY {} {} {native_creations} {native_elapsed_ns}",
            ready.as_nanos(),
            report.generation,
        )
        .map_err(|error| error.to_string())?;
        Ok(Self {
            arm,
            generation: report.generation,
            native_creations,
        })
    }

    pub(super) fn finish(
        self,
        site: &DeviceSite,
        host: &Runtime,
        directory: &Path,
    ) -> Result<(), String> {
        if self.arm == CreationProof::LatePipeline {
            prepare_late_pipeline(site);
        }
        let report = host.creation_checkpoint();
        if report.generation != self.generation {
            return Err("route changed its ready device generation".to_owned());
        }
        std::fs::write(directory.join("creation.txt"), report.to_string())
            .map_err(|error| error.to_string())?;
        let native_creations = report
            .native_creations
            .iter()
            .map(|row| row.count)
            .sum::<u64>();
        writeln!(
            std::io::stderr().lock(),
            "THRUM_METAL_FINISHED {} {} {}",
            report.frames,
            report.violations,
            native_creations - self.native_creations,
        )
        .map_err(|error| error.to_string())
    }
}

fn prepare_late_pipeline(site: &DeviceSite) {
    let module = site.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("late pipeline control"),
        source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn main() {}".into()),
    });
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("late pipeline control"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    drop(pipeline);
}
