#[path = "exact-support.rs"]
mod support;
fn main() { support::prove_kinds(); println!("PASS: every float bit in 36 kind/scope/size/generation combinations, three mips each"); }
