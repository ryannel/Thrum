use super::{ButtonState, Control, Direction, Intent, Owner, Routed, Router, eligible};
use crate::NodeId;

impl<K: Copy + Ord> Router<K> {
    /// State of a displayed button, for the game's chosen styles.
    pub fn button_state(&self, id: NodeId) -> ButtonState {
        if self
            .shown
            .controls
            .iter()
            .any(|c| c.id == id && !c.button.enabled)
        {
            return ButtonState::Disabled;
        }
        if self.presses.values().any(|press| {
            matches!(press.owner, Owner::Interface { button: Some(button), .. } if button == id)
        }) {
            return ButtonState::Pressed;
        }
        if self.show_focus && self.focus() == Some(id) {
            ButtonState::Focused
        } else {
            ButtonState::Idle
        }
    }

    pub(super) fn set_focus(&mut self, id: Option<NodeId>) {
        let Some(id) = id else { return };
        if self.top_id().is_some()
            && eligible(&self.shown, self.top_id(), id)
            && let Some(scope) = self.scopes.last_mut()
        {
            scope.focus = Some(id);
        }
    }

    pub(super) fn reconcile_focus(&mut self) {
        if let Some(scope) = self.scopes.last_mut() {
            let shown = &self.shown;
            let id = Some(scope.id);
            scope.focus = scope
                .focus
                .filter(|focus| eligible(shown, id, *focus))
                .or_else(|| {
                    scope
                        .policy
                        .initial
                        .filter(|focus| eligible(shown, id, *focus))
                })
                .or_else(|| {
                    shown
                        .controls
                        .iter()
                        .find(|control| control.scope == id && control.button.enabled)
                        .map(|control| control.id)
                });
        }
    }

    pub(super) fn hit(&self, point: [f64; 2]) -> Option<NodeId> {
        self.shown
            .controls
            .iter()
            .rev()
            .filter(|control| control.scope == self.top_id() && in_hit_box(control, point))
            .min_by(|a, b| distance_squared(a, point).total_cmp(&distance_squared(b, point)))
            .map(|control| control.id)
    }

    pub(super) fn apply_intent(&mut self, intent: Intent) -> Option<Routed<K>> {
        let scope = self.top_id()?;
        match intent {
            Intent::Activate => self.focus().map(Routed::Activated),
            Intent::Cancel => Some(Routed::Cancelled(scope)),
            Intent::Direction(direction) => {
                let focus = self.direction(direction);
                self.set_focus(focus);
                None
            }
            Intent::Next | Intent::Previous => {
                let controls = self
                    .shown
                    .controls
                    .iter()
                    .filter(|control| control.scope == Some(scope) && control.button.enabled);
                let focus = if intent == Intent::Next {
                    adjacent(controls, self.focus())
                } else {
                    adjacent(controls.rev(), self.focus())
                };
                self.set_focus(focus);
                None
            }
        }
    }

    fn direction(&self, direction: Direction) -> Option<NodeId> {
        let current = self
            .shown
            .controls
            .iter()
            .find(|control| Some(control.id) == self.focus())?;
        let index = direction as usize;
        if let Some(id) = current.button.neighbours[index]
            && eligible(&self.shown, current.scope, id)
        {
            return Some(id);
        }
        let axis = index / 2;
        let cross = 1 - axis;
        let forwards = index % 2 == 1;
        let start = current.bounds.origin;
        let end = current.bounds.end();
        let centre = centre(current);
        self.shown
            .controls
            .iter()
            .filter(|candidate| candidate.scope == current.scope && candidate.button.enabled)
            .filter_map(|candidate| {
                let other = self::centre(candidate);
                if (forwards && other[axis] <= centre[axis])
                    || (!forwards && other[axis] >= centre[axis])
                {
                    return None;
                }
                let near = candidate.bounds.origin;
                let far = candidate.bounds.end();
                let gap = if forwards {
                    near[axis] - end[axis]
                } else {
                    start[axis] - far[axis]
                };
                let offset = (other[cross] - centre[cross]).abs();
                let overlaps = far[cross] >= start[cross] && near[cross] <= end[cross];
                let gap = gap.max(0.0);
                Some((
                    !overlaps,
                    if overlaps {
                        gap
                    } else {
                        2.0f64.mul_add(offset, gap)
                    },
                    offset,
                    candidate.id,
                ))
            })
            .min_by(|a, b| {
                a.0.cmp(&b.0)
                    .then(a.1.total_cmp(&b.1))
                    .then(a.2.total_cmp(&b.2))
            })
            .map(|candidate| candidate.3)
    }
}

fn adjacent<'a>(
    mut controls: impl Iterator<Item = &'a Control>,
    focus: Option<NodeId>,
) -> Option<NodeId> {
    let first = controls.next()?.id;
    if Some(first) != focus {
        controls.find(|control| Some(control.id) == focus);
    }
    Some(controls.next().map_or(first, |control| control.id))
}

fn centre(control: &Control) -> [f64; 2] {
    std::array::from_fn(|axis| control.bounds.size[axis].mul_add(0.5, control.bounds.origin[axis]))
}

fn in_hit_box(control: &Control, point: [f64; 2]) -> bool {
    (0..2).all(|axis| {
        let padding = (44.0 - control.bounds.size[axis]).max(0.0) * 0.5;
        let start = control.bounds.origin[axis] - padding;
        let end = control.bounds.origin[axis] + control.bounds.size[axis] + padding;
        (start..=end).contains(&point[axis])
    })
}

fn distance_squared(control: &Control, point: [f64; 2]) -> f64 {
    let end = control.bounds.end();
    let [x, y] = std::array::from_fn(|axis| {
        (control.bounds.origin[axis] - point[axis])
            .max(point[axis] - end[axis])
            .max(0.0)
    });
    x.mul_add(x, y * y)
}
