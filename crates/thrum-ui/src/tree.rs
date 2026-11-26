//! Retained interface nodes, point-space boxes and cached paint.

mod layout;
mod paint;

use std::{collections::BTreeMap, fmt};

use thrum_asset::FontProduct;

use crate::{PaintList, PaintStamp, SpriteRow, TextError, TextOptions};

/// Stable identity supplied by the game.
pub type NodeId = u32;

/// Insets or padding in points, in left, top, right, bottom order.
pub type Insets = [f64; 4];

/// A box in points, with its origin at the upper left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Box2 {
    /// Left and top edges.
    pub origin: [f64; 2],
    /// Width and height.
    pub size: [f64; 2],
}

impl Box2 {
    /// Right and bottom edges.
    pub fn end(self) -> [f64; 2] {
        std::array::from_fn(|axis| self.origin[axis] + self.size[axis])
    }

    fn inset(self, padding: Insets) -> Self {
        let leading: [f64; 2] = std::array::from_fn(|axis| padding[axis].min(self.size[axis]));
        Self {
            origin: std::array::from_fn(|axis| self.origin[axis] + leading[axis]),
            size: std::array::from_fn(|axis| {
                (self.size[axis] - leading[axis] - padding[axis + 2]).max(0.0)
            }),
        }
    }
}

/// The rule for a node's width or height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizeRule {
    /// A length in points.
    Fixed(f64),
    /// Fit the contents.
    Fit,
    /// A fraction of the parent's inner length.
    Fraction(f64),
    /// A positive weight in a stack's remaining main-axis space.
    Share(f64),
}

/// A size and its authored limits in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    /// How to choose the length before applying limits.
    pub rule: SizeRule,
    /// Minimum length.
    pub least: f64,
    /// Maximum length.
    pub most: f64,
}

impl Size {
    /// Use the rule with no extra limits.
    pub const fn new(rule: SizeRule) -> Self {
        Self {
            rule,
            least: 0.0,
            most: f64::INFINITY,
        }
    }

    /// Fix a size in points.
    pub const fn fixed(points: f64) -> Self {
        Self::new(SizeRule::Fixed(points))
    }

    fn constrained(self, value: f64) -> f64 {
        value.clamp(self.least, self.most)
    }
}

/// Vertical or horizontal stacking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// Stack left to right.
    Row,
    /// Stack top to bottom.
    Column,
}

impl Axis {
    const fn index(self) -> usize {
        match self {
            Self::Row => 0,
            Self::Column => 1,
        }
    }
}

/// Placement on one axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Anchor {
    /// Align leading edges.
    #[default]
    Start,
    /// Align centres.
    Centre,
    /// Align trailing edges.
    End,
    /// Place the leading edge at this point relative to the parent's origin.
    Point(f64),
}

impl Anchor {
    fn offset(self, space: f64, length: f64) -> f64 {
        match self {
            Self::Start => 0.0,
            Self::Centre => (space - length) * 0.5,
            Self::End => space - length,
            Self::Point(point) => point,
        }
    }
}

/// A drawable frame and its extent in the source sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Image {
    /// Sheet slot in the paint list.
    pub sheet: u32,
    /// Frame index in that sheet.
    pub frame: u32,
    /// Frame dimensions in texels.
    pub texels: [u32; 2],
}

/// Text and style for one node.
#[derive(Clone, Debug)]
pub struct Text {
    /// Value displayed by the node.
    pub value: String,
    /// Index in the tree's font inputs.
    pub font: usize,
    /// Text style. The tree supplies the scale and fitted extent.
    pub options: TextOptions,
    /// Linear colour.
    pub tint: [f32; 4],
    /// Linear outline colour.
    pub outline: [f32; 4],
}

