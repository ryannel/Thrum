//! Public error inventory and wrapped-source contract.

#[cfg(not(target_arch = "wasm32"))]
mod error_types {
    mod lane;
}

use std::error::Error;
use thrum_core::{ClockError, sim::*};

fn assert_error<T: Error>() {}

#[test]
fn public_errors_implement_error() {
    assert_error::<ClockError>();
    assert_error::<InputFrameError>();
    assert_error::<RegistrationError>();
    assert_error::<ScheduleError>();
    assert_error::<RegisteredWorldError>();
    assert_error::<EditError>();
    assert_error::<RuntimeError>();
    assert_error::<TickError<std::io::Error>>();
    assert_error::<HistoryError<std::io::Error>>();
    assert_error::<ClockedWorldError<std::io::Error>>();
    assert_error::<PublicationBusy>();
    #[cfg(not(target_arch = "wasm32"))]
    assert_error::<WorldLaneFault>();
}

#[test]
fn wrapped_simulation_errors_preserve_sources() {
    let error = HistoryError::<std::io::Error>::Tick(TickError::Edit(EditError::World(
        RegisteredWorldError::Capacity,
    )));
    let tick = error.source().expect("tick source");
    assert!(tick.is::<TickError<std::io::Error>>());
    let edit = tick.source().expect("edit source");
    assert!(edit.is::<EditError>());
    assert!(
        edit.source()
            .expect("world source")
            .is::<RegisteredWorldError>()
    );
    let runtime = RuntimeError::Schedule(ScheduleError::Cycle);
    assert!(
        runtime
            .source()
            .expect("schedule source")
            .is::<ScheduleError>()
    );
    let game = TickError::Game(std::io::Error::other("game refused"));
    assert!(game.source().expect("game source").is::<std::io::Error>());
    let clock = ClockedWorldError::<std::io::Error>::Clock(ClockError::PendingTicks);
    assert!(clock.source().expect("clock source").is::<ClockError>());
    #[cfg(not(target_arch = "wasm32"))]
    {
        let start =
            WorldLaneFault::Start(std::sync::Arc::new(std::io::Error::other("thread limit")));
        assert!(
            start
                .source()
                .expect("thread source")
                .is::<std::io::Error>()
        );
    }
}
