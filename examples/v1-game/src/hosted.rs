//! V1's hosted runtime and control policy shared by its platform entry points.

use thrum_app::{Edits, Game, GameError, Gpu, KeyCode, KeyResponse, Prepared};
use thrum_controls::Controls;

use crate::{
    controls::{ControlEdits, GameControls},
    host::{self, GameScene, GameWorld, Runtime},
};

#[cfg(feature = "proving")]
use crate::interface_menu::{Menu, MenuFrame};
#[cfg(feature = "proving")]
use std::time::Duration;
#[cfg(feature = "proving")]
use thrum_ui::Event;

pub(crate) struct Readback {
    solar_time: f32,
    #[cfg(feature = "proving")]
    menu: Option<MenuFrame>,
    #[cfg(feature = "proving")]
    position: Option<[f64; 3]>,
}

pub(crate) struct HostedGame<T>(pub(crate) T);

pub(crate) trait RuntimeSetup {
    fn demo(&self) -> &GameControls;
    fn demo_mut(&mut self) -> &mut GameControls;
    fn replay(&self) -> ControlEdits;
    async fn prepare_runtime(
        &mut self,
        gpu: Gpu<'_>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<GameWorld, GameScene>, String>;

    #[cfg(target_os = "macos")]
    fn hardware_requests(&self) -> &[thrum_render::capabilities::HardwareRequest];
    #[cfg(target_os = "macos")]
    fn world(&self) -> Option<&dyn thrum_app::WorldLink>;
    #[cfg(target_os = "macos")]
    fn mac_scene_kind(&self) -> thrum_render::runtime::SceneKind {
        thrum_render::runtime::SceneKind::ThreeD
    }
    #[cfg(target_os = "macos")]
    fn supports_edr(&self) -> bool;

    fn readback(host: &Runtime) -> f32;

    #[cfg(all(target_os = "macos", feature = "proving"))]
    fn observe_display(&mut self, _display: thrum_app::DisplayRecord) {}

    #[cfg(all(target_os = "macos", feature = "proving"))]
    fn observe_frame(&mut self, _sample: &thrum_render::runtime::FrameSample) {}

    #[cfg(feature = "proving")]
    fn status_display(&self) -> bool;

    #[cfg(feature = "proving")]
    fn frame_edits(&self) -> ControlEdits {
        ControlEdits::default()
    }

    #[cfg(feature = "proving")]
    fn menu(&self) -> Option<&Menu> {
        None
    }

    #[cfg(feature = "proving")]
    fn menu_mut(&mut self) -> Option<&mut Menu> {
        None
    }

    #[cfg(feature = "proving")]
    fn interface_input(
        &mut self,
        event: Event<KeyCode>,
        now: Duration,
    ) -> Result<KeyResponse, String> {
        if let Some(menu) = self.menu_mut() {
            menu.event(event, now);
        }
        Ok(KeyResponse::Input)
    }

    fn key(&mut self, _key: KeyCode, _pressed: bool) -> Result<KeyResponse, String> {
        Ok(KeyResponse::Ignored)
    }

    fn focus_lost(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl<T: RuntimeSetup> Game for HostedGame<T> {
    type World = GameWorld;
    type Scene = GameScene;
    type Edits = ControlEdits;
    type Readback = Readback;

    #[cfg(target_os = "macos")]
    fn hardware_requests(&self) -> &[thrum_render::capabilities::HardwareRequest] {
        self.0.hardware_requests()
    }

    #[cfg(target_os = "macos")]
    fn mac_scene_kind(&self) -> thrum_render::runtime::SceneKind {
        self.0.mac_scene_kind()
    }

    #[cfg(target_os = "macos")]
    fn supports_edr(&self) -> bool {
        self.0.supports_edr()
    }

    #[cfg(all(target_os = "macos", feature = "proving"))]
    fn observe_display(&mut self, display: thrum_app::DisplayRecord) {
        self.0.observe_display(display);
    }

    #[cfg(all(target_os = "macos", feature = "proving"))]
    fn observe_frame(&mut self, sample: &thrum_render::runtime::FrameSample) {
        self.0.observe_frame(sample);
    }

    async fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<GameWorld, GameScene>, GameError> {
        self.0
            .prepare_runtime(gpu, world_advanced)
            .await
            .map_err(GameError::Rejected)
    }

    #[cfg(target_os = "macos")]
    fn world(&self) -> Option<&dyn thrum_app::WorldLink> {
        self.0.world()
    }

    fn controls(&self) -> &Controls {
        self.0.demo().model()
    }

    fn controls_mut(&mut self) -> &mut Controls {
        self.0.demo_mut().model_mut()
    }

    fn take_edits(&mut self) -> (ControlEdits, Option<bool>) {
        let edits = self.0.demo_mut().take_edits();
        (
            ControlEdits {
                paused: None,
                ..edits
            },
            edits.paused,
        )
    }

    #[cfg(feature = "proving")]
    fn wants_interface_output(&self) -> bool {
        self.0.status_display() || self.0.menu().is_some()
    }

    #[cfg(feature = "proving")]
    fn interface_output(&mut self, output: thrum_app::OutputMetrics) -> ControlEdits {
        ControlEdits {
            interface_output: Some(output),
            ..Default::default()
        }
    }

    #[cfg(feature = "proving")]
    fn frame_edits(&mut self) -> ControlEdits {
        self.0.frame_edits()
    }

    fn replay(&self) -> ControlEdits {
        self.0.replay()
    }

    fn apply(host: &mut Runtime, edits: ControlEdits) -> Result<(), GameError> {
        #[cfg(feature = "proving")]
        if let Some(output) = edits.interface_output {
            host.continuous_source_mut()
                .set_interface_output(output.density, output.insets);
        }
        #[cfg(feature = "proving")]
        if edits.status_cost {
            host.continuous_source_mut().measure_status_updates();
        }
        if let Some(value) = edits.time_of_day {
            host::edit_solar_time(host, value).map_err(GameError::Rejected)?;
        }
        if let Some(value) = edits.camera_pitch {
            host::set_camera_pitch(host, value).map_err(GameError::Rejected)?;
        }
        if let Some(value) = edits.camera_yaw {
            host::set_camera_yaw(host, value).map_err(GameError::Rejected)?;
        }
        if let Some(overlay) = edits.display_overlay() {
            host::set_display_overlay(host, overlay);
        }
        host::apply_cloud_edits(host, edits.clouds).map_err(GameError::Rejected)
    }

    fn readback(host: &Runtime) -> Readback {
        #[cfg(feature = "proving")]
        let menu = host.source().submitted_menu();
        Readback {
            solar_time: T::readback(host),
            #[cfg(feature = "proving")]
            position: menu
                .as_ref()
                .and_then(|_| host.world().controlled_position()),
            #[cfg(feature = "proving")]
            menu,
        }
    }

    fn observe(&mut self, readback: Readback) {
        self.0.demo_mut().observe_time(readback.solar_time);
    }

    #[cfg(feature = "proving")]
    fn wants_interface_input(&self) -> bool {
        self.0.menu().is_some()
    }

    #[cfg(feature = "proving")]
    fn interface_input(
        &mut self,
        event: Event<KeyCode>,
        now: Duration,
    ) -> Result<KeyResponse, GameError> {
        self.0
            .interface_input(event, now)
            .map_err(GameError::Rejected)
    }

    #[cfg(feature = "proving")]
    fn interface_tick(&mut self, now: Duration) {
        if let Some(menu) = self.0.menu_mut() {
            menu.tick(now);
        }
    }

    #[cfg(feature = "proving")]
    fn interface_submitted(&mut self, readback: &Readback, index: u64, now: Duration) {
        if let (Some(menu), Some(frame)) = (self.0.menu_mut(), readback.menu.as_ref()) {
            menu.submitted(frame, index, readback.position, now);
        }
    }

    #[cfg(all(feature = "proving", target_os = "macos"))]
    fn interface_script(&self) -> Option<&thrum_app::InterfaceScript> {
        self.0.menu()?.script.as_ref()
    }

    #[cfg(all(feature = "proving", target_os = "macos"))]
    fn interface_witness(&self) -> Option<String> {
        Some(self.0.menu()?.report().to_string())
    }

    #[cfg(feature = "proving")]
    fn take_clipboard_copy(&mut self) -> Option<String> {
        self.0.menu_mut()?.take_copy()
    }

    #[cfg(feature = "proving")]
    fn clipboard_result(&mut self, access: thrum_app::ClipboardAccess, allowed: bool) {
        if let Some(menu) = self.0.menu_mut() {
            menu.clipboard(access, allowed);
        }
    }

    fn observe_paused(&mut self, paused: bool) {
        self.0.demo_mut().observe_paused(paused);
    }

    fn key(&mut self, key: KeyCode, pressed: bool) -> Result<KeyResponse, GameError> {
        self.0.key(key, pressed).map_err(GameError::Rejected)
    }

    fn focus_lost(&mut self) -> Result<(), GameError> {
        self.0.focus_lost().map_err(GameError::Rejected)
    }
}

impl Edits for ControlEdits {
    fn merge(self, newer: Self) -> Self {
        Self {
            time_of_day: newer.time_of_day.or(self.time_of_day),
            camera_pitch: newer.camera_pitch.or(self.camera_pitch),
            camera_yaw: newer.camera_yaw.or(self.camera_yaw),
            paused: newer.paused.or(self.paused),
            exposure_zones: newer.exposure_zones.or(self.exposure_zones),
            motion_view: newer.motion_view.or(self.motion_view),
            clouds: crate::controls::CloudEdits {
                weather: newer.clouds.weather.or(self.clouds.weather),
                ground: newer.clouds.ground.or(self.clouds.ground),
                overlay: newer.clouds.overlay.or(self.clouds.overlay),
            },
            #[cfg(feature = "proving")]
            interface_output: newer.interface_output.or(self.interface_output),
            #[cfg(feature = "proving")]
            status_cost: newer.status_cost || self.status_cost,
        }
    }

    fn is_empty(&self) -> bool {
        ControlEdits::is_empty(self)
    }
}
