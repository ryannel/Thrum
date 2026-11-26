//! The V1 status display's interface tree, shared by the scene and its proofs.

/// CPU reference for submitted status-display qualification.
#[cfg(not(target_arch = "wasm32"))]
pub mod proof;

use thrum_render::sprite::{EmbeddedSheet, NamedSheet, SpriteRect};
use thrum_ui::{
    Alignment, Anchor, Axis, Elision, Image, Node, NodeKind, Size, SizeRule, Text, TextOptions,
    Tree, TreeError, Viewport,
};

const WARM: [f32; 4] = [1.0, 0.77, 0.44, 1.0];
const WHITE: [f32; 4] = [0.87, 0.93, 1.0, 1.0];
const MUTED: [f32; 4] = [0.47, 0.66, 0.77, 1.0];
const PANEL_SIZE: [f64; 2] = [304.0, 116.0];
const PANEL_MARGIN: f64 = 8.0;
const LARGE_MULTIPLIER: f64 = 2.0;
/// Identity of the health bar in the status tree.
pub const BAR: u32 = 6;
/// Identity of the timer in the status tree.
pub const TIMER: u32 = 7;
/// Identities of the score, gems and wave values in their displayed order.
pub const COUNTS: [u32; 3] = [20, 24, 28];
/// Slot of the white tile used for status fills.
pub const WHITE_TILE: Image = Image {
    sheet: 0,
    frame: 0,
    texels: [1, 1],
};

/// Source rectangles for the nine-slice status panel.
/// # Panics
/// The nine tile indices are guaranteed to fit in the frame type.
pub fn panel_tiles() -> [Image; 9] {
    std::array::from_fn(|index| {
        let axis = |cell| if cell == 1 { 1 } else { 2 };
        Image {
            sheet: 0,
            frame: 1 + u32::try_from(index).expect("nine tiles"),
            // A stretch axis needs only one source texel. This also keeps
            // the tile on its pixel-sampling path at the smallest panel.
            texels: [axis(index % 3), axis(index / 3)],
        }
    })
}

/// Original status panel art and the white fill tile.
pub fn panel_sheet() -> NamedSheet {
    let mut rgba = vec![255; 19 * 2 * 4];
    let mut frames = vec![SpriteRect {
        origin: [0, 0],
        size: [1; 2],
    }];
    for (index, tile) in panel_tiles().into_iter().enumerate() {
        let x_start = tile.frame * 2 - 1;
        frames.push(SpriteRect {
            origin: [x_start, 0],
            size: tile.texels,
        });
        let column = index % 3;
        let row = index / 3;
        for y in 0..tile.texels[1] {
            for x in 0..tile.texels[0] {
                let pixel = ((19 * y + x_start + x) * 4) as usize;
                let top = row == 0 && y == 0;
                let border =
                    top || row == 2 && y == 1 || column == 0 && x == 0 || column == 2 && x == 1;
                rgba[pixel..pixel + 4].copy_from_slice(if top {
                    &[86, 123, 140, 255]
                } else if border {
                    &[51, 74, 91, 255]
                } else {
                    &[16, 25, 37, 255]
                });
            }
        }
    }
    let mut sheet = NamedSheet::pixel(
        "status panel",
        EmbeddedSheet {
            extent: [19, 2],
            rgba: rgba.into(),
            frames: frames.into(),
            texels_per_unit: 1.0,
        },
    );
    sheet.tiles = (0..10).collect();
    sheet
}

/// The status display's authored sizes for the current presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    /// Standard panel and text in interface points.
    #[default]
    Standard,
    /// Larger glyph strikes and spacing for a dense display drawing at scale one.
    Large,
}

impl Appearance {
    /// Preserve the display-point reading size when a density-two display uses
    /// scale one. Use the compact panel if the safe area cannot fit the large one.
    pub fn for_output(viewport: Viewport, density: f64) -> Self {
        let [width, height] = viewport.safe().size;
        let [min_width, min_height] =
            PANEL_SIZE.map(|side| (side + 2.0 * PANEL_MARGIN) * LARGE_MULTIPLIER);
        if viewport.scale == 1 && density >= 1.5 && width >= min_width && height >= min_height {
            Self::Large
        } else {
            Self::Standard
        }
    }
}

/// Build the standard status display from the published health mark.
/// # Errors
/// Returns invalid node geometry or duplicate stable identity.
pub fn tree(badge: Image) -> Result<Tree, TreeError> {
    tree_with_appearance(badge, Appearance::Standard)
}

