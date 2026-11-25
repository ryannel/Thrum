//! Resolve stable frame handles and bind a single publication's table and pages.

use std::collections::HashMap;
use std::sync::Arc;

use thrum_asset::SheetKind;

use super::{FrameInput, GpuRect, SpriteScene, frame_buffer};
use crate::error::RenderError;
use crate::sprite::SpriteSheetPublication;

#[derive(Default)]
pub(super) struct CookedFrames {
    offer: Option<Arc<SpriteSheetPublication>>,
    slots: Vec<CookedSlot>,
    pub(super) buffer: Option<wgpu::Buffer>,
    pub(super) revision: u64,
}

#[derive(Default)]
struct CookedSlot {
    names_seen: usize,
    view: Option<wgpu::TextureView>,
    frames: Vec<Option<FrameInput>>,
}

#[derive(Clone, Copy)]
pub(super) enum Missing {
    Sheet,
    Frame,
}

impl CookedFrames {
    pub(super) fn frame(&self, slot: usize, handle: u32) -> Result<FrameInput, Missing> {
        let Some(slot) = self.slots.get(slot).filter(|slot| slot.view.is_some()) else {
            return Err(Missing::Sheet);
        };
        slot.frames
            .get(handle as usize)
            .copied()
            .flatten()
            .ok_or(Missing::Frame)
    }

    pub(super) fn view(&self, slot: usize) -> Option<&wgpu::TextureView> {
        self.slots.get(slot)?.view.as_ref()
    }

    pub(super) fn generation(&self) -> Option<u64> {
        self.offer.as_ref().map(|offer| offer.generation)
    }

    pub(super) fn paint_stamp(&self) -> Option<crate::sprite::PaintStamp> {
        self.offer.as_ref().and_then(|offer| offer.paint_stamp)
    }

    pub(super) fn update(
        &mut self,
        scene: &SpriteScene,
        device: &wgpu::Device,
        generation: u64,
        embedded: &[GpuRect],
    ) -> Result<bool, RenderError> {
        if scene.cooked.is_empty() {
            return Ok(false);
        }
        let offer = scene
            .lease
            .as_ref()
            .and_then(|source| source.sheet_publication())
            .filter(|offer| offer.device_generation == generation);
        let same_offer = match (&self.offer, &offer) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        let names: Vec<_> = scene
            .cooked
            .iter()
            .map(|sheet| {
                sheet
                    .names
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            })
            .collect();
        if same_offer
            && self.slots.len() == names.len()
            && self
                .slots
                .iter()
                .zip(&names)
                .all(|(slot, names)| slot.names_seen == names.len())
        {
            return Ok(false);
        }
        let mut table = embedded.to_vec();
        let slots = scene
            .cooked
            .iter()
            .zip(names)
            .map(|(sheet, names)| {
                let page = offer
                    .as_ref()
                    .and_then(|offer| offer.sheets.iter().find(|page| page.name == sheet.name));
                let Some(page) = page else {
                    return Ok(CookedSlot {
                        names_seen: names.len(),
                        ..CookedSlot::default()
                    });
                };
                let sampling = match page.product.kind() {
                    SheetKind::Pixel => crate::sprite::SpriteSampling::Pixel,
                    SheetKind::Smooth => crate::sprite::SpriteSampling::Smooth,
                };
                let by_name: HashMap<_, _>;
                let frames: Vec<_> = if sheet.indexed {
                    page.product
                        .frames()
                        .map(|(_, frame)| Some(frame))
                        .collect()
                } else {
                    by_name = page.product.frames().collect();
                    names
                        .iter()
                        .map(|name| by_name.get(name.as_str()).copied())
                        .collect()
                };
                let frames = frames
                    .into_iter()
                    .map(|frame| {
                        let Some(frame) = frame else {
                            return Ok(None);
                        };
                        let index = u32::try_from(table.len())
                            .map_err(|_| RenderError::SpriteFrameCapacity)?;
                        table.push(GpuRect {
                            origin: frame.origin,
                            size: frame.size,
                            page: frame.page,
                            kind: sampling as u32,
                            tile: frame.tile,
                            padding: 0,
                        });
                        Ok(Some(FrameInput {
                            index,
                            size: frame.size,
                            density: page.product.density(),
                            offset: trim_offset(frame.trim, frame.pivot),
                            sampling,
                            tile: frame.tile != 0,
                        }))
                    })
                    .collect::<Result<Vec<_>, RenderError>>()?;
                Ok(CookedSlot {
                    names_seen: names.len(),
                    view: Some(page.view.clone()),
                    frames,
                })
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        let limits = device.limits();
        let max = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        if table.len() as u64 > max / size_of::<GpuRect>() as u64 {
            return Err(RenderError::SpriteFrameCapacity);
        }
        self.buffer = Some(frame_buffer(device, &mut table));
        self.slots = slots;
        self.offer = offer;
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "sprite trim offsets are represented in GPU texel coordinates"
)]
fn trim_offset(trim: [u32; 2], pivot: [f32; 2]) -> [f32; 2] {
    [trim[0] as f32 - pivot[0], trim[1] as f32 - pivot[1]]
}
