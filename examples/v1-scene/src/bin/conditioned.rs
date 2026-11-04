//! Run the same V1 world with four delayed, lossy in-memory peer inputs.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::print_stdout, reason = "the diagnostic CLI reports its run")]
#[expect(clippy::print_stderr, reason = "the diagnostic CLI reports failures")]
fn main() {
    match v1_scene::conditioned::run() {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("v1-conditioned: {error}");
            std::process::exit(1);
        }
    }
}
