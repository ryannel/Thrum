use super::{Falls, active};
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use thrum_water_model::{FallSegment, Impulse, Map, PatternLoop, Plunge, PlungeProfile};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Source {
    pub place_axis: [f32; 4],
    pub footprint_rate: [f32; 4],
}

struct Impact {
    segment: FallSegment,
    plunge: Plunge,
    pattern: Option<PatternLoop>,
}

#[derive(Default)]
pub(crate) struct Plunges {
    key: Option<([u64; 2], std::time::Duration)>,
    impacts: Vec<Impact>,
    pub births: Vec<Source>,
    profile: PlungeProfile,
}

impl Plunges {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "world coordinates and birth rates are sent to the foam shader"
    )]
    pub(crate) fn record(
        &mut self,
        falls: &Falls,
        map: &Map,
        tick: std::time::Duration,
        waves: bool,
    ) -> bool {
        let key = (map.query_revision(), tick);
        if self.key == Some(key) {
            return false;
        }
        let same_tick_length = self.key.is_some_and(|(_, previous)| previous == tick);
        self.key = Some(key);
        self.profile = PlungeProfile {
            tick: tick.as_secs_f64(),
            ..falls.profile.plunge
        };
        let previous = std::mem::take(&mut self.impacts);
        self.births.clear();
        for source in active(&falls.records, map, falls.profile.sheet).flatten() {
            let Some(plunge) = source.plunge else {
                continue;
            };
            if plunge.drop <= 0.0 {
                continue;
            }
            let segment = source.data;
            let birth = segment.births(&self.profile, &plunge);
            let [a, b] = segment.lip.map(glam::DVec2::from_array);
            let axis = (b - a).normalize_or_zero().as_vec2();
            let gain = 0.0011 / (birth.width * birth.length).max(1e-9);
            self.births.push(Source {
                place_axis: [birth.place[0] as f32, birth.place[1] as f32, axis.x, axis.y],
                footprint_rate: [
                    birth.width as f32,
                    birth.length as f32,
                    (birth.air * gain).min(35.0) as f32,
                    (birth.foam * gain).min(35.0) as f32,
                ],
            });
            let pattern = if waves && self.profile.wave_share > 0.0 {
                previous
                    .iter()
                    .find(|old| {
                        same_tick_length
                            && old.segment.id == segment.id
                            && old.segment.launch_speed.to_bits() == segment.launch_speed.to_bits()
                            && old.segment.lip == segment.lip
                    })
                    .and_then(|old| old.pattern)
                    .or_else(|| {
                        Some(segment.pattern_loop(&self.profile, &|along, down| {
                            rank(along, down, segment.launch_speed)
                        }))
                    })
            } else {
                None
            };
            self.impacts.push(Impact {
                segment,
                plunge,
                pattern,
            });
        }
        true
    }

    pub(crate) fn impulses(&self, tick: u64) -> impl Iterator<Item = Impulse> + '_ {
        self.impacts.iter().filter_map(move |impact| {
            let loop_stats = impact.pattern.as_ref()?;
            Some(impact.segment.impulse(
                &self.profile,
                &|along, down| rank(along, down, impact.segment.launch_speed),
                loop_stats,
                &impact.plunge,
                tick,
            ))
        })
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "matches the sheet shader's f32 foam pattern"
)]
fn rank(along: f64, down: f64, launch: f64) -> f64 {
    let at = Vec2::new(along as f32, down as f32) * Vec2::new(2.4, 0.68);
    let span = (thrum_sea_model::LOOP_SECONDS * launch) as f32 * 0.68;
    let seconds = down as f32 / (launch as f32).max(0.001);
    let seconds_span = thrum_sea_model::LOOP_SECONDS as f32;
    let grouped = noise(Vec2::new(along as f32 * 0.015, seconds), seconds_span, 2.7);
    f64::from(0.43 * noise(at, span, 2.4) + 0.17 * noise(at, span, 8.7) + 0.4 * grouped)
}

fn noise(point: Vec2, span: f32, frequency: f32) -> f32 {
    let period = (span * frequency + 0.5).floor().max(1.0);
    let at = Vec2::new(point.x * frequency, point.y * period / span.max(0.001));
    let tile = at.floor();
    let f = at - tile;
    let w = f * f * (3.0 - 2.0 * f);
    let mix = |a: f32, b: f32, t: f32| (b - a).mul_add(t, a);
    let hash = |at: Vec2| {
        let p = Vec3::new(at.x, at.y, at.x) * Vec3::new(0.1031, 0.103, 0.0973);
        let p = p - p.floor();
        let p = p + p.dot(Vec3::new(p.y, p.z, p.x) + 33.33);
        let rank = (p.x + p.y) * p.z;
        rank - rank.floor()
    };
    let a = tile.y.rem_euclid(period);
    let b = (a + 1.0).rem_euclid(period);
    mix(
        mix(
            hash(Vec2::new(tile.x, a)),
            hash(Vec2::new(tile.x + 1.0, a)),
            w.x,
        ),
        mix(
            hash(Vec2::new(tile.x, b)),
            hash(Vec2::new(tile.x + 1.0, b)),
            w.x,
        ),
        w.y,
    )
}
