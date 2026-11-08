"""Install the pan witness into an isolated archive of the V1 game."""
from pathlib import Path
import shutil
import sys

source = Path(sys.argv[1]).resolve()
here = Path(__file__).resolve().parent


def replace(path, old, new):
    text = path.read_text()
    if text.count(old) != 1:
        raise RuntimeError(f"{path}: expected one match for {old!r}")
    path.write_text(text.replace(old, new, 1))


app_crate = source / "crates/thrum-app"
for file in ("pan_timeline.rs", "pan_witness.rs"):
    shutil.copyfile(here / file, app_crate / "src/app" / file)
build = app_crate / "build.rs"
if build.exists():
    raise RuntimeError("unexpected thrum-app build script")
build.write_text(
    "//! Link only the scratch M2 signpost bridge.\nfn main() {\n"
    f'    println!("cargo::rustc-link-search=native={here}");\n'
    '    println!("cargo::rustc-link-lib=static=thrum_m2_pan");\n'
    f'    println!("cargo::rerun-if-changed={here}/libthrum_m2_pan.a");\n'
    "}\n"
)
game_trait = app_crate / "src/game.rs"
replace(game_trait, "    type Readback: Copy + Send + 'static;",
        """    type Readback: Copy + Send + 'static;

    /// Committed yaw for the scratch Mac pan witness, if the game records it.
    fn m2_yaw(_readback: Self::Readback) -> Option<f32> {
        None
    }""")

app = app_crate / "src/app.rs"
replace(app, "mod pause;", "mod pause;\nmod pan_timeline;\nmod pan_witness;")
replace(app, "struct App<G: Game> {",
        "struct App<G: Game> {\n    pan: Option<pan_witness::Witness>,")
replace(app, "            config,\n            wake,",
        "            pan: pan_witness::Witness::from_env(),\n            config,\n            wake,")
replace(app, "        self.window = Some(window);", """        if self.pan.is_some() {
            eprintln!("m2 app pid={} window={:?}", std::process::id(), window.id());
        }
        self.window = Some(window);""")
replace(app, "            self.pause.acknowledge(reply.request)?;", """            self.pause.acknowledge(reply.request)?;
            if self.pan.is_some() {
                eprintln!("m2 lifecycle {:?} {:?}", reply.request, reply.id);
            }""")
replace(app, "                self.presented += 1;", """                self.presented += 1;
                if let (Some(pan), Some(yaw)) = (self.pan.as_mut(), G::m2_yaw(reply.readback)) {
                    pan.presented(reply.sequence, yaw)?;
                }""")
replace(app, """                self.pending_display_target = true;
                self.edits.request();
            }
            UserEvent::WorldAdvanced""", """                self.pending_display_target = true;
                self.edits.request();
                if let Some(pan) = self.pan.as_mut()
                    && let Err(error) = pan.sample(target, self.target_sequence,
                        self.panel.as_ref(), self.game.controls())
                {
                    self.error = Some(error);
                    event_loop.exit();
                    return;
                }
            }
            UserEvent::WorldAdvanced""")
replace(app, """                if let Err(error) = self.controls_edited() {
                    self.error = Some(error);
                    event_loop.exit();
                }
                return;
            }
            UserEvent::FilesSaved""", """                let logged = if let Some(pan) = self.pan.as_mut() {
                    pan.accepted(self.target_sequence, self.game.controls())
                } else {
                    Ok(())
                };
                if let Err(error) = logged.and_then(|()| self.controls_edited()) {
                    self.error = Some(error);
                    event_loop.exit();
                }
                return;
            }
            UserEvent::FilesSaved""")
replace(app, "    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {",
        """    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(pan) = self.pan.as_mut() && pan.should_finish() {
            if let Err(error) = pan.finish() {
                self.error = Some(error);
            }
            self.target_frames = self.presented;
            event_loop.exit();
            return;
        }""")

game = source / "examples/v1-game/src/window.rs"
replace(game, "pub fn run_window() {", """pub fn run_window() {
    if std::env::var_os("THRUM_M2_PANEL_PREFLIGHT").is_some() {
        if let Err(error) = panel_preflight() {
            eprintln!("m2 panel preflight: {error}");
            std::process::exit(1);
        }
        return;
    }""")
