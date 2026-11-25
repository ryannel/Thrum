//! The reference renderer driven by the same session as the socket proofs.

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(target_os = "macos")]
mod native;
#[cfg(target_os = "macos")]
pub use native::{client, host};

use std::{num::NonZeroU32, sync::Arc, time::Duration};

use crowd_2d_view::{CrowdScene, NoEdits, prepare_crowd, proof::ORDER_BAND};
use serde_json::{Value, json};
use thrum_app::{Game, GameError, Gpu, Prepared};
use thrum_controls::Controls;
use thrum_core::{
    ClockSample,
    sim::{DrivenAdvance, DrivenWorld, PublicationPage, SnapshotId, WorldDriver, WorldPublication},
};
use thrum_network::session::{Client, ClientState, Playout, World};
use thrum_render::runtime::FrameHost;
use thrum_transport::{Profile, Ticket};

use crate::world::{CrowdWorld, NoActions};

const CAP: NonZeroU32 = NonZeroU32::new(8).expect("catch-up cap");
const BODIES: u32 = 2_000;

type Host = FrameHost<DrivenWorld<Driver>, CrowdScene>;

struct WindowMatch {
    controls: Controls,
    driver: Option<Driver>,
}

impl WindowMatch {
    fn new(driver: Driver) -> Self {
        Self {
            controls: Controls::new(),
            driver: Some(driver),
        }
    }
}

impl Game for WindowMatch {
    type World = DrivenWorld<Driver>;
    type Scene = CrowdScene;
    type Edits = NoEdits;
    type Readback = ();

    async fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        _: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<Self::World, Self::Scene>, GameError> {
        let mut driver = self
            .driver
            .take()
            .ok_or_else(|| rejected("driver already prepared"))?;
        driver.record(json!({
            "event":"renderer", "backend":format!("{:?}", gpu.capabilities.adapter().backend),
            "adapter":gpu.capabilities.adapter().name, "extent":gpu.surface.extent,
            "clock":"driven",
        }))?;
        let mut host = prepare_crowd(
            gpu,
            DrivenWorld::new(driver, CAP),
            BODIES,
            false,
            ORDER_BAND,
        )
        .await?;
        host.set_timing_requested(false);
        Ok(Prepared::inline(host))
    }

    fn controls(&self) -> &Controls {
        &self.controls
    }

    fn controls_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }

    fn take_edits(&mut self) -> (NoEdits, Option<bool>) {
        (NoEdits, None)
    }

    fn replay(&self) -> NoEdits {
        NoEdits
    }

    fn apply(host: &mut Host, _: NoEdits) -> Result<(), GameError> {
        record_frame(host)?;
        #[cfg(target_os = "macos")]
        native::capture(host)?;
        Ok(())
    }

    fn readback(_: &Host) {}

    fn observe(&mut self, (): ()) {}
}

fn rejected(error: impl std::fmt::Display) -> GameError {
    GameError::Rejected(error.to_string())
}

fn micros(now: Duration) -> u64 {
    u64::try_from(now.as_micros()).expect("proof clock fits in microseconds")
}

fn record_frame(host: &mut Host) -> Result<(), GameError> {
    let Some(sample) = host
        .last_submitted_clock()
        .filter(|sample| sample.surface_presented)
    else {
        return Ok(());
    };
    let driver = host.world_mut().driver_mut();
    if driver.last_frame == Some(sample.submission_order) {
        return Ok(());
    }
    driver.last_frame = Some(sample.submission_order);
    let display_us = micros(sample.display_time);
    // Feed the submitted edge into the playout clock before its next advance.
    driver.peer.observe_display(display_us);
    driver.record(json!({
        "event": "frame", "submission": sample.submission_order, "tick": sample.tick,
        "alpha": sample.presentation_alpha, "display_us": display_us,
        "wall_us": crate::sockets::unix_us(),
    }))
}

struct Replica {
    session: Client<CrowdWorld, NoActions>,
    prior_state: ClientState,
    prior_epoch: SnapshotId,
}

impl Replica {
    fn connect(ticket: &Ticket) -> Result<Self, GameError> {
        let profile = Profile::default();
        #[cfg(target_os = "macos")]
        let link = thrum_transport::native::connect(ticket, profile).map_err(rejected)?;
        #[cfg(target_arch = "wasm32")]
        let link = thrum_transport::browser::connect(ticket, profile).map_err(rejected)?;
        let world = CrowdWorld::presented(BODIES, 10);
        let prior_epoch = world.window_endpoint().0;
        Ok(Self {
            session: Client::adaptive(world, NoActions, link, 0),
            prior_state: ClientState::Connecting,
            prior_epoch,
        })
    }

