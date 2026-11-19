#[path = "../../../../crates/thrum-network/tests/support.rs"]
mod support;

use support::{Match, local};
use thrum_network::session::World;

fn main() {
    let mut fixture = Match::new(local(), 1, None, false);
    fixture.ready();
    while fixture.host.world().tick() < 25 {
        fixture.neutral(100);
    }
    fixture.memory.stall(true);
    loop {
        let at = fixture.now;
        fixture.neutral(100);
        if fixture.clients[0].playout(at).unwrap().rate == 0.0 {
            let position = fixture.clients[0].playout(at).unwrap().position;
            fixture.memory.stall(false);
            for _ in 0..80 {
                fixture.memory.advance(fixture.now / 1_000);
                fixture.host.advance(fixture.now, 100);
                fixture.now += 2_500;
            }
            let resumed = fixture.now;
            let last_draw = fixture.clients[0].playout(resumed - 1).unwrap().position;
            fixture.neutral(100);
            let client = &mut fixture.clients[0];
            let recorded = client.take_stall().unwrap();
            println!("start_us={at} resume_us={resumed} observed_hold_us={} position={position} last_draw={last_draw} resumed_position={} reported_stall_us={} skip={:?}",
                     resumed - at, client.playout(resumed).unwrap().position, recorded.duration_us, client.take_skip());
            break;
        }
    }
}
