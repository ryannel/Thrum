//! Compare the real Metal interface stage with an independent nine-slice and bar compositor.

#[expect(
    dead_code,
    reason = "reuse the small inline world from the sheet publication check"
)]
mod sprite_sheet_fixture;
mod status_metal;

use status_metal as metal;
use v1_game::interface_status as status;

use std::{sync::Arc, time::Duration};

use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SpriteSceneRequests},
    sprite::{EmbeddedSheet, SpriteFrame, SpriteRect, SpriteScene, SpriteView},
    synthetic::FrameLighting,
};
use thrum_ui::{
    Anchor, Axis, Image, InterfaceConfig, Node, NodeKind, PaintInputs, PaintList, Size, SpriteRow,
    Tree, Viewport,
};

use sprite_sheet_fixture::World;
use status::proof::{Reference, Sheet};

struct Scene {
    paint: Option<PaintList>,
    config: InterfaceConfig,
    background: [SpriteRow; 1],
}

impl SceneSource<SourceTick> for Scene {
    fn extract(&self, _: &mut RenderExtractor, _: &SourceTick) -> ExtractReport {
        unreachable!("only 2D")
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        unreachable!("only 2D")
    }

    fn sprites(&self, _: SourceTick, _: f32, _: [u32; 2]) -> Result<SpriteFrame<'_>, FrameError> {
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [0.0; 2],
                pixels_per_unit: 1.0,
            },
            rows: &self.background,
        })
    }

    fn interface_config(&self) -> Option<InterfaceConfig> {
        Some(self.config.clone())
    }

    fn paint_list(&mut self, _: [u32; 2]) -> Option<PaintList> {
        self.paint.clone()
    }
}

const EXTENT: [u32; 2] = [144, 72];
const TRACK: [f32; 4] = [0.075, 0.19, 0.5, 1.0];
const FILL: [f32; 4] = [0.86, 0.2, 0.06, 1.0];

