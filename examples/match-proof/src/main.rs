//! Native and simulator entry point for headless session checks.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let packet = match arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["identity"] => serde_json::json!({
            "crowd": match_proof::identity(false),
            "mixed": match_proof::identity(true),
        }),
        ["capture", count, mixed, ticks] => {
            let ticks = ticks
                .split(',')
                .map(str::parse)
                .collect::<Result<Vec<_>, _>>()?;
            serde_json::from_str(&match_proof::capture(
                count.parse()?,
                mixed.parse()?,
                &ticks,
            ))?
        }
        ["restore", count, mixed, path, last] => {
            let image = std::fs::read(path)?;
            match match_proof::restore(count.parse()?, mixed.parse()?, &image, last.parse()?) {
                Ok(record) => serde_json::from_str(&record)?,
                Err(thrum_core::sim::WorldImageError::RuleSet) => {
                    serde_json::json!({"refusal": "rule_set"})
                }
                Err(error) => return Err(error.into()),
            }
        }
        _ => return Err("expected identity, capture or restore".into()),
    };
    let mut packet: serde_json::Value = packet;
    packet["pid"] = serde_json::json!(std::process::id());
    serde_json::to_writer(std::io::stdout().lock(), &packet)?;
    Ok(())
}
