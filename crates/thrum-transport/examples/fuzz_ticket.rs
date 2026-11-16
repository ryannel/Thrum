//! Stdin fuzz target for a join ticket's text, which a player pastes and so is untrusted. Compile once, then run
//! the binary under an external stdin fuzzer; a ticket that parses must print back as the same text.

use std::io::Read;

use thrum_transport::{TICKET_LIMIT, Ticket};

fn main() -> std::io::Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(TICKET_LIMIT as u64 * 4)
        .read_to_end(&mut bytes)?;
    if let Ok(text) = std::str::from_utf8(&bytes)
        && let Ok(ticket) = text.parse::<Ticket>()
    {
        assert_eq!(ticket.to_string(), text);
    }
    Ok(())
}
