use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    sync::Arc,
};

use crowd_2d::Body;
use thrum_core::sim::{PublicationPage, StructuralChange};
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameError, SceneFrame, SceneSource},
    sprite::{
        EmbeddedSheet, SpriteFrame, SpritePose, SpriteRect, SpriteRow, SpriteScene, SpriteSink,
        SpriteView,
    },
    synthetic::FrameLighting,
};

// 90th percentile of relative sort jitter at reversals, rounded up to 0.01.
pub(crate) const ORDER_BAND: f64 = 0.06;
const SIDE: u32 = 16;
const FRAMES: u32 = 5;
const DENSITY: f32 = 8.0;

fn position(body: &Body) -> [f64; 2] {
    [body.position.x, body.position.y].map(|axis| f64::from(f32::from_bits(axis.to_bits())))
}

#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "art coordinates are sixteen texels and the opacity ramp is a bounded positive byte"
)]
fn sprite_pixel(frame: u32, x: u32, y: u32) -> [u8; 4] {
    let x = x as f32 - 7.5;
    let y = y as f32 - 7.5;
    let radius = x.hypot(y);
    match frame {
        0 if x.abs() + y.abs() > 9.0 => [0; 4],
        0 if x.abs() + y.abs() > 6.0 => [8, 29, 39, 255],
        0 if x.abs() <= 1.5 && y.abs() <= 3.5 => [240, 255, 240, 255],
        0 if y < 0.0 => [91, 253, 216, 255],
        0 => [37, 150, 184, 255],
        1..=3 if x.hypot(y + 0.5) > 6.7 => [0; 4],
        1..=3 if radius > 5.4 => [22, 17, 33, 255],
        1..=3 if y < -1.0 && (x.abs() - 2.5).abs() < 1.0 => [255, 213, 159, 255],
        1 if y < 1.0 => [180, 80, 102, 255],
        2 if y < 1.0 => [153, 68, 113, 255],
        3 if y < 1.0 => [184, 99, 83, 255],
        1..=3 => [80, 36, 57, 255],
        _ if radius <= 2.2 => [255, 242, 166, 255],
        _ if radius < 7.0 => [255, 128, 27, ((7.0 - radius) * 22.0) as u8],
        _ => [0; 4],
    }
}

pub(crate) fn configuration(count: u32, band: f64) -> SpriteScene {
    let pixels: Vec<_> = (0..SIDE)
        .flat_map(|y| {
            (0..FRAMES)
                .flat_map(move |frame| (0..SIDE).flat_map(move |x| sprite_pixel(frame, x, y)))
        })
        .collect();
    let mut scene = SpriteScene::new(
        EmbeddedSheet {
            extent: [SIDE * FRAMES, SIDE],
            rgba: pixels.into(),
            frames: (0..FRAMES)
                .map(|frame| SpriteRect {
                    origin: [frame * SIDE, 0],
                    size: [SIDE; 2],
                })
                .collect(),
            texels_per_unit: DENSITY,
        },
        0,
    );
    // Exactly the physical slot capacity of the reference trace.
    scene.world_capacity = count + 128;
    scene.order_band = band;
    scene
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Extraction {
    pub tick: u64,
    pub live: usize,
    pub mapped: u32,
    pub reused: u32,
    pub skipped: u32,
}

/// Maps published bodies to presentation without writing any rule state.
pub struct CrowdScene {
    close: bool,
    camera: Cell<[[f64; 2]; 2]>,
    extraction: Cell<Extraction>,
    projectiles: RefCell<BTreeMap<u32, (u64, Body)>>,
    flashes: RefCell<Vec<([f64; 2], u64)>>,
}

impl CrowdScene {
    pub(crate) fn new(close: bool) -> Self {
        Self {
            close,
            camera: Cell::default(),
            extraction: Cell::default(),
            projectiles: RefCell::default(),
            flashes: RefCell::new(Vec::with_capacity(16)),
        }
    }

    pub(crate) fn extraction(&self) -> Extraction {
        self.extraction.get()
    }
}

pub(crate) fn row(body: &Body, identity: u128) -> SpriteRow {
    let [x, y] = position(body);
    let (frame, scale, layer, additive) = match body.kind {
        0 => (0, 2.3, 0, 0.0),
        1 => (
            1 + u32::try_from(identity % 3).expect("three colours"),
            1.27,
            0,
            0.0,
        ),
        _ => (4, 1.1, 1, 0.75),
    };
    SpriteRow {
        scale: [scale; 2],
        layer,
        additive,
        sort: Some(-y),
        ..SpriteRow::new(frame, [x - f64::from(scale), y + f64::from(scale)])
    }
}

impl SceneSource<Arc<PublicationPage>> for CrowdScene {
    fn extract(&self, _: &mut RenderExtractor, _: &Arc<PublicationPage>) -> ExtractReport {
        unreachable!("the Crowd prepares a 2D scene")
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Err(FrameError::InvalidState("the Crowd has no 3D camera"))
    }

    fn extract_sprites(&self, sink: &mut SpriteSink<'_>, page: &Arc<PublicationPage>) {
        let tick = page.id().tick;
        let mut extraction = Extraction {
            tick,
            live: page.len(),
            ..Extraction::default()
        };
        let mut projectiles = self.projectiles.borrow_mut();
        let mut flashes = self.flashes.borrow_mut();
        flashes.retain(|(_, expiry)| *expiry > tick);
        for event in page.structural_events() {
            if let StructuralChange::Despawn(handle) = *event
                && let Some((generation, body)) = projectiles.remove(&handle.slot)
                && generation == handle.generation
                && body.health > 1
            {
                flashes.push((position(&body), tick + 7));
            }
        }
        page.visit(|handle, identity| {
            let Some(body) = page.component::<Body>(handle) else {
                return;
            };
            let previous = page.previous_component::<Body>(handle);
            let mut current = row(body, identity.0);
            if body.kind == 2 {
                projectiles.insert(handle.slot, (handle.generation, *body));
            } else {
                let [x, y] = position(body);
                if flashes.iter().any(|([a, b], _)| (x - a).hypot(y - b) < 2.2) {
                    current.flash = [1.0, 0.67, 0.23, 0.88];
                }
            }
            if body.kind == 0 && body.player == 0 {
                self.camera.set([
                    previous.map_or_else(|| position(body), position),
                    position(body),
                ]);
            }
            extraction.mapped += 1;
            if handle.generation > 1 && previous.is_none() {
                extraction.reused += 1;
                extraction.skipped += 1;
            }
            let previous = previous.map(|body| SpritePose::from(row(body, identity.0)));
            sink.push_pair(
                handle.slot,
                handle.generation,
                identity.0,
                current,
                previous,
            );
        });
        self.extraction.set(extraction);
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "physical output extents fit the float mantissa"
    )]
    fn sprites(
        &self,
        _: SourceTick,
        alpha: f32,
        extent: [u32; 2],
    ) -> Result<SpriteFrame<'_>, FrameError> {
        let [previous, current] = self.camera.get();
        let centre = if self.close {
            [0, 1].map(|axis| previous[axis] + f64::from(alpha) * (current[axis] - previous[axis]))
        } else {
            [0.0; 2]
        };
        Ok(SpriteFrame {
            view: SpriteView {
                centre,
                pixels_per_unit: extent[0].min(extent[1]) as f32
                    / if self.close { 44.0 } else { 178.0 },
            },
            rows: &[],
        })
    }
}
