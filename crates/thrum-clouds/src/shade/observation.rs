//! On-demand handles to a single committed browser submission.

use std::sync::Arc;

use super::{Mailbox, Prepared, WeatherInput};
use crate::{RowStamp, sun_path::ground_reference::GroundReference};

/// One submitted weather map and the Sun paths used to compose its ground field.
/// Copy the products on the supplied queue before the next render callback.
pub struct CloudSubmission {
    /// Device that owns all of this submission's resources.
    pub device: wgpu::Device,
    /// Queue that submitted the products.
    pub queue: wgpu::Queue,
    /// Generation of the submitted device.
    pub generation: u64,
    /// Authoritative state the submission selected.
    pub state: Arc<thrum_cloud_model::State>,
    /// The weather textures, in column, optical and identity order.
    pub weather: [wgpu::Texture; 3],
    /// Air-frame origin of each submitted weather level.
    pub weather_origins: Vec<[f64; 2]>,
    /// Source of every weather row.
    pub weather_rows: Vec<Vec<RowStamp>>,
    /// Three retained optical-depth paths, near to far.
    pub sun: [wgpu::Texture; 3],
    /// Source of every Sun-path row.
    pub sun_rows: [Vec<RowStamp>; 3],
    /// Packed ground optical-depth field.
    pub ground: wgpu::Texture,
    /// First trace distance, opacity, sample count and traced flag when Sky is enabled.
    pub sky_front: Option<wgpu::Texture>,
    ground_reference: Option<GroundReference>,
}

impl CloudSubmission {
    /// Replay the ground slice at every row stamped with this submission's tick.
    /// The caller must drain the readback before allowing another submission.
    pub fn replay_ground_slices(&self) -> Option<&wgpu::Texture> {
        Some(self.ground_reference.as_ref()?.replay(
            &self.device,
            &self.queue,
            &self.sun_rows,
            self.state.tick(),
        ))
    }
}

#[derive(Default)]
pub(super) struct Observation {
    armed: Option<bool>,
    latest: Option<CloudSubmission>,
}

impl WeatherInput {
    /// Arm an inspection of the next submitted cloud products.
    pub fn observe_next_submission(&self, ground: bool) {
        if let Ok(mut input) = self.0.lock() {
            input.observation.armed = Some(ground);
            input.observation.latest = None;
        }
    }

    /// Take the armed submission, if there was one.
    pub fn take_submission(&self) -> Option<CloudSubmission> {
        self.0.lock().ok()?.observation.latest.take()
    }
}

impl Mailbox {
    pub(super) fn observe(
        &mut self,
        gpu: &mut Prepared,
        generation: u64,
        state: Option<Arc<thrum_cloud_model::State>>,
    ) {
        let Some(ground) = self.observation.armed.take() else {
            return;
        };
        if let Some(state) = state {
            let ground_reference = ground.then(|| {
                gpu.ground_reference
                    .get_or_insert_with(|| {
                        GroundReference::new(&gpu.device, &gpu.sun, &gpu.weather)
                    })
                    .clone()
            });
            self.observation.latest = Some(CloudSubmission {
                device: gpu.device.clone(),
                queue: gpu.queue.clone(),
                generation,
                state,
                weather: gpu.weather.textures().clone(),
                weather_origins: gpu.weather.origins().to_vec(),
                weather_rows: gpu.weather.row_stamps().to_vec(),
                sun: gpu.sun.products().clone(),
                sun_rows: gpu.sun.row_stamps().clone(),
                ground: gpu.sun.ground().clone(),
                ground_reference,
                sky_front: self.sky.as_ref().map(|sky| sky.fresh_front.clone()),
            });
        }
    }
}
