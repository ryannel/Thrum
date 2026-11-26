//! The proving app's Join scene, prepared on the render owner.

use std::{
    path::PathBuf,
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use thrum_render::{
    capabilities::DeviceCapabilities, present::SurfaceProfile, runtime::FrameConfig,
};
use v1_game::{
    GameHost, GameSetup, InterfaceText, WorldControls, mobile_interface::MobileInterface,
};

use super::Owner;
use crate::{ios::Host, lease::TickPermission};

pub(in crate::ios) struct Opened {
    pub(in crate::ios) controls: WorldControls,
    pub(in crate::ios) menu: MobileInterface,
    pub(in crate::ios) origin: Instant,
}

pub(super) async fn prepare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
    config: FrameConfig,
    store: PathBuf,
) -> Result<(GameSetup, Host), String> {
    let setup = GameSetup {
        interface_text: Some(InterfaceText::Menu),
        sprite_scale: Some(2),
        sprite_rows: Some(10_000),
        ..GameSetup::default()
    };
    let host = setup
        .prepare_with_sheet_source(device, queue, capabilities, config, store.into())
        .await
        .map_err(|error| error.to_string())?;
    Ok((setup, host))
}

pub(super) fn attach_menu(host: &mut Host, diagnostics: bool) -> MobileInterface {
    let menu = MobileInterface::new(diagnostics);
    menu.view().attach(host.continuous_source_mut());
    menu
}

impl Owner {
    pub(super) fn open_join(&mut self, store: PathBuf) -> Result<Option<Opened>, String> {
        let gpu = self
            .gpu
            .as_mut()
            .ok_or("iOS render owner is not prepared")?;
        let background = || self.signals.background.load(Ordering::Acquire);
        let epoch = self.lease.epoch();
        let extent = self.lease.observed_extent();
        if background() || extent.contains(&0) || gpu.destroyed || gpu.watch.is_lost() {
            return Ok(None);
        }
        // The last scene's GPU packets must retire before its host goes away.
        let retired = gpu.host.poll_completed();
        if gpu.watch.is_lost() {
            return Ok(None);
        }
        retired.map_err(|error| error.to_string())?;
        if gpu.host.pending_frames() != 0 {
            return Ok(None);
        }
        let config = FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: gpu.format,
                extent,
            }),
        };
        let candidate = pollster::block_on(prepare(
            &gpu.device,
            &gpu.queue,
            gpu.host.device_capabilities(),
            config,
            store,
        ));
        if background() || gpu.watch.is_lost() || !self.lease.is_current(epoch) {
            return Ok(None);
        }
        let (_, mut candidate) = candidate?;
        let menu = attach_menu(&mut candidate, self.diagnostics);
        let origin = Instant::now();
        let (mut candidate, controls) =
            candidate.into_threaded(origin, crate::ios::notify_world)?;
        candidate.set_tick_submission_permission(Box::new(TickPermission {
            lease: Arc::clone(&self.lease),
            request_epoch: Arc::clone(&gpu.request_epoch),
            background: Arc::clone(&self.signals.background),
        }));
        // Publish at a live lease without holding it across preparation or
        // stopping the old world lane.
        let mut candidate = Some(candidate);
        let previous = self.lease.section(epoch, |_| {
            if background() || gpu.watch.is_lost() {
                return None;
            }
            candidate
                .take()
                .map(|host| std::mem::replace(&mut gpu.host, host))
        });
        let Some(Some(mut previous)) = previous else {
            return Ok(None);
        };
        gpu.host_extent = extent;
        gpu.cooked = None;
        gpu.cooked_view = None;
        if let Err(error) = self
            .capture
            .cancel(&mut previous, &self.signals, "opened Join")
        {
            self.capture.failed(&mut previous, &self.signals, &error);
        }
        self.origin = origin;
        Ok(Some(Opened {
            controls,
            menu,
            origin,
        }))
    }
}
