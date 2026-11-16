//! Metal conformance of the GPU weather map against the CPU authority.
mod support;

#[test]
#[cfg(target_os = "macos")]
fn columns_and_integer_cells_match_the_cpu_at_their_written_tick() {
    support::conformance(false, false);
}
