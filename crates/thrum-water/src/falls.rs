use std::sync::Arc;

use thrum_render::contribution::{QualityInvalidation, QualityKnob, QualitySettings};
use thrum_water_model::{
    FallSamples, FallSegment, Map, PathPoint, Plunge, PlungeProfile, SheetProfile,
};

mod plunge;
mod sheet;
pub(crate) use plunge::{Plunges, Source};
pub(super) use sheet::{Sheet, declaration};

pub(super) const SPACING: &str = "river.fall_path_spacing";

/// Physical settings used by the falling water and the plunge it drives.
#[derive(Clone, Copy, Debug, Default)]
pub struct FallsProfile {
    /// Breakup and tear thresholds for the sheet.
    pub sheet: SheetProfile,
    /// Submerged air and wave shares. The scene supplies the runtime tick length.
    pub plunge: PlungeProfile,
}

impl FallsProfile {
    pub(super) fn valid(self) -> bool {
        [
            self.sheet.breakup,
            self.sheet.breakup_exponent,
            self.sheet.tear,
            self.plunge.foam_cell,
        ]
        .into_iter()
        .all(|v| v.is_finite() && v > 0.0)
            && [self.plunge.air_share, self.plunge.wave_share]
                .into_iter()
                .all(|v| (0.0..=1.0).contains(&v))
    }
}

pub(super) struct Falls {
    pub records: Arc<[(usize, FallSamples)]>,
    pub profile: FallsProfile,
    pub quality: QualitySettings,
}

impl Falls {
    pub(super) fn new(records: Vec<(usize, FallSamples)>, profile: FallsProfile) -> Self {
        Self {
            records: records.into(),
            profile,
            quality: QualitySettings::new(vec![QualityKnob {
                name: SPACING.into(),
                effect: "Spacing along the waterfall sheet, millimetres",
                value: 200,
                range: 100..=1200,
                invalidation: QualityInvalidation::None,
                governor_steps: None,
            }])
            .expect("fall path spacing"),
        }
    }
}

pub(super) struct Segment {
    pub data: FallSegment,
    pub plunge: Option<Plunge>,
    pub bottom: PathPoint,
    pub tongue: Option<PathPoint>,
}

impl Segment {
    pub(super) fn new(
        data: FallSegment,
        above: &str,
        map: &Map,
        profile: SheetProfile,
    ) -> Option<Self> {
        let plunge = data.plunge(&profile, |place| {
            let water = map.sample(place)?;
            let body = map.body(water.body)?;
            (water.depth > 0.0 && body.name != above).then_some(water.row.level)
        });
        let bottom = plunge.map_or_else(
            || *data.path.last().expect("stored fall path"),
            |impact| {
                let k = data
                    .path
                    .partition_point(|point| point.distance < impact.distance)
                    .clamp(1, data.path.len() - 1);
                let [a, b] = [data.path[k - 1], data.path[k]];
                let fraction = (impact.distance - a.distance) / (b.distance - a.distance).max(1e-9);
                PathPoint {
                    place: impact.place,
                    height: (b.height - a.height).mul_add(fraction, a.height),
                    time: impact.time,
                    distance: impact.distance,
                }
            },
        );
        let lip = data.path[0];
        let path_direction = data.path.iter().skip(1).find_map(|point| {
            (glam::DVec2::from_array(point.place) - glam::DVec2::from_array(lip.place))
                .try_normalize()
        });
        let tongue = path_direction.and_then(|direction| {
            let cell = map.bed().lattice().1;
            [0.20, 0.45, 0.70, 1.0, 1.35]
                .into_iter()
                .find_map(|offset| {
                    let distance = cell * offset;
                    let place =
                        (glam::DVec2::from_array(lip.place) - direction * distance).to_array();
                    let water = map.sample(place)?;
                    (water.depth > 0.0 && map.body(water.body)?.name == above).then_some(
                        PathPoint {
                            place,
                            height: water.row.level - 0.005,
                            time: -distance / data.launch_speed.max(0.02),
                            distance: -distance,
                        },
                    )
                })
        });
        (data.discharge > 0.0 && bottom.distance > 0.0).then_some(Self {
            data,
            plunge,
            bottom,
            tongue,
        })
    }
}

pub(super) fn active<'a>(
    records: &'a [(usize, FallSamples)],
    map: &'a Map,
    profile: SheetProfile,
) -> impl Iterator<Item = Vec<Segment>> + 'a {
    records
        .iter()
        .flat_map(move |(id, samples)| samples.at(map.gauge(*id)))
        .map(move |fall| {
            fall.segments
                .into_iter()
                .filter_map(|data| Segment::new(data, &fall.above, map, profile))
                .collect()
        })
}
