//! V1 scene setup, controls and proof observations for the shared mobile shell.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use thrum_app::{
    GameError, Gpu, WorldLink,
    ios::{MobileControls, MobileGame, MobileHost, MobilePrepared, SceneRequest},
};
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};
use thrum_render::runtime::FrameConfig;
use v1_game::{GameHost, GameScene, GameSetup, GameWorld, WorldControls};

#[cfg(feature = "proving")]
use thrum_app::{KeyCode, OutputMetrics};
#[cfg(feature = "proving")]
use thrum_ui::{Event, Routed, Viewport};
#[cfg(feature = "proving")]
use v1_game::{
    InterfaceText,
    mobile_interface::{self, InterfaceFrame, MobileInterface},
};

mod cooked_view;
mod culling;
#[cfg(feature = "proving")]
mod invite;

pub(super) struct V1;

type Host = MobileHost<V1>;

pub(super) struct Controls(WorldControls);

impl WorldLink for Controls {
    fn request_lifecycle(&self, request: WorldLifecycle) -> Result<(), WorldLaneFault> {
        self.0.request_lifecycle(request)
    }

    fn poll_lifecycle_reply(&self) -> Result<Option<WorldLifecycleReply>, WorldLaneFault> {
        self.0.poll_lifecycle_reply()
    }

    fn check_health(&self) -> Result<u64, WorldLaneFault> {
        self.0.check_health()
    }
}

impl MobileControls for Controls {
    fn world(&self) -> Option<&dyn WorldLink> {
        Some(self)
    }

    fn axes(&self, axes: [f32; 3]) -> Result<(), GameError> {
        self.0.set_controls(axes).map_err(GameError::Rejected)
    }
}

pub(super) struct Presentation {
    cooked: Option<Arc<Mutex<thrum_asset_gpu::CookedStats>>>,
    view: Option<cooked_view::View>,
}

impl MobileGame for V1 {
    type World = GameWorld;
    type Scene = GameScene;
    type Presentation = Presentation;
    type Controls = Controls;
    #[cfg(feature = "proving")]
    type Input = Option<MobileInterface>;
    #[cfg(feature = "proving")]
    type Readback = InterfaceFrame;
    #[cfg(not(feature = "proving"))]
    type Input = ();
    #[cfg(not(feature = "proving"))]
    type Readback = ();

    async fn prepare(
        gpu: Gpu<'_>,
        origin: Instant,
        wake: fn(),
        request: Option<SceneRequest>,
        qualification: bool,
    ) -> Result<MobilePrepared<Self>, GameError> {
        let config = FrameConfig {
            render_extent: gpu.surface.extent,
            surface: Some(gpu.surface),
        };
        let invite_scene = request.is_some();
        let (setup, mut host) = prepare_scene(&gpu, config, request)
            .await
            .map_err(GameError::Rejected)?;
        if qualification && std::env::var("THRUM_CULLING_ORACLE").as_deref() == Ok("1") {
            culling::record(
                &mut host,
                gpu.device,
                gpu.queue,
                gpu.capabilities,
                config,
                setup,
            )
            .map_err(GameError::Rejected)?;
        }
        #[cfg(feature = "proving")]
        if qualification && setup.sprite_scale.is_some() && setup.sprite_rows.is_some() {
            std::fs::write(
                std::env::temp_dir().join("thrum-sprite-card.rgba"),
                v1_game::sprite_card_texels(),
            )
            .map_err(|error| GameError::Rejected(error.to_string()))?;
        }
        #[cfg(feature = "proving")]
        let input = (setup.interface_text == Some(InterfaceText::Menu)).then(|| {
            let menu = MobileInterface::new(qualification);
            menu.view().attach(host.continuous_source_mut());
            menu
        });
        #[cfg(feature = "proving")]
        let output_metrics = matches!(
            setup.interface_text,
            Some(InterfaceText::Status | InterfaceText::Menu)
        );
        #[cfg(not(feature = "proving"))]
        let (input, output_metrics) = ((), false);
        let (cooked, view) = if invite_scene {
            (None, None)
        } else {
            (
                v1_game::attach_cooked_scene(&mut host).map_err(GameError::Rejected)?,
                cooked_view::View::prepare(&mut host).map_err(GameError::Rejected)?,
            )
        };
        let (host, controls) = host
            .into_threaded(origin, wake)
            .map_err(GameError::Rejected)?;
        Ok(MobilePrepared {
            host,
            scene: Presentation { cooked, view },
            controls: Controls(controls),
            input,
            output_metrics,
        })
    }

    fn sample_position(host: &Host) -> Option<[f64; 3]> {
        host.world().controlled_position()
    }