/// A node's contents and child placement rule.
#[derive(Clone, Debug)]
pub enum NodeKind {
    /// Lay out child boxes in one direction.
    Stack {
        /// Stacking direction.
        axis: Axis,
        /// Gap between visible children in points.
        gap: f64,
        /// Alignment on the other axis.
        align: Anchor,
    },
    /// Place children by their anchors.
    Overlay,
    /// Cooked text.
    Text(Text),
    /// Art drawn into the fitted box.
    Image(Image),
    /// A tint applied to the white tile.
    Fill([f32; 4]),
    /// A panel using nine frames in row-major order. Children use overlay placement.
    Panel([Image; 9]),
    /// A coloured track and fill.
    Bar {
        /// Fraction of the track occupied by its fill.
        fraction: f64,
        /// Linear colour of the track.
        track: [f32; 4],
        /// Linear colour of the fill.
        fill: [f32; 4],
    },
}

/// A node declaration. Supply one root to the retained tree.
#[derive(Clone, Debug)]
pub struct Node {
    /// Identity unique within the tree.
    pub id: NodeId,
    /// Drawing and child placement rule.
    pub kind: NodeKind,
    /// Width and height.
    pub size: [Size; 2],
    /// Insets on a container.
    pub padding: Insets,
    /// Placement inside an overlay.
    pub anchor: [Anchor; 2],
    /// Hidden nodes take no room and produce no rows.
    pub shown: bool,
    /// Take the whole output extent, including the safe-area insets.
    pub full_bleed: bool,
    /// Children in stacking and paint order.
    pub children: Vec<Self>,
}

impl Node {
    /// Declare a visible, content-fitting node.
    pub const fn new(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            kind,
            size: [Size::new(SizeRule::Fit); 2],
            padding: [0.0; 4],
            anchor: [Anchor::Start; 2],
            shown: true,
            full_bleed: false,
            children: Vec::new(),
        }
    }
}

/// Physical output size and the interface's point grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Output extent in physical pixels.
    pub extent: [u32; 2],
    /// Safe-area insets in interface points.
    pub insets: Insets,
    /// Interface scale, 1, 2 or 3.
    pub scale: u32,
}

impl Viewport {
    /// Full extent in points.
    pub fn full(self) -> Box2 {
        Box2 {
            origin: [0.0; 2],
            size: self
                .extent
                .map(|side| f64::from(side) / f64::from(self.scale.clamp(1, 3))),
        }
    }

    /// Usable extent in points.
    pub fn safe(self) -> Box2 {
        self.full().inset(self.insets.map(nonnegative))
    }
}

/// A cooked font and its sheet in the current layout's scale and publication.
#[derive(Clone, Copy)]
pub struct Font<'a> {
    /// Cooked font metrics and frame dimensions.
    pub product: &'a FontProduct,
    /// Sheet slot in the paint list.
    pub sheet: u32,
}

/// The exact assets used for a layout and its paint list.
#[derive(Clone, Copy)]
pub struct PaintInputs<'a> {
    /// Cooked sheet publication and scale, if there are leased sheets.
    pub stamp: Option<PaintStamp>,
    /// Font slots referred to by text nodes.
    pub fonts: &'a [Font<'a>],
    /// A white one-texel tile for fills and bars.
    pub white: Image,
}

/// A declaration or font input refused by the tree.
#[derive(Clone, Copy, Debug)]
pub enum TreeError {
    /// Two nodes use the same identity.
    Duplicate(NodeId),
    /// An invalid size, anchor, padding, gap or viewport.
    Geometry(NodeId),
    /// A child takes a fraction of a parent that fits its children on that axis.
    CircularFraction(NodeId),
    /// A drawable leaf cannot have children.
    LeafChildren(NodeId),
    /// A text node names an unavailable font slot.
    Font(NodeId),
    /// A text node names a size or scale that was not cooked.
    Text(NodeId, TextError),
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(f, "duplicate interface node {id}"),
            Self::Geometry(id) => write!(f, "invalid geometry for interface node {id}"),
            Self::CircularFraction(id) => {
                write!(f, "node {id} takes a fraction of a fitting parent")
            }
            Self::LeafChildren(id) => write!(f, "interface leaf {id} has children"),
            Self::Font(id) => write!(f, "font missing for interface node {id}"),
            Self::Text(id, error) => write!(f, "interface text {id}: {error}"),
        }
    }
}

impl std::error::Error for TreeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Text(_, error) => Some(error),
            _ => None,
        }
    }
}

