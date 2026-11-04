//! Run the same V1 world with four delayed, lossy in-memory peer inputs.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
#[expect(clippy::print_stdout, reason = "the diagnostic CLI reports its run")]
#[expect(clippy::print_stderr, reason = "the diagnostic CLI reports failures")]
fn main() {
    use v1_scene::conditioned::ConditionedScenario;
    let scenario = match std::env::args().nth(1).as_deref() {
        None => ConditionedScenario::Standard,
        Some("--reconnect") => ConditionedScenario::DrainedReconnect,
        Some(_) => {
            eprintln!("usage: conditioned [--reconnect]");
            std::process::exit(2);
        }
    };
    match v1_scene::conditioned::run(scenario) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("v1-conditioned: {error}");
            std::process::exit(1);
        }
    }
}