replace(game, "struct WindowGame {", """fn panel_preflight() -> Result<(), String> {
    use std::{cell::Cell, rc::Rc};
    use thrum_controls::macos::MacPanel;

    let mut controls = GameControls::new().map_err(|error| error.to_string())?;
    let id = controls.model().find("camera.yaw").ok_or("no yaw slider")?;
    let wakes = Rc::new(Cell::new(0));
    let wake_count = Rc::clone(&wakes);
    let mut panel = MacPanel::new(
        controls.model(),
        "Thrum M2 headless slider proof",
        Some(Box::new(move || wake_count.set(wake_count.get() + 1))),
    )?;
    let mut previous = 0.0;
    let mut min_step = f32::MAX;
    let mut max_step = 0.0_f32;
    for index in 1..=240 {
        if !panel.simulate(id, f64::from(index) * 0.15) {
            return Err(format!("slider refused index {index}"));
        }
        if panel.apply_edits(controls.model_mut()) != 1 {
            return Err(format!("AppKit did not deliver one changed yaw at index {index}"));
        }
        let yaw = controls
            .take_edits()
            .camera_yaw
            .ok_or_else(|| format!("game did not accept yaw at index {index}"))?
            .to_degrees();
        let delta = yaw - previous;
        min_step = min_step.min(delta);
        max_step = max_step.max(delta);
        previous = yaw;
    }
    if wakes.get() != 240 || min_step <= 0.0 || (previous - 36.0).abs() > 0.01 {
        return Err("AppKit did not dispatch the complete monotone pan".into());
    }
    println!("m2 hidden panel actions={} final_yaw={previous:.6} min_step={min_step:.6} max_step={max_step:.6}", wakes.get());
    Ok(())
}

struct WindowGame {""")
replace(game, "struct WindowGame {", """static PAN_COOKED: std::sync::OnceLock<(
    std::path::PathBuf, std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>
)> = std::sync::OnceLock::new();
static PAN_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct WindowGame {""")
replace(game, "        crate::attach_cooked_scene(&mut host)?;", """        let cooked = crate::attach_cooked_scene(&mut host)?;
        if let Some(output) = std::env::var_os("THRUM_M2_DIR") {
            host.pause()?;
            let cooked = cooked.ok_or("the M2 witness needs the cooked valley")?;
            drop(PAN_COOKED.set((std::path::PathBuf::from(output), cooked)));
        }""")
replace(game, "    type Readback = f32;", """    type Readback = (f32, f32);

    fn m2_yaw(readback: Self::Readback) -> Option<f32> {
        Some(readback.1)
    }""")
replace(game, """    fn readback(host: &Runtime) -> f32 {
        host::time_of_day(host)
    }

    fn observe(&mut self, time_of_day: f32) {
        self.demo.observe_time(time_of_day);
    }""", """    fn readback(host: &Runtime) -> Self::Readback {
        use std::sync::atomic::Ordering::Relaxed;
        if !PAN_READY.load(Relaxed)
            && let Some((output, cooked)) = PAN_COOKED.get()
            && let Ok(stats) = cooked.try_lock()
            && let Some(generation) = stats.drawn
        {
            PAN_READY.store(true, Relaxed);
            match std::fs::write(output.join("cooked-ready"), generation.to_string()) {
                Ok(()) => eprintln!("m2 cooked ready generation={generation}"),
                Err(error) => eprintln!("m2 cooked readiness write failed: {error}"),
            }
        }
        (host::time_of_day(host), host::m2_camera_yaw(host))
    }

    fn observe(&mut self, readback: Self::Readback) {
        self.demo.observe_time(readback.0);
    }""")
host = source / "examples/v1-game/src/host.rs"
replace(host, "/// Turn the game camera and reset presentation history.\npub(crate) fn set_camera_yaw",
        """pub(crate) fn m2_camera_yaw(runtime: &Runtime) -> f32 {
    runtime.source().camera_yaw
}

/// Turn the game camera and reset presentation history.
pub(crate) fn set_camera_yaw""")