/// Build the status display with the selected cooked text sizes and spacing.
/// # Errors
/// Returns invalid node geometry or duplicate stable identity.
pub fn tree_with_appearance(badge: Image, appearance: Appearance) -> Result<Tree, TreeError> {
    let (size, caption, value) = match appearance {
        Appearance::Standard => (1.0, 12, 16),
        Appearance::Large => (LARGE_MULTIPLIER, 24, 24),
    };
    let mut root = Node::new(0, NodeKind::Overlay);
    root.size = [FILL; 2];
    root.padding = [PANEL_MARGIN * size; 4];
    let mut panel = Node::new(1, NodeKind::Panel(panel_tiles()));
    panel.size = [
        Size {
            most: PANEL_SIZE[0] * size,
            ..FILL
        },
        Size::fixed(PANEL_SIZE[1] * size),
    ];
    panel.padding = [10.0, 8.0, 10.0, 8.0].map(|side| side * size);
    panel.anchor[0] = Anchor::Centre;
    let mut column = stack(2, Axis::Column, 4.0 * size);
    column.size = [FILL, FILL];
    let mut labels = stack(3, Axis::Row, 5.0 * size);
    labels.size = [FILL, Size::fixed(21.0 * size)];
    let mut icon = Node::new(4, NodeKind::Image(badge));
    icon.size = badge.texels.map(|side| Size::fixed(f64::from(side) * size));
    let mut health = label(5, "HEALTH", caption, WARM, false, Alignment::Left);
    health.size[0] = FILL;
    labels.children = vec![
        icon,
        health,
        label(TIMER, "00:00", caption, WHITE, true, Alignment::Right),
    ];
    let mut bar = Node::new(
        BAR,
        NodeKind::Bar {
            fraction: 1.0,
            track: [0.065, 0.10, 0.14, 1.0],
            fill: [0.23, 0.88, 0.51, 1.0],
        },
    );
    bar.size = [FILL, Size::fixed(6.0 * size)];
    let mut counters = stack(8, Axis::Row, 6.0 * size);
    counters.size = [FILL, Size::fixed(64.0 * size)];
    for (counter, name) in COUNTS.into_iter().zip(["SCORE", "GEMS FOUND", "WAVE"]) {
        let mut group = stack(counter - 2, Axis::Column, size);
        group.size = [FILL, Size::fixed(64.0 * size)];
        let mut line = Node::new(counter - 1, NodeKind::Fill([0.11, 0.19, 0.24, 1.0]));
        line.size = [FILL, Size::fixed(size)];
        let mut caption = label(counter + 1, name, caption, MUTED, false, Alignment::Centre);
        if let NodeKind::Text(text) = &mut caption.kind {
            text.options.wrap = true;
            caption.size[0] = FILL;
        }
        group.children = vec![
            line,
            label(counter, "000", value, WHITE, true, Alignment::Centre),
            caption,
        ];
        counters.children.push(group);
    }
    column.children = vec![labels, bar, counters];
    panel.children.push(column);
    root.children.push(panel);
    Tree::new(root)
}

const FILL: Size = Size::new(SizeRule::Share(1.0));

fn stack(id: u32, axis: Axis, gap: f64) -> Node {
    Node::new(
        id,
        NodeKind::Stack {
            axis,
            gap,
            align: Anchor::Centre,
        },
    )
}

fn label(
    id: u32,
    text: &str,
    points: u8,
    tint: [f32; 4],
    tabular_digits: bool,
    alignment: Alignment,
) -> Node {
    Node::new(
        id,
        NodeKind::Text(Text {
            value: text.into(),
            font: 0,
            options: TextOptions {
                points: f32::from(points),
                scale: 1,
                extent: [0; 2],
                alignment,
                wrap: false,
                elision: Elision::End,
                tabular_digits,
                outlined: false,
            },
            tint,
            outline: [0.0; 4],
        }),
    )
}

/// Apply the status values for a simulation tick.
pub fn update(tree: &mut Tree, tick: u64) {
    update_values(tree, tick, false);
}

pub(crate) fn update_values(tree: &mut Tree, tick: u64, every_tick: bool) {
    // One four-second cycle at the field's 60 Hz tick rate.
    let step = u16::try_from(tick % 240).expect("cycle fits u16");
    tree.set_fraction(BAR, f64::from(step.abs_diff(120)) / 120.0);
    let seconds = if every_tick { tick } else { tick / 60 };
    tree.set_text(
        TIMER,
        &format!("{:02}:{:02}", seconds / 60 % 100, seconds % 60),
    );
    let counts = if every_tick {
        let value = tick % 1000;
        [value, value * 3, value * 7]
    } else {
        [tick / 6, tick / 19, 1 + tick / 240]
    };
    for (id, count) in COUNTS.into_iter().zip(counts) {
        tree.set_text(id, &format!("{:03}", count % 1000));
    }
}

/// Measure one layout and one paint build for each requested size, in the
/// caller's monotonic milliseconds. Geometry and rows are the same on both hosts.
pub(crate) fn measure_layout(now_ms: impl Fn() -> f64) -> Result<Vec<[f64; 3]>, TreeError> {
    let view = Viewport {
        extent: [1920, 1080],
        insets: [0.0; 4],
        scale: 2,
    };
    let inputs = thrum_ui::PaintInputs {
        stamp: None,
        fonts: &[],
        white: WHITE_TILE,
    };
    let mut report = Vec::new();
    for count in [100, 500, 2000] {
        let mut root = Node::new(0, NodeKind::Overlay);
        let mut lane = stack(1, Axis::Column, 0.0);
        lane.size = [FILL; 2];
        for id in 2..count {
            let mut child = Node::new(id, NodeKind::Fill([1.0; 4]));
            child.size = [Size::fixed(8.0), Size::new(SizeRule::Share(1.0))];
            lane.children.push(child);
        }
        root.children.push(lane);
        let mut tree = Tree::new(root)?;
        for arm in 0..32 {
            let view = Viewport {
                extent: [view.extent[0], view.extent[1] + arm],
                ..view
            };
            let start = now_ms();
            tree.prepare_layout(view, inputs)?;
            let laid_out = now_ms();
            let paint = tree.frame(view, inputs)?;
            let painted = now_ms();
            std::hint::black_box(paint);
            report.push([f64::from(count), laid_out - start, painted - laid_out]);
        }
    }
    Ok(report)
}
