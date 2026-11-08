"""Install the M1 opt-in witness in an isolated archive, including queue entry logs."""
from pathlib import Path
import sys

source, backend = map(Path, sys.argv[1:])

def replace(file, before, after):
    text = file.read_text()
    assert text.count(before) == 1, (file, before)
    file.write_text(text.replace(before, after, 1))

actor = source / "crates/thrum-app/src/app/render_actor.rs"
replace(actor, "struct Actor<G: Game> {", """#[path = "m1_loss_witness.rs"]
mod m1_loss_witness;
use m1_loss_witness::Witness;

struct Actor<G: Game> {
    witness: Witness,""")
replace(actor, "    let mut actor = Actor::<G> {", """    let mut actor = Actor::<G> {
        witness: Witness::from_env()?,""")
replace(actor, "        let advanced = self.host.advance(now)?;", """        self.witness.before_frame(&mut self.host, self.gpu.watch.generation)?;
        let advanced = self.host.advance(now)?;""")
replace(actor, "        let view = frame.texture.create_view(&TextureViewDescriptor::default());", """        if self.witness.should_destroy(self.gpu.watch.generation) {
            let loss = self.witness.destroy(&self.gpu, request.surface_generation, request.sequence);
            encoded.abandon_lost_device();
            drop(frame);
            self.witness.record_abandon(self.gpu.watch.generation, request.sequence);
            return Err(loss);
        }
        let view = frame.texture.create_view(&TextureViewDescriptor::default());""")
replace(actor, "            self.gpu.queue.present(frame);", """            self.gpu.queue.present(frame);
            self.witness.presented(self.gpu.watch.generation, request.sequence);""")
replace(actor, "        let report = encoded.submit(Some(&output))?;", """        self.witness.before_submit(
            self.gpu.watch.generation, request.surface_generation, request.sequence,
        );
        let report = encoded.submit(Some(&output))?;""")
replace(actor, "            RenderJob::Finish => RenderOutcome::Finish(self.finish()),", """            RenderJob::Finish => {
                self.witness.finish();
                RenderOutcome::Finish(self.finish())
            }""")
replace(actor, "        let inputs = self\n            .host\n            .device_prepare_inputs()", """        self.witness.recovery_requested(from, generation);
        let inputs = self
            .host
            .device_prepare_inputs()""")
replace(actor, "        self.gpu = replacement;", """        self.gpu = replacement;
        self.witness.published(&self.gpu);""")
app = source / "crates/thrum-app/src/app.rs"
replace(app, "        self.refresh_title();\n        if let Some(window) = &self.window {\n            window.request_redraw();\n        }\n        Ok(())\n    }\n\n    fn admit_recovery", """        if std::env::var_os("THRUM_M1_WITNESS").is_some() {
            eprintln!("m1 app loss-observed generation={generation} tick={} window={:?} pid={}",
                self.world_tick, self.window.as_ref().map(|window| window.id()), std::process::id());
        }
        self.refresh_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        Ok(())
    }

    fn admit_recovery""")
replace(app, "    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {", """    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if std::env::var_os("THRUM_M1_WITNESS").is_some_and(|path|
            std::path::PathBuf::from(path).join("finish").is_file())
        {
            event_loop.exit();
            return;
        }""")
replace(app, "        if was_transitioning || self.pause.needs_reply() || self.pause.next_request().is_some() {",
"""        if std::env::var_os("THRUM_M1_WITNESS").is_some()
            && (was_transitioning || !self.recovery.ready()) {
            eprintln!("m1 app authority tick={} state={} window={:?}", self.world_tick,
                self.world_status(), self.window.as_ref().map(|window| window.id()));
        }
        if was_transitioning || self.pause.needs_reply() || self.pause.next_request().is_some() {""")
replace(app, "                        self.set_key(code, event.state == ElementState::Pressed)", """                        if std::env::var_os("THRUM_M1_WITNESS").is_some()
                            && code == KeyCode::KeyE && event.state == ElementState::Pressed
                            && !event.repeat
                        {
                            eprintln!("m1 app camera E tick={} window={:?}", self.world_tick,
                                self.window.as_ref().map(|window| window.id()));
                            let unix_ms = std::time::SystemTime::UNIX_EPOCH.elapsed()
                                .unwrap_or_default().as_millis();
                            eprintln!("m1 event camera unix_ms={unix_ms}");
                        }
                        self.set_key(code, event.state == ElementState::Pressed)""")
replace(app, "                self.presented += 1;", """                self.presented += 1;
                if std::env::var_os("THRUM_M1_WITNESS").is_some()
                    && (self.presented == 1 || reply.device_generation > 1) {
                    eprintln!("m1 app presented generation={} sequence={} tick={} window={:?} pid={}",
                        reply.device_generation, reply.sequence, self.world_tick,
                        self.window.as_ref().map(|window| window.id()), std::process::id());
                }""")

core = backend / "src/backend/wgpu_core.rs"
replace(core, "        let index = match self.context.0.queue_submit(self.id, &command_buffer_ids) {",
"""        if std::env::var_os("THRUM_M1_WITNESS").is_some() {
            std::eprintln!("m1 queue submit id={:?} count={}", self.id, command_buffer_ids.len());
        }
        let index = match self.context.0.queue_submit(self.id, &command_buffer_ids) {""")
replace(core, "        match self.context.0.surface_present(detail.surface_id) {",
"""        if std::env::var_os("THRUM_M1_WITNESS").is_some() {
            std::eprintln!("m1 queue present id={:?} surface={:?}", self.id, detail.surface_id);
        }
        match self.context.0.surface_present(detail.surface_id) {""")
with (source / "Cargo.toml").open("a") as config:
    config.write('\n[patch.crates-io]\nwgpu = { path = "../wgpu" }\n')


window = source / "examples/v1-game/src/window.rs"
replace(window, "struct WindowGame {", """static WITNESS_COOKED: std::sync::OnceLock<
    std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>,
> = std::sync::OnceLock::new();

struct WindowGame {""")
replace(window, "        crate::attach_cooked_scene(&mut host)?;", """        let stats = crate::attach_cooked_scene(&mut host)?;
        if std::env::var_os("THRUM_M1_WITNESS").is_some() {
            // Freeze the initial endpoint before startup can admit a tick.
            // The window still sends and awaits the normal preset Pause.
            host.pause()?;
            if let Some(stats) = stats {
                drop(WITNESS_COOKED.set(stats));
            }
        }""")
replace(window, "        host::time_of_day(host)\n    }", """        let time = host::time_of_day(host);
        if let Some(stats) = WITNESS_COOKED.get()
            && let Ok(stats) = stats.lock()
        {
            eprintln!("m1 game time={time:?} drawn={:?} swaps={} submitted={}",
                stats.drawn, stats.swaps.len(), stats.submitted_frames);
        }
        time
    }""")

replace(app, "            self.pause.acknowledge(reply.request)?;", """            self.pause.acknowledge(reply.request)?;
            if std::env::var_os("THRUM_M1_WITNESS").is_some() {
                eprintln!("m1 app pause-ack request={:?} endpoint={:?}",
                    reply.request, reply.id);
            }""")
