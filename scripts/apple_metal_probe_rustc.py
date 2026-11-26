#!/usr/bin/env python3
"""Instrument the locked wgpu-core and wgpu-hal boundaries in a disposable build."""
import os
from pathlib import Path
import shutil
import sys


def once(source, anchor, replacement):
    if source.count(anchor) != 1:
        raise ValueError(f"wgpu-hal changed around {anchor!r}")
    return source.replace(anchor, replacement)


def bracket(source, first, last, kind):
    start = source.index(first)
    end = source.index(last, start) + len(last)
    body = source[start:end]
    return source[:start] + f'thrum_probe("{kind}", || {body})' + source[end:]


TIMER = '''
fn thrum_probe<T>(stage: &str, call: impl FnOnce() -> T) -> T {
    let start = std::time::Instant::now();
    let result = call();
    let elapsed = start.elapsed().as_nanos();
    std::eprintln!("THRUM_METAL_STAGE {stage} {elapsed}");
    result
}
'''


def instrument_hal(device):
    source = device.read_text()
    source = bracket(source, "naga::back::pipeline_constants::process_overrides(", "stage.constants,\n                )", "wgsl_specialize")
    source = bracket(source, "naga::back::msl::write_string(", "&pipeline_options,\n                )", "wgsl_to_msl")
    source = bracket(source, "self\n                    .shared\n                    .device\n                    .newLibraryWithSource_options_error(", "Some(&options),\n                    )", "msl_compile")
    source = bracket(source, "super::library_from_metallib::new_library_from_metallib_bytes(", "file,\n        )", "metallib_load")
    source = bracket(source, "match descriptor {\n                MetalGenericRenderPipelineDescriptor::Standard", "\n            }", "pipeline_state")
    source = bracket(source, "super::archive::create_pipeline(\n                desc.cache,", "\n            )", "pipeline_state")
    source = once(source, "        self.counters.shader_modules.add(1);", '''        thrum_probe("shader_module", || self.counters.shader_modules.add(1));''')
    source = once(
        source,
        "            let descriptor = MTLComputePipelineDescriptor::new();",
        """            if let Some(label) = desc.label.filter(|label| label.starts_with("(wgpu internal) ")) {
                std::eprintln!("THRUM_METAL_INTERNAL {label}");
            }
            let descriptor = MTLComputePipelineDescriptor::new();""",
    )
    device.write_text(source + TIMER)


def instrument_core(resource):
    source = resource.read_text()
    source = once(source, "frontend.parse(&code)", 'thrum_probe("wgsl_parse", || frontend.parse(&code))')
    source = bracket(source, "create_validator(\n            self.features,", ".validate(&module)", "wgsl_validate")
    resource.write_text(source + TIMER)


def main():
    rustc, *args = sys.argv[1:]
    crate = args[args.index("--crate-name") + 1] if "--crate-name" in args else None
    if crate in ("wgpu_hal", "wgpu_core"):
        manifest = Path(os.environ["CARGO_MANIFEST_DIR"])
        if manifest.name != crate.replace("_", "-") + "-30.0.1":
            raise ValueError(f"update the Metal boundary probe for {manifest.name}")
        # Copy only our scratch input. Cargo.lock and the shared registry stay untouched.
        probe = Path(os.environ["THRUM_METAL_PROBE_SOURCE"]) / crate
        shutil.copytree(manifest / "src", probe / "src", dirs_exist_ok=True)
        if crate == "wgpu_hal":
            instrument_hal(probe / "src/metal/device.rs")
        else:
            instrument_core(probe / "src/device/resource.rs")
        source = next(index for index, arg in enumerate(args) if arg.endswith("/src/lib.rs"))
        args[source] = str(probe / "src/lib.rs")
    os.execvp(rustc, [rustc, *args])


if __name__ == "__main__":
    main()
