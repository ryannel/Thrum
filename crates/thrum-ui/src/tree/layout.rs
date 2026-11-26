use super::{
    Anchor, Box2, NodeKind, PaintInputs, SizeRule, Tree, TreeError, Viewport, nonnegative,
};

impl Tree {
    pub(super) fn layout(
        &mut self,
        viewport: Viewport,
        inputs: PaintInputs<'_>,
    ) -> Result<(), TreeError> {
        self.nodes[0].visible = self.nodes[0].node.shown;
        for index in 0..self.nodes.len() {
            let visible = self.nodes[index].visible;
            for child in 0..self.nodes[index].children.len() {
                let child = self.nodes[index].children[child];
                self.nodes[child].visible = visible && self.nodes[child].node.shown;
            }
        }
        for entry in &mut self.nodes {
            entry.overflow = false;
            entry.sizes_overflow = false;
            entry.paint_dirty = true;
        }
        let safe = viewport.safe();
        let root = if self.nodes[0].node.full_bleed {
            viewport.full()
        } else {
            safe
        };
        self.nodes[0].rect = root;
        for axis in 0..2 {
            for index in (0..self.nodes.len()).rev() {
                self.measure(index, axis, viewport, inputs)?;
            }
            for index in 0..self.nodes.len() {
                if self.nodes[index].visible {
                    self.place_children(index, axis, viewport);
                }
            }
        }
        self.stats.layouts += 1;
        Ok(())
    }

    fn measure(
        &mut self,
        index: usize,
        axis: usize,
        viewport: Viewport,
        inputs: PaintInputs<'_>,
    ) -> Result<(), TreeError> {
        let entry = &self.nodes[index];
        if !entry.visible {
            return Ok(());
        }
        let rule = entry.node.size[axis];
        let points = match rule.rule {
            SizeRule::Fixed(value) => value,
            SizeRule::Fraction(_) | SizeRule::Share(_) => rule.least,
            SizeRule::Fit => match &entry.node.kind {
                NodeKind::Image(image) => f64::from(image.texels[axis]),
                NodeKind::Panel(images) => {
                    panel_min(*images, axis).max(self.contents(index, axis, None))
                }
                NodeKind::Text(text) => {
                    let mut extent = [u32::MAX; 2];
                    if axis == 1 {
                        extent[0] = super::paint::pixel_size(entry.rect, viewport.scale)[0];
                    }
                    let run =
                        super::paint::shape(entry.node.id, text, viewport.scale, extent, inputs)?;
                    f64::from(run.size()[axis]) / f64::from(viewport.scale)
                }
                NodeKind::Stack {
                    axis: flow, gap, ..
                } if flow.index() == axis => self.contents(index, axis, Some(*gap)),
                _ => self.contents(index, axis, None),
            },
        };
        self.nodes[index].natural[axis] = rule.constrained(points);
        Ok(())
    }

    fn contents(&self, index: usize, axis: usize, gap: Option<f64>) -> f64 {
        let entry = &self.nodes[index];
        let mut total: f64 = 0.0;
        let mut count = 0_u32;
        for &child in &entry.children {
            let child = &self.nodes[child];
            if !child.visible || child.node.full_bleed {
                continue;
            }
            let length = child.natural[axis];
            if gap.is_some() {
                total += length;
                count += 1;
            } else {
                let offset = match child.node.anchor[axis] {
                    Anchor::Point(point) => point.max(0.0),
                    _ => 0.0,
                };
                total = total.max(length + offset);
            }
        }
        total
            + gap.unwrap_or_default() * f64::from(count.saturating_sub(1))
            + entry.node.padding[axis]
            + entry.node.padding[axis + 2]
    }

