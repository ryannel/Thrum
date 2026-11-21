//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod a1_schedule;
mod clock;
mod clock_spans;
mod deferred_undo_cost;
mod display_phase;
mod g2_edit_ownership;
mod g2_runtime;
mod g2s_chunked_kernel;
mod profile_resources;
mod rewind_publication;
// Two test files that sit beside the profile game they share, in `support/`.
mod support {
    mod profile_game;
    mod profile_refusal;
    mod world_digest;
}
mod world_image;
mod world_lifecycle;
