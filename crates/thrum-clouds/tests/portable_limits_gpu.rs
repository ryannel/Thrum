//! Both cloud products on Metal with WebGPU's default limits.
mod support;

#[test]
#[cfg(target_os = "macos")]
fn weather_and_sun_path_work_with_default_limits() {
    support::conformance("weather_and_sun_path_work_with_default_limits", true);
}