    fn advance(
        &mut self,
        now: u64,
        cap: NonZeroU32,
        rows: &mut Vec<Value>,
    ) -> Result<(Playout, u32), GameError> {
        use thrum_network::session::Controls as Input;

        let held = self.session.held();
        self.session
            .advance_with_step_budget(now, u8::try_from(cap.get()).unwrap_or(u8::MAX), |tick| {
                Input::axes(crate::seeded_axes(71, 1, tick))
            })
            .map_err(rejected)?;
        while self.session.result().is_some() {}

        let endpoint = self.session.world().window_endpoint().0;
        if endpoint.epoch != self.prior_epoch.epoch {
            let (tick, digest, _) = self
                .session
                .world()
                .restored
                .expect("new epoch restores an image");
            rows.push(json!({"event":"restore", "tick":tick, "digest":digest.to_string(), "clock_us":now}));
            self.prior_epoch = endpoint;
        }
        if self.session.state() != &self.prior_state {
            self.prior_state = self.session.state().clone();
            rows.push(json!({"event":"state", "tick":endpoint.tick, "state":format!("{:?}", self.prior_state),
                "clock_us":now}));
        }
        if let Some(stall) = self.session.take_stall() {
            rows.push(
                json!({"event":"stall", "position":stall.position, "duration_us":stall.duration_us,
                             "work_cap":stall.work_cap, "clock_us":now}),
            );
        }
        if let Some(skip) = self.session.take_skip() {
            rows.push(
                json!({"event":"skip", "first":skip.first, "last":skip.last, "clock_us":now}),
            );
        }
        let playout = self
            .session
            .playout(now)
            .unwrap_or_else(|| held_playout(endpoint.tick));
        if self.session.held() > held + 1 {
            rows.push(
                json!({"event":"burst", "first":held+1, "last":self.session.held(), "clock_us":now,
                             "position":playout.position,"rate":playout.rate}),
            );
        }
        let period = self.session.settings().unwrap_or_default().tick_us;
        Ok((playout, period))
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the proof uses a few thousand ticks"
)]
fn held_playout(tick: u64) -> Playout {
    Playout {
        position: tick as f64,
        rate: 0.0,
    }
}

enum Peer {
    #[cfg(target_os = "macos")]
    Authority(Box<native::Authority>),
    Replica(Box<Replica>),
}

impl Peer {
    fn observe_display(&self, display_us: u64) {
        match self {
            #[cfg(target_os = "macos")]
            Self::Authority(_) => {}
            Self::Replica(replica) => {
                replica.session.playout(display_us);
            }
        }
    }

    fn world(&self) -> &CrowdWorld {
        match self {
            #[cfg(target_os = "macos")]
            Self::Authority(authority) => authority.session.world(),
            Self::Replica(replica) => replica.session.world(),
        }
    }
}

struct Driver {
    peer: Peer,
    #[cfg(target_arch = "wasm32")]
    rows: std::cell::RefCell<Vec<Value>>,
    last_frame: Option<u64>,
    #[cfg(target_os = "macos")]
    native: native::Output,
}

impl Driver {
    #[cfg_attr(
        target_os = "macos",
        expect(
            clippy::needless_pass_by_value,
            reason = "the browser retains the same row before uploading"
        )
    )]
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::unnecessary_wraps,
            reason = "writing the native record can fail"
        )
    )]
    fn record(&mut self, row: Value) -> Result<(), GameError> {
        #[cfg(target_os = "macos")]
        self.native.write(&row).map_err(rejected)?;
        #[cfg(target_arch = "wasm32")]
        self.rows.get_mut().push(row);
        Ok(())
    }
}

impl WorldDriver for Driver {
    type Control = ();
    type Page = Arc<PublicationPage>;
    type Error = GameError;

    fn validate_control((): ()) -> Result<(), Self::Error> {
        Ok(())
    }
    fn same_control((): (), (): ()) -> bool {
        true
    }
    fn apply_control(&mut self, (): ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn endpoint(&self) -> (SnapshotId, u64) {
        self.peer.world().window_endpoint()
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "the proof uses a few thousand ticks"
    )]
    fn advance(&mut self, now: Duration, cap: NonZeroU32) -> Result<DrivenAdvance, Self::Error> {
        let clock_us = micros(now);
        let wall_us = crate::sockets::unix_us();
        let mut rows = Vec::new();
        let (playout, period) = match &mut self.peer {
            #[cfg(target_os = "macos")]
            Peer::Authority(authority) => authority.advance(clock_us, cap, &mut rows)?,
            Peer::Replica(replica) => replica.advance(clock_us, cap, &mut rows)?,
        };
        let world = self.peer.world();
        let tick = world.tick();
        let steps = world.window_steps();
        let completed_ticks = u32::try_from(steps.len()).expect("bounded commits");
        let digests = std::mem::take(&mut *world.digests.borrow_mut());
        let fraction = if tick == 0 {
            1.0
        } else {
            (playout.position - (tick - 1) as f64).clamp(0.0, 1.0)
        };
        rows.push(json!({"event":"advance", "clock_us":clock_us, "tick":tick,
                         "steps":steps, "position":playout.position, "rate":playout.rate}));
        for (tick, digest) in digests {
            rows.push(json!({"event":"digest","tick":tick,"digest":digest.to_string(),"clock_us":clock_us}));
        }
        for mut row in rows {
            row["wall_us"] = json!(wall_us);
            self.record(row)?;
        }
        let presentation = ClockSample::driven(
            now,
            fraction,
            1_000_000.0 * playout.rate / f64::from(period),
        )
        .ok_or_else(|| rejected("invalid session phase"))?;
        Ok(DrivenAdvance {
            completed_ticks,
            presentation,
            next_due: Duration::from_millis(2),
        })
    }

    fn publication(&mut self) -> Result<Option<WorldPublication<Self::Page>>, Self::Error> {
        Ok(self
            .peer
            .world()
            .window_page()
            .map(|page| WorldPublication {
                id: page.id(),
                timeline: page.timeline(),
                page,
            }))
    }
}
