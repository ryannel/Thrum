//! A render and a compute pipeline round-trip through the public wgpu cache API.
#![cfg(target_os = "macos")]

use std::{borrow::Cow, env, fs, path::Path, process::Command};
use thrum_test_support::Scratch;
use wgpu::hal::metal::{ArchiveOutcome, with_archive_policy};

const MODE: &str = "THRUM_TEST_PIPELINE_ARCHIVE_MODE";
const FILE: &str = "THRUM_TEST_PIPELINE_ARCHIVE_FILE";
const TEST: &str = "metal_pipeline_cache_round_trip";
const SHADER: &str = "
@compute @workgroup_size(1) fn compute_main() {}
@vertex fn vertex_main() -> @builtin(position) vec4f {
    return vec4f(0.0, 0.0, 0.0, 1.0);
}
@fragment fn fragment_main() -> @location(0) vec4f {
    return vec4f(1.0);
}
";

#[test]
fn metal_pipeline_cache_round_trip() {
    if let Ok(mode) = env::var(MODE) {
        let file = env::var_os(FILE).expect("archive file");
        child(&mode, Path::new(&file));
        return;
    }
    let scratch = Scratch::new("public-pipeline-archive");
    let path = scratch.join("cache");
    for mode in ["harvest", "read", "wrong-adapter"] {
        let output = Command::new(env::current_exe().expect("test executable"))
            .args(["--exact", TEST, "--nocapture"])
            .env(MODE, mode)
            .env(FILE, &path)
            .output()
            .expect("fresh test process");
        assert!(
            output.status.success(),
            "{mode}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn child(mode: &str, file: &Path) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("Metal adapter");
    assert!(adapter.features().contains(wgpu::Features::PIPELINE_CACHE));
    let (device, _) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::PIPELINE_CACHE,
        ..Default::default()
    }))
    .expect("Metal device with pipeline cache");
    let mut data = (mode != "harvest").then(|| fs::read(file).expect("saved cache"));
    if mode == "wrong-adapter" {
        // The pinned wgpu header has a backend byte at offset 16 and starts
        // its adapter key at 17. Leave the native archive bytes untouched.
        data.as_mut().expect("cache from get_data")[17] ^= 1;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        drop(cache(&device, data.as_deref()));
        let failure = pollster::block_on(scope.pop()).expect("adapter mismatch must be rejected");
        assert!(
            failure.to_string().contains("different device"),
            "{failure}"
        );
        return;
    }
    let cache = cache(&device, data.as_deref());
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
    });
    let expected = if mode == "harvest" {
        ArchiveOutcome::Unused
    } else {
        ArchiveOutcome::Hit
    };
    let (_, compute) = with_archive_policy(true, || {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("archive compute"),
            layout: None,
            module: &module,
            entry_point: Some("compute_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: Some(&cache),
        })
    });
    assert_eq!(compute, expected);
    let (_, render) = with_archive_policy(true, || {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("archive render"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            multiview_mask: None,
            cache: Some(&cache),
        })
    });
    assert_eq!(render, expected);
    if mode == "harvest" {
        let bytes = cache.get_data().expect("public pipeline cache data");
        assert!(bytes.starts_with(b"WGPUPLCH") && bytes.len() > 64);
        fs::write(file, bytes).expect("persist archive");
    }
}

#[expect(
    unsafe_code,
    reason = "exercise the public cache validation and Metal round trip"
)]
fn cache(device: &wgpu::Device, data: Option<&[u8]>) -> wgpu::PipelineCache {
    // SAFETY: Data comes from get_data on the same adapter. The negative control
    // changes only core's adapter-key field, which core checks before native import.
    unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("public archive witness"),
            data,
            fallback: false,
        })
    }
}
