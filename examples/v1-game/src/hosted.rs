//! V1's hosted runtime and control policy shared by its platform entry points.

use thrum_app::{Edits, Game, GameError, Gpu, KeyCode, KeyResponse, Prepared};
use thrum_controls::Controls;

use crate::{
    controls::{ControlEdits, GameControls},
    host::{self, GameScene, GameWorld, Runtime},
};

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
    fn supports_edr(&self) -> bool;

    fn readback(host: &Runtime) -> f32;

    #[cfg(feature = "proving")]
    fn status_display(&self) -> bool;

    #[cfg(feature = "proving")]
    fn frame_edits(&self) -> ControlEdits {
        ControlEdits::default()
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
    type Readback = f32;

    #[cfg(target_os = "macos")]
    fn hardware_requests(&self) -> &[thrum_render::capabilities::HardwareRequest] {
        self.0.hardware_requests()
    }

    #[cfg(target_os = "macos")]
    fn supports_edr(&self) -> bool {
        self.0.supports_edr()
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
        self.0.status_display()
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
            host::set_time_of_day(host, value).map_err(GameError::Rejected)?;
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
        Ok(())
    }

    fn readback(host: &Runtime) -> f32 {
        T::readback(host)
    }

    fn observe(&mut self, time_of_day: f32) {
        self.0.demo_mut().observe_time(time_of_day);
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