/// Counts of actual tree work, accumulated over its lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeStats {
    /// Full layouts.
    pub layouts: u64,
    /// Paint lists built.
    pub lists: u64,
    /// Rows generated for nodes whose paint changed.
    pub rows: u64,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "visibility, fitting and paint invalidation are independent"
)]
struct Entry {
    node: Node,
    parent: Option<usize>,
    children: Vec<usize>,
    natural: [f64; 2],
    rect: Box2,
    visible: bool,
    overflow: bool,
    sizes_overflow: bool,
    elided: bool,
    paint_dirty: bool,
    tabular: bool,
    rows: Vec<SpriteRow>,
}

/// Retained boxes and paint for one active or prepared layout.
pub struct Tree {
    nodes: Vec<Entry>,
    ids: BTreeMap<NodeId, usize>,
    layout_dirty: bool,
    paint_dirty: bool,
    viewport: Option<Viewport>,
    stamp: Option<PaintStamp>,
    paint: Option<PaintList>,
    stats: TreeStats,
}

impl Tree {
    /// Validate and retain a declaration.
    ///
    /// # Errors
    /// Returns the identity of the first invalid node.
    pub fn new(root: Node) -> Result<Self, TreeError> {
        let mut tree = Self {
            nodes: Vec::new(),
            ids: BTreeMap::new(),
            layout_dirty: true,
            paint_dirty: true,
            viewport: None,
            stamp: None,
            paint: None,
            stats: TreeStats::default(),
        };
        tree.insert(root, None)?;
        Ok(tree)
    }

    fn insert(&mut self, mut node: Node, parent: Option<usize>) -> Result<usize, TreeError> {
        let id = node.id;
        if node.size.iter().any(|size| !valid_size(*size))
            || node.padding.into_iter().any(|value| !valid_length(value))
            || node.anchor.iter().any(|a| !valid_anchor(*a))
            || matches!(node.kind, NodeKind::Stack { gap, align, .. }
                if !valid_length(gap) || !valid_anchor(align))
        {
            return Err(TreeError::Geometry(id));
        }
        if !node.full_bleed
            && parent.is_some_and(|parent| {
                (0..2).any(|axis| {
                    self.nodes[parent].node.size[axis].rule == SizeRule::Fit
                        && matches!(node.size[axis].rule, SizeRule::Fraction(_))
                })
            })
        {
            return Err(TreeError::CircularFraction(id));
        }
        if matches!(node.kind, NodeKind::Stack { .. })
            && let Some(full_bleed) = node.children.iter().find(|child| child.full_bleed)
        {
            return Err(TreeError::Geometry(full_bleed.id));
        }
        if !node.children.is_empty()
            && !matches!(
                node.kind,
                NodeKind::Stack { .. } | NodeKind::Overlay | NodeKind::Panel(_)
            )
        {
            return Err(TreeError::LeafChildren(id));
        }
        let index = self.nodes.len();
        if self.ids.insert(id, index).is_some() {
            return Err(TreeError::Duplicate(id));
        }
        let children = std::mem::take(&mut node.children);
        self.nodes.push(Entry {
            node,
            parent,
            children: Vec::with_capacity(children.len()),
            natural: [0.0; 2],
            rect: Box2::default(),
            visible: false,
            overflow: false,
            sizes_overflow: false,
            elided: false,
            paint_dirty: true,
            tabular: false,
            rows: Vec::new(),
        });
        for child in children {
            let child = self.insert(child, Some(index))?;
            self.nodes[index].children.push(child);
        }
        Ok(index)
    }

    /// Last laid-out box of a visible node.
    pub fn bounds(&self, id: NodeId) -> Option<Box2> {
        let entry = &self.nodes[*self.ids.get(&id)?];
        entry.visible.then_some(entry.rect)
    }

