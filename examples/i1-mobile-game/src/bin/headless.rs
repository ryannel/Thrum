//! CPU-only executable entry for the bundled game and native build proof.

fn main() -> Result<(), &'static str> {
    i1_mobile_game::headless_probe()?;
    println!("thrum-headless-v1 taps=1 results=1 events=1");
    Ok(())
}
