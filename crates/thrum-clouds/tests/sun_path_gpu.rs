//! Sun-path conformance and far-value reduction on mac-metal.
mod support;

#[test]
#[cfg(target_os = "macos")]
fn coarse_sun_paths_match_the_cpu_reference() {
    support::conformance("coarse_sun_paths_match_the_cpu_reference", false);
}
