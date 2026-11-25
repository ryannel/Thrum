use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    net::{Ipv4Addr, SocketAddr},
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crowd_2d_view::proof;
use serde_json::{Value, json};
use thrum_app::{
    Config, GameError,
    capture_writer::{ImageWriter, SpoolEncoding},
};
use thrum_network::session::{HostState, Playout, Settings, World};
use thrum_render::runtime::{CaptureLimits, SubmittedImage};
use thrum_transport::{Profile, Ticket, native};

use super::{BODIES, Driver, Host, NoActions, Peer, Replica, WindowMatch, rejected};
use crate::world::CrowdWorld;

pub(super) struct Authority {
    pub(super) session: thrum_network::session::Host<CrowdWorld, NoActions>,
    transport: native::Host,
    control: PathBuf,
    end: u64,
    staged: u64,
    offered: bool,
}

impl Authority {
    pub(super) fn advance(
        &mut self,
        now: u64,
        cap: NonZeroU32,
        rows: &mut Vec<Value>,
    ) -> Result<(Playout, u32), GameError> {
        if !self.offered
            && let Some(ticket) = self.transport.ticket()
        {
            let settings = Settings::default();
            rows.push(
                json!({"event":"host", "ticket":ticket.to_string(), "pid":std::process::id(),
                "tick_us":settings.tick_us, "catch_up":settings.catch_up, "behind":settings.behind,
                "absence":settings.absence, "digest_interval":settings.digest_interval,
                "playout_catch_up":settings.playout_catch_up, "bodies":BODIES}),
            );
            self.offered = true;
        }
        while let Some(link) = self.transport.accept() {
            self.session.connect(link, now).map_err(rejected)?;
        }
        if self.session.state() == HostState::Lobby
            && fs::read_to_string(&self.control).is_ok_and(|command| command.trim() == "run")
        {
            self.session.start(now);
            rows.push(json!({"event":"start", "clock_us":now}));
        }
        let through = self
            .end
            .min(self.session.world().tick() + u64::from(cap.get()));
        if self.session.state() == HostState::Running {
            for tick in self.staged + 1..=through {
                self.session
                    .stage_local(tick, crate::seeded_axes(71, 0, tick), [])
                    .map_err(rejected)?;
                self.staged = tick;
            }
        }
        self.session.advance(now, through);
        if self.session.state() == HostState::Running && self.session.world().tick() == self.end {
            self.session.stop();
            rows.push(json!({"event":"finish", "clock_us":now, "tick":self.end}));
        }
        Ok((self.session.playout(now), Settings::default().tick_us))
    }
}

pub(super) struct Output {
    rows: Option<SyncSender<Value>>,
    writer: Option<JoinHandle<()>>,
    control: Option<PathBuf>,
    command: Arc<AtomicU8>,
    capture: Option<ImageWriter<SubmittedImage>>,
    stopped_capture: bool,
}

const RECORD: u8 = 1;
const STOP: u8 = 2;

impl Output {
    #[expect(
        clippy::print_stderr,
        reason = "surface worker I/O failures in the proof process log"
    )]
    fn new(path: &Path, control: Option<PathBuf>) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let (rows, incoming) = mpsc::sync_channel(1024);
        let command = Arc::new(AtomicU8::new(0));
        let control_path = control.clone();
        let command_out = Arc::clone(&command);
        let writer = thread::Builder::new()
            .name("match-record".into())
            .spawn(move || {
                if let Err(error) =
                    write_record(file, &incoming, control_path.as_deref(), &command_out)
                {
                    eprintln!("match record writer: {error}");
                }
            })?;
        Ok(Self {
            rows: Some(rows),
            writer: Some(writer),
            control,
            command,
            capture: None,
            stopped_capture: false,
        })
    }

    // Evidence must retain every row. An overloaded writer fails the proof
    // without waiting in the render actor or silently dropping observations.
    pub(super) fn write(&self, row: Value) -> std::io::Result<()> {
        self.rows
            .as_ref()
            .expect("writer sender is present until shutdown")
            .try_send(row)
            .map_err(|error| match error {
                TrySendError::Full(_) => std::io::Error::other("match record queue full"),
                TrySendError::Disconnected(_) => {
                    std::io::Error::other("match record writer stopped")
                }
            })
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        drop(self.rows.take());
        if let Some(writer) = self.writer.take() {
            drop(writer.join());
        }
    }
}

