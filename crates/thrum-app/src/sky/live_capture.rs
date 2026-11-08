//! Live review capture of a sky scene, for a creator's saves on a machine with no visible window:
//! the same host, contribution, file watch and reload body as the window, rendered offscreen.
//! After each settled save is applied and drawn it writes the next numbered PNG and one report
//! line, so a script can check each save's outcome. It exercises the shared reload path, not the
//! window's event path, and makes no presentation or timing claim.

use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use thrum_render::runtime::{FrameConfig, FrameHost};

use super::{SkyGame, SkyScene, StillWorld, capture};
use crate::{
    Config, Game,
    file_watch::{self, FileWatch, Kinds, Reload},
    game::apply_params_file,
};

/// With `THRUM_CAPTURE_PNG`, the seconds to keep watching and capturing.
pub(super) const LIVE_VARIABLE: &str = "THRUM_CAPTURE_LIVE";
/// Interval between idle frames, which advance the contribution's uploads.
const IDLE_FRAME: Duration = Duration::from_millis(20);

#[expect(
    clippy::print_stderr,
    reason = "capture failures are explained to the person who asked for them"
)]
pub(super) fn run(config: Config, game: impl FnOnce() -> Result<SkyGame, String>, path: &Path) {
    if let Err(error) = live(config, game, path) {
        eprintln!(
            "{}: live capture to {}: {error}",
            config.name,
            path.display()
        );
        std::process::exit(1);
    }
}

fn live(
    config: Config,
    game: impl FnOnce() -> Result<SkyGame, String>,
    path: &Path,
) -> Result<(), String> {
    let seconds: f64 = std::env::var(LIVE_VARIABLE)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|seconds: &f64| seconds.is_finite() && *seconds > 0.0 && *seconds <= 3600.0)
        .ok_or_else(|| format!("{LIVE_VARIABLE} must be a number of seconds up to 3600"))?;
    let game = game()?;
    let (device, queue, capabilities) = pollster::block_on(capture::headless_device())?;
    let extent = config.window_size;
    let params = std::env::var_os("THRUM_PARAMS").map(PathBuf::from);
    let mut live = Live {
        name: config.name,
        game,
        host: None,
        params: params.clone(),
        extent,
        path: path.to_owned(),
        index: 0,
        outcome: String::new(),
        awaiting: false,
    };
    // The window applies the file and reloads the game once it is up; so does this.
    live.reload_params(false)?;
    let mut host = live.game.build_host(
        &capabilities,
        &queue,
        FrameConfig {
            render_extent: extent,
            surface: None,
        },
    )?;
    capture::wait_drawn(
        &mut live.game,
        &mut host,
        Instant::now() + capture::CONTENT_WAIT,
    )?;
    live.host = Some(host);
    live.reload_game();
    live.settle()?;
    live.write("start")?;
    let files = params
        .into_iter()
        .map(|path| (path, Kinds::PARAMS))
        .chain(
            live.game
                .reload_files()
                .into_iter()
                .map(|path| (path, Kinds::GAME)),
        )
        .collect();
    let watch = FileWatch::start(files, file_watch::TICK, Box::new(|| {}))
        .map_err(|error| format!("cannot watch files: {error}"))?
        .ok_or("nothing to watch: set THRUM_PARAMS or THRUM_MATERIAL")?;
    let end = Instant::now() + Duration::from_secs_f64(seconds);
    while Instant::now() < end {
        let kinds = watch.take();
        if kinds.is_empty() {
            live.frame()?;
            std::thread::sleep(IDLE_FRAME);
            continue;
        }
        file_watch::deliver(&mut live, kinds, true)?;
        live.settle()?;
        let label = match (kinds.params(), kinds.game()) {
            (true, true) => "params+game",
            (true, false) => "params",
            _ => "game",
        };
        live.write(label)?;
    }
    drop(watch);
    drop(live.host.take());
    drop(device);
    Ok(())
}

struct Live {
    name: &'static str,
    game: SkyGame,
    host: Option<FrameHost<StillWorld, SkyScene>>,
    params: Option<PathBuf>,
    extent: [u32; 2],
    path: PathBuf,
    index: usize,
    /// What the reloads since the last PNG reported.
    outcome: String,
    /// A game reload started and has not reported yet.
    awaiting: bool,
}

impl Live {
    /// Render one frame: a frame boundary at which uploads and a prepared material install.
    fn frame(&mut self) -> Result<Vec<u8>, String> {
        let host = self.host.as_mut().ok_or("no host")?;
        host.capture_pixels().map_err(|error| error.to_string())
    }

    /// Render until a started game reload reports, then once more so its result is drawn.
    fn settle(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(30);
        while self.awaiting {
            drop(self.frame()?);
            if let Some((_, report)) = self.game.reload.as_mut() {
                for line in report() {
                    let _ = match line {
                        Ok(line) => write!(self.outcome, "; game: drawn {line}"),
                        Err(line) => write!(self.outcome, "; game: kept the last good ({line})"),
                    };
                    self.awaiting = false;
                }
            }
            if self.awaiting && Instant::now() >= deadline {
                return Err("the game's reload did not report within 30 s".to_owned());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
    }

    /// Write the next numbered PNG and its report line.
    #[expect(
        clippy::print_stdout,
        reason = "each line reports one capture to the script driving the saves"
    )]
    fn write(&mut self, label: &str) -> Result<(), String> {
        let pixels = self.frame()?;
        let stem = self
            .path
            .file_stem()
            .ok_or("the capture path has no file name")?
            .to_string_lossy()
            .into_owned();
        let numbered = self
            .path
            .with_file_name(format!("{stem}-{}.png", self.index));
        capture::write_png(&numbered, self.extent, &pixels)?;
        println!(
            "live {} {label}: {}{}",
            self.index,
            numbered.display(),
            self.outcome
        );
        self.outcome.clear();
        self.index += 1;
        Ok(())
    }
}

impl Reload for Live {
    fn stopped(&self) -> bool {
        false
    }

    /// The window's apply: a refused entry is reported and keeps its value while the others
    /// apply; an unreadable or malformed file changes nothing.
    #[expect(
        clippy::print_stderr,
        reason = "parameter diagnostics are for the person editing the file"
    )]
    fn reload_params(&mut self, saved: bool) -> Result<(), String> {
        let Some(path) = self.params.as_deref() else {
            return Ok(());
        };
        let tag = if saved { " (saved)" } else { "" };
        match apply_params_file(path, &mut self.game.controls) {
            Ok(report) => {
                eprintln!("{}: {}{tag}: {report}", self.name, path.display());
                for problem in &report.problems {
                    eprintln!("  {problem}");
                }
                let _ = write!(self.outcome, "; params: {report}");
                let (edits, _) = self.game.take_edits();
                if let Some(host) = self.host.as_mut() {
                    SkyGame::apply(host, edits)?;
                }
            }
            Err(error) => {
                eprintln!(
                    "{}: {}{tag}: {error}; controls keep their values",
                    self.name,
                    path.display()
                );
                let _ = write!(self.outcome, "; params: kept ({error})");
            }
        }
        Ok(())
    }

    fn reload_game(&mut self) {
        if let Some((reload, _)) = self.game.reload.as_mut() {
            match reload() {
                Ok(()) => self.awaiting = true,
                Err(error) => {
                    let _ = write!(self.outcome, "; game: kept ({error})");
                }
            }
        }
    }

    fn redraw(&self) {}
}
