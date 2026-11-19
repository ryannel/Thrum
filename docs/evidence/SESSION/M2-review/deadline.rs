#[path = "../../../../crates/thrum-network/tests/support.rs"]
mod support;

use support::{Match, local};
use thrum_network::session::Controls;

fn main() {
    for hello_ms in [3_999_u64, 4_001] {
        let mut fixture = Match::new(local(), 1, None, false);
        fixture.memory.advance(0);
        fixture.host.advance(3_999_000, 0);
        let at = hello_ms * 1_000;
        fixture.clients[0].advance(at, |_| Controls::axes([0.0; 3])).unwrap();
        fixture.memory.advance(hello_ms);
        fixture.host.advance(at, 0);
        fixture.memory.advance(hello_ms);
        fixture.clients[0].advance(at, |_| Controls::axes([0.0; 3])).unwrap();
        println!("deadline_ms=4000 hello_ms={hello_ms} state={:?} slot={:?}",
                 fixture.clients[0].state(), fixture.clients[0].slot());
    }
}