fn write_record(
    file: File,
    incoming: &Receiver<Value>,
    control: Option<&Path>,
    command: &AtomicU8,
) -> std::io::Result<()> {
    let mut rows = BufWriter::new(file);
    let interval = Duration::from_millis(8);
    let mut next = Instant::now() + interval;
    loop {
        match incoming.recv_timeout(next.saturating_duration_since(Instant::now())) {
            Ok(row) => {
                serde_json::to_writer(&mut rows, &row)?;
                rows.write_all(b"\n")?;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return rows.flush(),
        }
        if Instant::now() >= next {
            rows.flush()?;
            if command.load(Ordering::Relaxed) != STOP
                && let Some(text) = control.and_then(|path| fs::read_to_string(path).ok())
            {
                match text.trim() {
                    "record" => command.store(RECORD, Ordering::Relaxed),
                    "stop" => command.store(STOP, Ordering::Relaxed),
                    _ => {}
                }
            }
            next = Instant::now() + interval;
        }
    }
}

fn config(name: &'static str, x: i32) -> Config {
    let mut config = Config::new("match-proof", name);
    config.window_size = [640, 360];
    config.window_position = Some([x, 64]);
    config
}

/// Open the authority's native Crowd window and publish its join ticket.
///
/// # Errors
/// A socket, control or record could not be opened.
pub fn host(output: &Path, control: PathBuf, ticks: u64) -> Result<(), GameError> {
    let transport = native::Host::start(
        SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        Profile::default(),
    )
    .map_err(rejected)?;
    let settings = Settings::default();
    let session = thrum_network::session::Host::new(
        CrowdWorld::presented(BODIES, 1),
        NoActions,
        settings,
        [7; 16],
        71,
        true,
        0,
    )
    .map_err(rejected)?;
    let peer = Peer::Authority(Box::new(Authority {
        session,
        transport,
        control,
        end: ticks,
        staged: 0,
        offered: false,
    }));
    run(
        config("Crowd Match Host", 32),
        peer,
        Output::new(output, None).map_err(rejected)?,
    );
    Ok(())
}

/// Join through a socket from a second native Crowd window.
///
/// # Errors
/// The proof could not open the link or its record.
pub fn client(
    ticket: &Ticket,
    output: &Path,
    capture_control: Option<PathBuf>,
) -> Result<(), GameError> {
    let peer = Peer::Replica(Box::new(Replica::connect(ticket)?));
    let mut config = config("Crowd Match Client", 400);
    // Leave time for this small sprite scene to encode while keeping more of
    // the interval between observations available for committing due ticks.
    config.frame_budget = Some(Duration::from_micros(1_100));
    run(
        config,
        peer,
        Output::new(output, capture_control).map_err(rejected)?,
    );
    Ok(())
}

fn run(config: Config, peer: Peer, native: Output) {
    thrum_app::run_until_closed(config, || {
        WindowMatch::new(Driver {
            peer,
            native,
            last_frame: None,
        })
    });
}

fn drain(host: &mut Host, writer: &ImageWriter<SubmittedImage>) -> Result<(), GameError> {
    while let Some(image) = writer.recycled() {
        host.recycle_submitted_image(image);
    }
    while let Some(image) = host.try_submitted_image()? {
        let mut row = proof::image_sample(&image);
        row["event"] = json!("capture");
        host.world_mut()
            .driver_mut()
            .native
            .write(row)
            .map_err(rejected)?;
        if let Some(image) = writer.submit(image) {
            host.recycle_submitted_image(image);
            return Err(rejected("native T2 writer did not accept an image"));
        }
    }
    Ok(())
}

pub(super) fn capture(host: &mut Host) -> Result<(), GameError> {
    let output = &mut host.world_mut().driver_mut().native;
    let Some(control) = output.control.clone() else {
        return Ok(());
    };
    if output.stopped_capture {
        return Ok(());
    }
    let command = output.command.load(Ordering::Relaxed);
    let mut writer = output.capture.take();
    if writer.is_none() && command == RECORD {
        host.enable_submitted_capture_with_limits(
            1,
            CaptureLimits {
                staging_slots: 8,
                cpu_buffers: 32,
                byte_allowance: 128 * 1024 * 1024,
            },
        )?;
        let file = File::create(control.with_file_name("frames.spool")).map_err(rejected)?;
        writer = Some(
            ImageWriter::start_with_frame_limit(
                file,
                24,
                2 * 1024 * 1024 * 1024,
                SpoolEncoding::ZstdDelta,
                3600,
            )
            .map_err(rejected)?,
        );
    }
    if let Some(active) = &writer {
        if command == STOP {
            host.stop_submitted_capture();
        }
        drain(host, active)?;
    }
    let drained = host
        .submitted_capture_usage()
        .is_some_and(|(staging, _)| staging == 0);
    if command == STOP
        && drained
        && let Some(active) = writer.take()
    {
        let omitted = host.submitted_capture_omissions();
        host.disable_submitted_capture();
        let summary = active.finish(Duration::from_secs(3)).map_err(rejected)?;
        let output = &mut host.world_mut().driver_mut().native;
        output.stopped_capture = true;
        output
            .write(
                json!({"event":"capture_finished", "mode":"realtime", "frames":summary.frames,
            "omitted":omitted, "hashes":summary.frame_hashes}),
            )
            .map_err(rejected)?;
    }
    host.world_mut().driver_mut().native.capture = writer;
    Ok(())
}