    fn frame(
        host: &mut Host,
        scene: &mut Presentation,
        command: u64,
        _: Duration,
    ) -> Result<(), GameError> {
        if let Some(view) = &mut scene.view {
            view.update(host, command == 11)
                .map_err(GameError::Rejected)?;
        }
        Ok(())
    }

    fn command_name(command: u64) -> &'static str {
        match command {
            11 => "CookedPan",
            22 => "OpenChoice",
            23 => "CloseChoice",
            _ => "Game",
        }
    }

    fn frame_witness(_: &Host, scene: &Presentation) -> String {
        let mut witness = scene.cooked.as_ref().and_then(|stats| stats.lock().ok())
            .map_or_else(String::new, |stats| {
                format!(
                    "cooked_path={:?}\ncooked_drawn={:?}\ncooked_device_generation={:?}\ncooked_submitted={}\ncooked_errors={:?}\ncooked_texture_profile={:?}",
                    stats.draw_path, stats.drawn, stats.drawn_device_generation, stats.submitted_frames, stats.errors,
                    stats.swaps.last().and_then(|swap| swap.texture_profile)
                )
            });
        if let Some(view) = &scene.view {
            witness.push_str(&view.witness());
        }
        witness
    }

    #[cfg(feature = "proving")]
    fn output(host: &mut Host, output: OutputMetrics) {
        mobile_interface::set_output(host.continuous_source_mut(), output.density, output.insets);
    }

    #[cfg(feature = "proving")]
    fn readback(host: &mut Host) -> Option<InterfaceFrame> {
        InterfaceFrame::read(host.continuous_source_mut())
    }

    #[cfg(feature = "proving")]
    fn viewport(shown: &InterfaceFrame) -> Option<Viewport> {
        Some(shown.viewport())
    }

    #[cfg(feature = "proving")]
    fn submitted(
        input: &mut Self::Input,
        frame: &InterfaceFrame,
        index: u64,
        now: Duration,
    ) -> bool {
        input
            .as_mut()
            .is_some_and(|menu| menu.submitted(frame, index, now))
    }

    #[cfg(feature = "proving")]
    fn wants_input(input: &Self::Input) -> bool {
        input.is_some()
    }

    #[cfg(feature = "proving")]
    fn event(input: &mut Self::Input, event: Event<KeyCode>, now: Duration) {
        if let Some(menu) = input {
            menu.event(event, now);
        }
    }

    #[cfg(feature = "proving")]
    fn tick(input: &mut Self::Input, now: Duration) {
        if let Some(menu) = input {
            menu.tick(now);
        }
    }

    #[cfg(feature = "proving")]
    fn command(input: &mut Self::Input, command: u64, now: Duration) {
        if let Some(menu) = input
            && matches!(command, 22 | 23)
        {
            menu.set_choice(command == 22, now);
        }
    }

    #[cfg(feature = "proving")]
    fn take_input(input: &mut Self::Input) -> impl Iterator<Item = Routed<KeyCode>> {
        input.iter_mut().flat_map(MobileInterface::input)
    }

    #[cfg(feature = "proving")]
    fn scene_request(input: &mut Self::Input, request: &SceneRequest, now: Duration) -> bool {
        if let Some(menu) = input {
            menu.open_invite(request.payload.clone(), now);
            true
        } else {
            false
        }
    }

    #[cfg(feature = "proving")]
    fn witness(input: &Self::Input) -> String {
        input.as_ref().map_or_else(String::new, |menu| {
            format!("interface_menu={}\n", menu.report())
        })
    }

    #[cfg(feature = "proving")]
    fn still_bounds(host: &mut Host) -> Option<serde_json::Value> {
        mobile_interface::status_bounds(host.continuous_source_mut())
    }
}

async fn prepare_scene(
    gpu: &Gpu<'_>,
    config: FrameConfig,
    request: Option<SceneRequest>,
) -> Result<(GameSetup, Host), String> {
    #[cfg(feature = "proving")]
    if let Some(request) = request {
        let setup = GameSetup {
            interface_text: Some(InterfaceText::Menu),
            sprite_scale: Some(2),
            sprite_rows: Some(10_000),
            ..GameSetup::default()
        };
        let host = setup
            .prepare_with_sheet_source(
                gpu.device,
                gpu.queue,
                gpu.capabilities,
                config,
                request.content.into(),
            )
            .await
            .map_err(|error| error.to_string())?;
        return Ok((setup, host));
    }
    #[cfg(not(feature = "proving"))]
    drop(request);
    let setup = v1_game::selected_setup()?;
    let host = setup
        .prepare(gpu.device, gpu.queue, gpu.capabilities, config)
        .await?;
    Ok((setup, host))
}

pub(super) fn route_enabled() -> bool {
    cfg!(feature = "proving") && std::env::var("THRUM_V1_ROUTE").as_deref() == Ok("1")
}