    /// Nodes that could not fit their requested geometry.
    pub fn overflows(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|e| e.visible && e.overflow)
            .map(|e| e.node.id)
    }

    /// Nodes whose text was elided or could not fit even one line.
    pub fn elisions(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|e| e.visible && e.elided)
            .map(|e| e.node.id)
    }

    /// Counts from the actual layout and paint paths.
    pub const fn stats(&self) -> TreeStats {
        self.stats
    }

    /// Change a bar's fraction without changing its box.
    pub fn set_fraction(&mut self, id: NodeId, fraction: f64) {
        let Some(&index) = self.ids.get(&id) else {
            return;
        };
        let entry = &mut self.nodes[index];
        let fraction = nonnegative(fraction).min(1.0);
        if let NodeKind::Bar {
            fraction: value, ..
        } = &mut entry.node.kind
            && value.to_bits() != fraction.to_bits()
        {
            *value = fraction;
            entry.paint_dirty = true;
            self.paint_dirty |= entry.visible;
        }
    }

    /// Change the tint of a fill or text node.
    pub fn set_tint(&mut self, id: NodeId, tint: [f32; 4]) {
        let Some(&index) = self.ids.get(&id) else {
            return;
        };
        let entry = &mut self.nodes[index];
        let value = match &mut entry.node.kind {
            NodeKind::Fill(value) => value,
            NodeKind::Text(text) => &mut text.tint,
            _ => return,
        };
        if value.map(f32::to_bits) != tint.map(f32::to_bits) {
            *value = tint;
            entry.paint_dirty = true;
            self.paint_dirty |= entry.visible;
        }
    }

    /// Change visibility and reclaim the hidden subtree's space.
    pub fn set_shown(&mut self, id: NodeId, shown: bool) {
        if let Some(&index) = self.ids.get(&id)
            && self.nodes[index].node.shown != shown
        {
            self.nodes[index].node.shown = shown;
            // A pending ancestor change already dirties layout. Otherwise
            // its last laid-out visibility tells whether this child can take space.
            self.layout_dirty |= self.nodes[index]
                .parent
                .is_none_or(|parent| self.nodes[parent].visible);
        }
    }

    /// Move an overlay child to a point in its parent's box.
    pub fn set_anchor(&mut self, id: NodeId, anchor: [Anchor; 2]) {
        if let Some(&index) = self.ids.get(&id)
            && anchor.into_iter().all(valid_anchor)
            && self.nodes[index].node.anchor != anchor
        {
            self.nodes[index].node.anchor = anchor;
            let entry = &self.nodes[index];
            self.layout_dirty |= entry.visible
                && !entry.node.full_bleed
                && entry.parent.is_some_and(|parent| {
                    !matches!(self.nodes[parent].node.kind, NodeKind::Stack { .. })
                });
        }
    }

    /// Update a text value. Only intrinsic sizes can depend on its glyphs.
    pub fn set_text(&mut self, id: NodeId, value: &str) {
        let Some(&index) = self.ids.get(&id) else {
            return;
        };
        let entry = &mut self.nodes[index];
        if let NodeKind::Text(text) = &mut entry.node.kind
            && text.value != value
        {
            let intrinsic = entry.visible
                && index != 0
                && !entry.node.full_bleed
                && entry
                    .node
                    .size
                    .iter()
                    .any(|size| size.rule == SizeRule::Fit && size.least < size.most);
            self.layout_dirty |=
                intrinsic && !(entry.tabular && same_digit_pattern(&text.value, value));
            text.value.clear();
            text.value.push_str(value);
            entry.paint_dirty = true;
            self.paint_dirty |= entry.visible;
        }
    }
}

fn same_digit_pattern(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .all(|(a, b)| a == b || a.is_ascii_digit() && b.is_ascii_digit())
}

fn nonnegative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn valid_anchor(anchor: Anchor) -> bool {
    !matches!(anchor, Anchor::Point(point) if !point.is_finite())
}

fn valid_length(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_size(size: Size) -> bool {
    size.least.is_finite()
        && size.least >= 0.0
        && size.most >= size.least
        && match size.rule {
            SizeRule::Fixed(value) => valid_length(value),
            SizeRule::Fraction(value) => (0.0..=1.0).contains(&value),
            SizeRule::Share(weight) => weight.is_finite() && weight > 0.0,
            SizeRule::Fit => true,
        }
}
