use std::sync::Arc;

use thrum_app::{GameError, OutputMetrics};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedSource, FontCandidate, FontScale, ProductLease,
};
use thrum_render::sprite::{EmbeddedSheet, NamedSheet, SpriteRect};
use thrum_ui::{
    Alignment, Anchor, Axis, Elision, Font, Image, InterfaceConfig, InterfaceSheets, Node,
    NodeKind, PaintInputs, PaintList, Text, TextOptions, Tree, Viewport,
};

const FONT: &str = "type";
const CARD: u32 = 1;
const TICK: u32 = 3;
const BODIES: u32 = 4;
const WHITE: Image = Image {
    sheet: 0,
    frame: 0,
    texels: [1, 1],
};

pub(super) struct Display {
    lease: ProductLease,
    scale: FontScale,
    output: OutputMetrics,
    active: Option<Layout>,
    prepared: Option<Layout>,
}

struct Layout {
    values: (u64, usize),
    viewport: Viewport,
    tree: Tree,
    paint: PaintList,
}

impl Display {
    pub(super) fn new(
        source: CookedSource,
        extent: [u32; 2],
    ) -> Result<(Self, CookedAssets), GameError> {
        let mut config = CookedConfig::new(source, Vec::new());
        config.lease_font(FONT);
        let scale = config.font_scale_request();
        let assets =
            CookedAssets::new(config).map_err(|error| GameError::Rejected(error.to_string()))?;
        Ok((
            Self {
                lease: assets.lease(),
                scale,
                output: OutputMetrics {
                    extent,
                    density: 1.0,
                    insets: [0.0; 4],
                },
                active: None,
                prepared: None,
            },
            assets,
        ))
    }

    pub(super) fn output(&mut self, output: OutputMetrics) {
        self.output = output;
        self.scale.set(if output.density < 1.5 {
            1
        } else if output.density < 2.5 {
            2
        } else {
            3
        });
    }

    pub(super) fn config(&self) -> InterfaceConfig {
        #[cfg_attr(
            target_arch = "wasm32",
            expect(
                clippy::arc_with_non_send_sync,
                reason = "the browser lease stays on its render thread"
            )
        )]
        let source = Arc::new(self.lease.clone());
        InterfaceConfig {
            sheets: vec![NamedSheet::pixel(
                "status fill",
                EmbeddedSheet {
                    extent: [3, 3],
                    rgba: Arc::from([
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    ]),
                    frames: Arc::from([SpriteRect {
                        origin: [1, 1],
                        size: [1, 1],
                    }]),
                    texels_per_unit: 1.0,
                },
            )],
            cooked: Some(InterfaceSheets {
                names: vec![FONT.into()],
                source,
            }),
            row_capacity: 64,
        }
    }

    pub(super) fn paint(
        &mut self,
        extent: [u32; 2],
        values: (u64, usize),
    ) -> [Option<PaintList>; 2] {
        let committed = self.lease.committed_font();
        if let Some(font) = &committed
            && self
                .prepared
                .as_ref()
                .is_some_and(|layout| layout.paint.stamp == Some(font.stamp))
        {
            self.active = self.prepared.take();
        }
        let active = committed
            .as_ref()
            .and_then(|font| layout(&mut self.active, font, self.output, extent, values).ok());
        let prepared = self.lease.font_candidate().and_then(|font| {
            match layout(&mut self.prepared, &font, self.output, extent, values) {
                Ok(paint) => {
                    self.lease.answer_font(font.stamp, Ok(()));
                    Some(paint)
                }
                Err(reason) => {
                    self.prepared = None;
                    self.lease.answer_font(font.stamp, Err(reason));
                    None
                }
            }
        });
        [active, prepared]
    }

    pub(super) fn shown(&self) -> Option<(Viewport, [[f64; 2]; 2])> {
        let stamp = self.lease.committed_font()?.stamp;
        let layout = self
            .prepared
            .iter()
            .chain(self.active.iter())
            .find(|layout| layout.paint.stamp == Some(stamp))?;
        let box_ = layout.tree.bounds(CARD)?;
        Some((layout.viewport, [box_.origin, box_.end()]))
    }
}

fn layout(
    slot: &mut Option<Layout>,
    font: &FontCandidate,
    output: OutputMetrics,
    extent: [u32; 2],
    values: (u64, usize),
) -> Result<PaintList, String> {
    let stamp = font.stamp;
    let viewport = Viewport {
        extent,
        scale: stamp.scale,
        insets: output
            .insets
            .map(|inset| inset * output.density / f64::from(stamp.scale)),
    };
    if let Some(layout) = slot
        && layout.values == values
        && layout.viewport == viewport
        && layout.paint.stamp == Some(stamp)
    {
        return Ok(layout.paint.clone());
    }
    let product = font.font(FONT).ok_or("Crowd status font is missing")?;
    let mut tree = slot
        .take()
        .map_or_else(status_tree, |layout| Ok(layout.tree))
        .map_err(|error| error.to_string())?;
    tree.set_text(TICK, &format!("Tick  {}", values.0));
    tree.set_text(BODIES, &format!("Bodies  {}", values.1));
    let paint = tree
        .frame(
            viewport,
            PaintInputs {
                stamp: Some(stamp),
                fonts: &[Font { product, sheet: 1 }],
                white: WHITE,
            },
        )
        .map_err(|error| error.to_string())?;
    *slot = Some(Layout {
        values,
        viewport,
        tree,
        paint: paint.clone(),
    });
    Ok(paint)
}

fn status_tree() -> Result<Tree, thrum_ui::TreeError> {
    use thrum_ui::{Size, SizeRule};
    let mut root = Node::new(0, NodeKind::Overlay);
    root.size = [Size::new(SizeRule::Fraction(1.0)); 2];
    root.padding = [12.0; 4];
    let mut card = Node::new(CARD, NodeKind::Overlay);
    card.size = [
        Size::new(SizeRule::Fixed(150.0)),
        Size::new(SizeRule::Fixed(68.0)),
    ];
    let mut background = Node::new(2, NodeKind::Fill([0.011, 0.028, 0.052, 1.0]));
    background.size = [Size::new(SizeRule::Fraction(1.0)); 2];
    let mut counters = Node::new(
        5,
        NodeKind::Stack {
            axis: Axis::Column,
            gap: 6.0,
            align: Anchor::Start,
        },
    );
    counters.anchor = [Anchor::Point(12.0), Anchor::Centre];
    counters.children = vec![
        label(TICK, [0.69, 0.90, 1.0, 1.0]),
        label(BODIES, [0.92, 0.97, 1.0, 1.0]),
    ];
    card.children = vec![background, counters];
    root.children.push(card);
    Tree::new(root)
}

fn label(id: u32, tint: [f32; 4]) -> Node {
    Node::new(
        id,
        NodeKind::Text(Text {
            value: String::new(),
            font: 0,
            options: TextOptions {
                points: 16.0,
                scale: 1,
                extent: [0; 2],
                alignment: Alignment::Left,
                wrap: false,
                elision: Elision::End,
                tabular_digits: true,
                outlined: false,
            },
            tint,
            outline: [0.0; 4],
        }),
    )
}
