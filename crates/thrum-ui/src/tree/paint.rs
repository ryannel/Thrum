use super::{
    Box2, Image, NodeId, NodeKind, PaintInputs, Text, Tree, TreeError, Viewport, nonnegative,
};
use crate::{PaintList, SpriteRow, TextOptions, TextRun};

impl Tree {
    /// Make current point-space boxes available without building the paint list.
    ///
    /// # Errors
    /// Returns an invalid scale or an unavailable font input.
    pub fn prepare_layout(
        &mut self,
        viewport: Viewport,
        inputs: PaintInputs<'_>,
    ) -> Result<(), TreeError> {
        if !(1..=3).contains(&viewport.scale)
            || inputs.stamp.is_some_and(|s| s.scale != viewport.scale)
        {
            return Err(TreeError::Geometry(self.nodes[0].node.id));
        }
        if self.viewport != Some(viewport) || self.stamp != inputs.stamp {
            self.layout_dirty = true;
        }
        if self.layout_dirty {
            self.layout(viewport, inputs)?;
            self.layout_dirty = false;
            self.paint_dirty = true;
            self.viewport = Some(viewport);
            self.stamp = inputs.stamp;
        }
        Ok(())
    }

    /// Update the boxes and paint list only as needed. The immutable list can be reused
    /// by the frame host, and the caller must keep each resource set under its own stamp.
    ///
    /// # Errors
    /// Returns an invalid scale, missing font or unavailable text strike.
    pub fn frame(
        &mut self,
        viewport: Viewport,
        inputs: PaintInputs<'_>,
    ) -> Result<PaintList, TreeError> {
        self.prepare_layout(viewport, inputs)?;
        if !self.paint_dirty
            && let Some(paint) = &self.paint
        {
            return Ok(paint.clone());
        }
        for entry in &mut self.nodes {
            if entry.visible && entry.paint_dirty {
                Self::paint_entry(entry, viewport.scale, inputs)?;
                self.stats.rows += entry.rows.len() as u64;
            }
        }
        self.paint_dirty = false;
        let paint = PaintList {
            extent: viewport.extent,
            stamp: inputs.stamp,
            rows: self
                .nodes
                .iter()
                .filter(|entry| entry.visible)
                .flat_map(|entry| entry.rows.iter().copied())
                .collect(),
        };
        self.paint = Some(paint.clone());
        self.stats.lists += 1;
        Ok(paint)
    }

    fn paint_entry(
        entry: &mut super::Entry,
        scale: u32,
        inputs: PaintInputs<'_>,
    ) -> Result<(), TreeError> {
        let origin = edges(entry.rect, scale).0;
        let size = pixel_size(entry.rect, scale);
        let rows = &mut entry.rows;
        rows.clear();
        entry.elided = false;
        entry.overflow = entry.sizes_overflow;
        match &entry.node.kind {
            NodeKind::Fill(tint) => draw(rows, inputs.white, origin, size, *tint),
            NodeKind::Image(image) => draw(rows, *image, origin, size, [1.0; 4]),
            NodeKind::Panel(images) => entry.overflow |= !panel(rows, *images, origin, size, scale),
            NodeKind::Bar {
                fraction,
                track,
                fill,
            } => {
                draw(rows, inputs.white, origin, size, *track);
                draw(
                    rows,
                    inputs.white,
                    origin,
                    [filled(size[0], *fraction), size[1]],
                    *fill,
                );
            }
            NodeKind::Text(text) => {
                let run = shape(entry.node.id, text, scale, size, inputs)?;
                let font = inputs.fonts[text.font];
                entry.tabular = text.options.tabular_digits
                    && TextRun::tabular_supported(
                        font.product,
                        TextOptions {
                            scale,
                            ..text.options
                        },
                    );
                entry.elided = run.elided();
                rows.extend(run.rows(font.sheet, origin, text.tint, text.outline));
            }
            NodeKind::Overlay
            | NodeKind::Stack { .. }
            | NodeKind::Scope(_)
            | NodeKind::Button(_)
            | NodeKind::DropTarget(_) => {}
        }
        entry.paint_dirty = false;
        Ok(())
    }
}

pub(super) fn shape(
    id: NodeId,
    text: &Text,
    scale: u32,
    extent: [u32; 2],
    inputs: PaintInputs<'_>,
) -> Result<TextRun, TreeError> {
    let font = inputs.fonts.get(text.font).ok_or(TreeError::Font(id))?;
    TextRun::for_tree(
        font.product,
        &text.value,
        TextOptions {
            scale,
            extent,
            ..text.options
        },
    )
    .map_err(|error| TreeError::Text(id, error))
}

fn edges(rect: Box2, scale: u32) -> ([f64; 2], [f64; 2]) {
    // Rational point lengths derived from a whole-pixel font can land a few
    // ulps below a half pixel. Use one tie rule for both edges of that box.
    let snap = |point: f64| point.mul_add(f64::from(scale), 1e-8).round();
    (rect.origin.map(snap), rect.end().map(snap))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "snapped edges are nonnegative and within the output extent"
)]
pub(super) fn pixel_size(rect: Box2, scale: u32) -> [u32; 2] {
    let (start, end) = edges(rect, scale);
    std::array::from_fn(|axis| (end[axis] - start[axis]).max(0.0) as u32)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the rounded fill is bounded by the track"
)]
fn filled(width: u32, fraction: f64) -> u32 {
    (f64::from(width) * nonnegative(fraction).min(1.0)).round() as u32
}

fn draw(rows: &mut Vec<SpriteRow>, image: Image, origin: [f64; 2], size: [u32; 2], tint: [f32; 4]) {
    if size.contains(&0) || image.texels.contains(&0) {
        return;
    }
    let mut row = SpriteRow::new(image.frame, origin);
    row.sheet = image.sheet;
    row.tint = tint;
    row.scale = std::array::from_fn(|axis| size[axis] as f32 / image.texels[axis] as f32);
    rows.push(row);
}

fn panel(
    rows: &mut Vec<SpriteRow>,
    images: [Image; 9],
    origin: [f64; 2],
    size: [u32; 2],
    scale: u32,
) -> bool {
    let left_top = images[0].texels.map(|texels| texels.saturating_mul(scale));
    let right_bottom =
        [images[2].texels[0], images[6].texels[1]].map(|texels| texels.saturating_mul(scale));
    if (0..2).any(|axis| left_top[axis].saturating_add(right_bottom[axis]) > size[axis]) {
        return false;
    }
    let widths = [
        left_top[0],
        size[0] - left_top[0] - right_bottom[0],
        right_bottom[0],
    ];
    let heights = [
        left_top[1],
        size[1] - left_top[1] - right_bottom[1],
        right_bottom[1],
    ];
    let mut y = origin[1];
    for (row, height) in heights.into_iter().enumerate() {
        let mut x = origin[0];
        for (column, width) in widths.into_iter().enumerate() {
            draw(
                rows,
                images[row * 3 + column],
                [x, y],
                [width, height],
                [1.0; 4],
            );
            x += f64::from(width);
        }
        y += f64::from(height);
    }
    true
}
