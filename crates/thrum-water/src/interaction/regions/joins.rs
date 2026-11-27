use glam::DVec2;
use thrum_water_model::{BodyKind, Map};

#[derive(Clone, Copy)]
pub(super) struct Join {
    bodies: [usize; 2],
    a: DVec2,
    ab: DVec2,
    reach: f64,
}

impl Join {
    pub(super) fn prepared(map: &Map) -> Vec<Self> {
        (0..)
            .map_while(|id| Some((id, map.body(id)?)))
            .filter_map(|(id, body)| match &body.kind {
                BodyKind::Chart(chart) => Some((id, chart)),
                _ => None,
            })
            .flat_map(|(id, chart)| {
                chart
                    .ends
                    .iter()
                    .filter(|end| !end.is_brink())
                    .map(move |end| {
                        let [a, b] = end.segment.map(DVec2::from_array);
                        Self {
                            bodies: [id, end.body],
                            a,
                            ab: b - a,
                            reach: end.length,
                        }
                    })
            })
            .collect()
    }

    pub(super) fn connects(self, a: usize, b: usize, point: DVec2) -> bool {
        if self.bodies != [a, b] && self.bodies != [b, a] {
            return false;
        }
        let t = ((point - self.a).dot(self.ab) / self.ab.length_squared()).clamp(0.0, 1.0);
        point.distance_squared(self.a + self.ab * t) <= self.reach * self.reach
    }
}