    fn place_children(&mut self, parent: usize, axis: usize, viewport: Viewport) {
        let entry = &self.nodes[parent];
        let frame = entry.rect.inset(entry.node.padding);
        let [start, length] = child_span(frame, viewport.safe(), axis);
        let (stack, gap, align) = match entry.node.kind {
            NodeKind::Stack {
                axis: flow,
                gap,
                align,
            } => (flow.index() == axis, gap, Some(align)),
            _ => (false, 0.0, None),
        };
        let (weight_unit, weight_normalizer, mut deficit) = if stack {
            self.share_unit(parent, axis, length, gap)
        } else {
            (length, 1.0, 0.0)
        };
        let mut cursor = start;
        let end = start + length;
        for slot in 0..self.nodes[parent].children.len() {
            let index = self.nodes[parent].children[slot];
            let child = &mut self.nodes[index];
            if !child.visible {
                continue;
            }
            if child.node.full_bleed {
                child.sizes_overflow |= start > 0.0 || length < viewport.full().size[axis];
                child.overflow |= child.sizes_overflow;
                child.rect.origin[axis] = 0.0;
                child.rect.size[axis] = viewport.full().size[axis];
                continue;
            }
            let size = child.node.size[axis];
            let requested = match size.rule {
                SizeRule::Fixed(_) | SizeRule::Fit => child.natural[axis],
                SizeRule::Fraction(fraction) => size.constrained(length * fraction),
                SizeRule::Share(weight) => size.constrained(if stack {
                    weight_unit * (weight / weight_normalizer)
                } else {
                    length
                }),
            };
            let requested =
                if stack && deficit > 0.0 && matches!(child.node.kind, NodeKind::Text(_)) {
                    {
                        let reduced = (requested - deficit).max(size.least);
                        deficit -= requested - reduced;
                        reduced
                    }
                } else {
                    requested
                };
            let anchor = align.unwrap_or(child.node.anchor[axis]);
            let origin = if stack {
                cursor
            } else {
                start + anchor.offset(length, requested)
            };
            let position = origin.clamp(start, end);
            let available = (end - position).max(0.0);
            let actual = requested.min(available);
            // Text is fitted at paint time. Its elision does not imply geometric overflow.
            let clipped =
                requested > available + 1e-8 && !matches!(child.node.kind, NodeKind::Text(_));
            child.sizes_overflow |= clipped || origin < start - 1e-8 || origin > end + 1e-8;
            child.overflow |= child.sizes_overflow;
            child.rect.origin[axis] = position;
            child.rect.size[axis] = actual;
            if stack {
                cursor = (position + actual + gap).min(end);
            }
        }
    }

    fn share_unit(&self, parent: usize, axis: usize, length: f64, gap: f64) -> (f64, f64, f64) {
        let children = || {
            self.nodes[parent]
                .children
                .iter()
                .map(|&index| &self.nodes[index])
                .filter(|e| e.visible && !e.node.full_bleed)
        };
        let mut fixed = -gap;
        let mut least = 0.0;
        let mut largest_weight: f64 = 0.0;
        for child in children() {
            let size = child.node.size[axis];
            fixed += gap;
            match size.rule {
                SizeRule::Share(weight) => {
                    least += size.least;
                    largest_weight = largest_weight.max(weight);
                }
                SizeRule::Fraction(fraction) => fixed += size.constrained(length * fraction),
                _ => fixed += child.natural[axis],
            }
        }
        let remaining = length - fixed;
        if remaining <= least || largest_weight == 0.0 {
            return (0.0, 1.0, (least - remaining).max(0.0));
        }
        let shares = || {
            children().filter_map(|child| {
                let size = child.node.size[axis];
                match size.rule {
                    SizeRule::Share(weight) => Some((size, weight / largest_weight)),
                    _ => None,
                }
            })
        };
        let unit = remaining / shares().map(|(_, weight)| weight).sum::<f64>();
        if shares().all(|(size, weight)| {
            let share = unit * weight;
            share >= size.least && share <= size.most
        }) {
            return (unit, largest_weight, 0.0);
        }
        // Solve sum(clamp(weight * unit, least, most)) == remaining. A capped
        // share leaves space to the other shares without making order matter.
        let mut low = 0.0;
        let mut high = remaining;
        for _ in 0..48 {
            let unit = f64::midpoint(low, high);
            let used: f64 = shares()
                .map(|(size, weight)| size.constrained(unit * weight))
                .sum();
            if used > remaining {
                high = unit;
            } else {
                low = unit;
            }
        }
        (low, largest_weight, 0.0)
    }
}

fn child_span(parent: Box2, safe: Box2, axis: usize) -> [f64; 2] {
    let start = parent.origin[axis].max(safe.origin[axis]);
    let end = parent.end()[axis].min(safe.end()[axis]);
    [start.min(safe.end()[axis]), nonnegative(end - start)]
}

fn panel_min(images: [super::Image; 9], axis: usize) -> f64 {
    f64::from(images[0].texels[axis])
        + f64::from(images[if axis == 0 { 2 } else { 6 }].texels[axis])
}