fn tree(panel_size: [u32; 2]) -> Tree {
    let mut root = Node::new(
        0,
        NodeKind::Stack {
            axis: Axis::Row,
            gap: 2.5,
            align: Anchor::Start,
        },
    );
    root.padding = [3.5, 4.5, 0.0, 0.0];
    let mut panel = Node::new(1, NodeKind::Panel(status::panel_tiles()));
    panel.size = panel_size.map(|side| Size::fixed(f64::from(side)));
    let mut bar = Node::new(
        2,
        NodeKind::Bar {
            fraction: 0.0,
            track: TRACK,
            fill: FILL,
        },
    );
    bar.size = [Size::fixed(9.0), Size::fixed(3.0)];
    let mut image = Node::new(
        3,
        NodeKind::Image(Image {
            sheet: 0,
            frame: 1,
            texels: [2; 2],
        }),
    );
    image.size = [Size::fixed(2.0); 2];
    let mut fill = Node::new(4, NodeKind::Fill([0.34, 0.3, 0.15, 0.5]));
    fill.size = [Size::fixed(2.0); 2];
    root.children = vec![panel, bar, image, fill];
    Tree::new(root).expect("test tree")
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the reference uses tiny positive integer coordinates"
)]
fn expected(
    tree: &Tree,
    reference: &mut Reference,
    sheet: &EmbeddedSheet,
    scale: u32,
    fraction: f64,
) {
    let bounds = |id| {
        let rect = tree.bounds(id).expect("laid out");
        [rect.origin, rect.end()]
            .map(|edge| edge.map(|value| (value * f64::from(scale)).round() as u32))
    };
    let draw =
        |reference: &mut Reference, frame, origin: [u32; 2], size: [u32; 2], tint: [f32; 4]| {
            if !size.contains(&0) {
                let source = sheet.frames[frame as usize].size;
                let mut row = SpriteRow::new(frame, origin.map(f64::from));
                row.tint = tint;
                row.scale = std::array::from_fn(|axis| size[axis] as f32 / source[axis] as f32);
                reference.draw(row, Sheet::Embedded(sheet));
            }
        };
    let [[left, top], [right, bottom]] = bounds(1);
    let corner = 2 * scale;
    let columns = [left, left + corner, right - corner, right];
    let rows = [top, top + corner, bottom - corner, bottom];
    for i in 0..9 {
        let [x, y] = [i % 3, i / 3];
        draw(
            reference,
            1 + u32::try_from(i).expect("nine tiles"),
            [columns[x], rows[y]],
            [columns[x + 1] - columns[x], rows[y + 1] - rows[y]],
            [1.0; 4],
        );
    }
    let span = |id| {
        let [start, end] = bounds(id);
        (start, [end[0] - start[0], end[1] - start[1]])
    };
    let (start, size) = span(2);
    draw(reference, 0, start, size, TRACK);
    draw(
        reference,
        0,
        start,
        [(f64::from(size[0]) * fraction).round() as u32, size[1]],
        FILL,
    );
    for (id, frame, colour) in [(3, 1, [1.0; 4]), (4, 0, [0.34, 0.3, 0.15, 0.5])] {
        let (start, size) = span(id);
        draw(reference, frame, start, size, colour);
    }
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "requires Metal for the submitted pixel comparison"
)]
fn snapped_paint_matches_nine_slice_fill_image_and_bar_at_each_scale() {
    let gpu = metal::open();
    let panel = status::panel_sheet();
    let mut sprites = SpriteScene::new(
        EmbeddedSheet {
            extent: [1, 1],
            rgba: Arc::new([255; 4]),
            frames: Arc::new([SpriteRect {
                origin: [0; 2],
                size: [1; 2],
            }]),
            texels_per_unit: 1.0,
        },
        1,
    );
    sprites.sheets[0].tiles.push(0);
    let mut background = SpriteRow::new(0, [-72.0, 36.0]);
    background.tint = [0.51, 0.0, 0.42, 1.0];
    background.scale = [144.0, 72.0];
    let scene = Scene {
        paint: None,
        config: InterfaceConfig {
            sheets: vec![panel.clone()],
            cooked: None,
            row_capacity: 24,
        },
        background: [background],
    };
    let mut host = pollster::block_on(FrameHost::new_2d(
        &gpu.capabilities,
        &gpu.queue,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(thrum_render::present::SurfaceProfile {
                extent: EXTENT,
                format: metal::FORMAT,
            }),
        },
        World(0),
        scene,
        sprites,
        SpriteSceneRequests::default(),
    ))
    .expect("2D interface host");
    host.enable_submitted_capture(1).expect("capture");
    let drawable = metal::drawable(&gpu, EXTENT);
    let mut now = Duration::ZERO;
    let mut draw = |host: &mut _| {
        now += Duration::from_millis(16);
        metal::submit(host, &drawable, EXTENT, now)
    };
    let plain = draw(&mut host);
    for scale in 1..=3 {
        for size in [[4, 4], [5, 6], [13, 10], [21, 11]] {
            let mut tree = tree(size);
            for fraction in [0.0, 0.04, 1.0 / 3.0, 0.5, 0.96, 1.0] {
                tree.set_fraction(2, fraction);
                let paint = tree
                    .frame(
                        Viewport {
                            extent: EXTENT,
                            scale,
                            insets: [0.0; 4],
                        },
                        PaintInputs {
                            stamp: None,
                            fonts: &[],
                            white: status::WHITE_TILE,
                        },
                    )
                    .expect("primitive layout");
                assert_eq!(tree.overflows().count(), 0);
                assert!(
                    paint
                        .rows
                        .iter()
                        .all(|row| row.position.iter().all(|x| x.fract() == 0.0))
                );
                host.continuous_source_mut().paint = Some(paint);
                let image = draw(&mut host);
                let mut reference = Reference::new(EXTENT, plain.rgba.clone());
                expected(&tree, &mut reference, &panel.sheet, scale, fraction);
                assert_eq!(
                    reference.first_difference(&image.rgba, [[0; 2], EXTENT]),
                    None,
                    "scale {scale}, panel {size:?}, fraction {fraction}"
                );
            }
            let unchanged = draw(&mut host);
            assert_eq!(
                unchanged
                    .sample
                    .interface
                    .expect("interface stats")
                    .uploaded_bytes,
                0
            );
        }
    }
}
