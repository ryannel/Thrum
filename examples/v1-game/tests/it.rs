//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

pub mod support;

mod capture_display_phase_gpu;
mod clouds_display_phase_gpu;
mod clouds_frame_contract_gpu;
mod clouds_shade_gpu;
mod clouds_weather_gpu;
mod cooked_rows_gpu;
mod effect_after_opaque_gpu;
mod effect_cadence_gpu;
mod effect_display_phase_gpu;
mod effect_resume_gpu;
mod effect_state_gpu;
mod exit_scene_gpu;
mod host_gpu;
mod lent_products_gpu;
mod sprite_launch;
mod sprite_scene_3d_control;
mod surface_gpu;
mod temporal_volume_gpu;
