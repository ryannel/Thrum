use crate::{Circle, valid};
use thrum_core::profile::{Number as N, Vector};

/// Read-only query geometry. Touching counts as intersection.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    /// A circle, including a point when radius is zero.
    Circle {
        /// Centre.
        centre: Vector,
        /// Nonnegative radius.
        radius: N,
    },
    /// Axis-aligned closed bounds, including a line or point.
    Box {
        /// Lower corner.
        minimum: Vector,
        /// Upper corner.
        maximum: Vector,
    },
    /// A circle swept along a closed segment.
    Capsule {
        /// Segment start.
        start: Vector,
        /// Segment end.
        end: Vector,
        /// Nonnegative swept radius.
        radius: N,
    },
}

fn point_valid(point: Vector) -> bool {
    valid(point.x) && valid(point.y)
}
fn squared(a: Vector, b: Vector) -> N {
    let x = a.x - b.x;
    let y = a.y - b.y;
    x * x + y * y
}

impl Shape {
    pub(super) fn valid(self) -> bool {
        match self {
            Self::Circle { centre, radius } => {
                point_valid(centre) && valid(radius) && radius >= N::ZERO
            }
            Self::Box { minimum, maximum } => {
                point_valid(minimum)
                    && point_valid(maximum)
                    && minimum.x <= maximum.x
                    && minimum.y <= maximum.y
            }
            Self::Capsule { start, end, radius } => {
                point_valid(start) && point_valid(end) && valid(radius) && radius >= N::ZERO
            }
        }
    }

    pub(super) fn bounds(self) -> (Vector, Vector) {
        match self {
            Self::Circle { centre, radius } => (
                Vector {
                    x: centre.x - radius,
                    y: centre.y - radius,
                },
                Vector {
                    x: centre.x + radius,
                    y: centre.y + radius,
                },
            ),
            Self::Box { minimum, maximum } => (minimum, maximum),
            Self::Capsule { start, end, radius } => (
                Vector {
                    x: start.x.min(end.x) - radius,
                    y: start.y.min(end.y) - radius,
                },
                Vector {
                    x: start.x.max(end.x) + radius,
                    y: start.y.max(end.y) + radius,
                },
            ),
        }
    }

    /// Exact profile predicate, shared with brute-force search. Callers must
    /// supply validated domain values (the index validates before calling).
    pub fn intersects(self, circle: &Circle) -> bool {
        match self {
            Self::Circle { centre, radius } => {
                let sum = radius + circle.radius;
                squared(centre, circle.centre) <= sum * sum
            }
            Self::Box { minimum, maximum } => {
                let closest = Vector {
                    x: circle.centre.x.clamp(minimum.x, maximum.x),
                    y: circle.centre.y.clamp(minimum.y, maximum.y),
                };
                squared(closest, circle.centre) <= circle.radius * circle.radius
            }
            Self::Capsule { start, end, radius } => {
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let length = dx * dx + dy * dy;
                let t = if length == N::ZERO {
                    N::ZERO
                } else {
                    (((circle.centre.x - start.x) * dx + (circle.centre.y - start.y) * dy) / length)
                        .clamp(N::ZERO, N::ONE)
                };
                let closest = Vector {
                    x: start.x + dx * t,
                    y: start.y + dy * t,
                };
                let sum = radius + circle.radius;
                squared(closest, circle.centre) <= sum * sum
            }
        }
    }
}
