//! Prepared variant attachment residency and native storage diagnostics.
use super::{DEPTH_FORMAT, OffscreenTarget, scene_graph::CompiledScene};
use crate::capabilities::Recipe;

impl CompiledScene {
    pub(super) fn prepare_day_depth(
        &self,
        device: &wgpu::Device,
        recipe: Recipe,
        width: u32,
        height: u32,
    ) -> Option<(wgpu::Texture, wgpu::TextureView)> {
        // A sampled night depth and a discard-only day depth are unique resident
        // allocations. No stored twin is allocated solely for a force override.
        (recipe == Recipe::AppleTile
            && self.night.is_some()
            && self.day.scene.memoryless_eligible(&self.ids.depth)
            && self.day.readback.memoryless_eligible(&self.ids.depth))
        .then(|| {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("graph-qualified day depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (texture, view)
        })
    }
}

impl OffscreenTarget {
    /// Prepared graph storage decisions and unique resident texture accounting.
    /// Native mode inspection reads existing handles only, without encoding work.
    #[must_use]
    pub fn storage_report(&self) -> String {
        let graphs = [
            Some(("day", &self.day_graphs)),
            self.night_graphs.as_ref().map(|graphs| ("stars", graphs)),
        ];
        let mut lines = Vec::new();
        for (name, graphs) in graphs.into_iter().flatten() {
            for (capture, graph) in [(false, &graphs.scene), (true, &graphs.readback)] {
                for item in &graph.plan().attachment_storage {
                    let texture = if name == "day" && item.resource == self.ids.depth {
                        self.day_depth
                            .as_ref()
                            .map(|(texture, _)| texture)
                            .or_else(|| self.textures.texture(&item.resource))
                    } else {
                        self.textures.texture(&item.resource)
                    };
                    if let Some(texture) = texture {
                        #[cfg(any(target_os = "macos", target_os = "ios"))]
                        let mode = crate::capabilities::apple::storage_mode(texture);
                        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
                        let mode: Option<&str> = None;
                        let role = if item.resource == self.ids.depth {
                            "depth"
                        } else if item.resource == self.ids.motion {
                            "motion"
                        } else if item.resource == self.ids.reactive {
                            "reactive"
                        } else if item.resource == self.ids.temporal.exposure {
                            "resolved exposure"
                        } else if item.resource == self.ids.scene {
                            "scene"
                        } else if item.resource == self.ids.output {
                            "output"
                        } else {
                            "auxiliary"
                        };
                        lines.push(format!("{name} capture={capture} role={role} resource={:?} reason={} usage={:?} metal={mode:?}", item.resource, item.reason, texture.usage()));
                    }
                }
            }
        }
        lines.push(format!("unique textures/nominal bytes={:?} fallback-only bytes=0; retained generations accounted by FrameHost", self.texture_allocation()));
        lines.join("\n")
    }
}

impl super::PreparedSynthetic {
    pub(super) fn prepare_star_target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        extent: [u32; 2],
    ) -> (Option<super::StarTarget>, bool) {
        let initial_view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        self.stars.as_ref().zip(self.atmosphere.as_ref()).map_or(
            (None, false),
            |(stars, atmosphere)| {
                let (binding, draw) = stars.target(
                    device,
                    depth,
                    atmosphere,
                    &self.exposure.samples,
                    (initial_view, extent),
                );
                (Some(binding), draw)
            },
        )
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "ios")))]
mod tests;
