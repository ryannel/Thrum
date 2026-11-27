use super::{
    Answer, CascadeSample, Component, Interaction, Point, RiverPoint, RiverReceiver, Settings,
};
use thrum_test_support::{record, records};

mod held_answer {
    use crate::query::{
        BodyAnswers, BodyQuery, QueryAnswer, QueryLimits, QueryMailbox, QueryPoint,
    };
    use std::{sync::Arc, time::Duration};
    use thrum_render::extract::SourceTick;
    use thrum_sea_model::{LongWaves, Sample};
    use thrum_water_model::{Bed, Map, Profile, Surface};

    const STEP: Duration = Duration::from_secs(1);

    fn endpoint(tick: u64) -> SourceTick {
        SourceTick {
            tick,
            epoch: 0,
            timeline: 0,
        }
    }

    fn mailbox() -> QueryMailbox {
        let map = Map::prepare(
            Bed::new([0.0; 2], 1.0, [2, 2], vec![0.0; 4]).expect("bed"),
            Vec::new(),
            Profile {
                margin_height: 1.0,
                swash_ratio: 1.0,
            },
        )
        .expect("map");
        QueryMailbox::new(
            QueryLimits {
                points: 1,
                max_age: 8,
            },
            Arc::new(map.into()),
            LongWaves::new(Vec::new(), 0.0).expect("calm waves"),
            [0.0; 4],
        )
    }

    fn publish(mailbox: &QueryMailbox, posted: u64, generation: u32, height: f64) {
        let endpoint = endpoint(posted);
        let use_tick = posted + 1;
        mailbox.post(
            endpoint,
            vec![BodyQuery {
                id: 1,
                generation,
                points: Vec::new(),
            }],
        );
        mailbox.state.lock().expect("mailbox").ready.insert(
            (1, posted),
            super::super::ReadyBody {
                answers: BodyAnswers {
                    id: 1,
                    generation,
                    posted,
                    held: false,
                    endpoint,
                    cut_nodes: [0; 2],
                    points: vec![QueryAnswer {
                        index: 0,
                        point: QueryPoint {
                            position: [0.0; 2],
                            radius: 0.0,
                            use_tick,
                        },
                        surface: Surface {
                            body: Some(0),
                            wet: height > 0.0,
                            sample: Sample {
                                height,
                                height_dt: -0.5,
                                ..Sample::default()
                            },
                            ..Surface::default()
                        },
                        bed: Some(0.0),
                        sources: [None; 5],
                        supported: [false; 5],
                        wake_height: 0.0,
                        wave_heights: [0.0; 4],
                    }],
                },
                predictions: Vec::new(),
                frame_offset: 0.0,
                frame_tick: posted,
                frame_seconds: 0.0,
                step: STEP,
            },
        );
    }

    fn check_reply(mailbox: &QueryMailbox, tick: u64, posted: u64, height: f64, held: bool) {
        let replies = mailbox.drain(endpoint(tick), STEP);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].posted, posted);
        assert_eq!(replies[0].held, held);
        let surface = &replies[0].points[0].surface;
        assert!((surface.sample.height - height).abs() < f64::EPSILON);
        assert_eq!(surface.wet, height > 0.0);
    }

    fn check_empty_drain(mailbox: &QueryMailbox, tick: u64) {
        let stats = mailbox.stats();
        assert!(mailbox.drain(endpoint(tick), STEP).is_empty());
        assert_eq!(mailbox.stats(), stats);
    }

    fn check(mailbox: &QueryMailbox, tick: u64, posted: u64, height: f64, held: bool) {
        check_reply(mailbox, tick, posted, height, held);
        check_empty_drain(mailbox, tick);
    }

    #[test]
    fn held_answer_allows_an_undelivered_result_on_a_repeated_drain() {
        for publish_before_first_drain in [false, true] {
            let mailbox = mailbox();
            publish(&mailbox, 1, 0, 2.0);
            if publish_before_first_drain {
                publish(&mailbox, 2, 0, 3.0);
            }
            check_reply(&mailbox, 3, 1, 1.5, false);
            let first = mailbox.stats();
            assert_eq!([first.age_one, first.age_two, first.held_points], [0, 1, 0]);

            if !publish_before_first_drain {
                publish(&mailbox, 2, 0, 3.0);
            }
            check_reply(&mailbox, 3, 2, 3.0, false);
            let second = mailbox.stats();
            assert_eq!(
                [second.age_one, second.age_two, second.held_points],
                [1, 1, 0]
            );
            check_empty_drain(&mailbox, 3);
        }
    }

    #[test]
    fn held_answer_carries_from_use_tick_once_per_drain_until_expired() {
        let mailbox = mailbox();
        publish(&mailbox, 1, 0, 1.0);
        check(&mailbox, 2, 1, 1.0, false);
        check(&mailbox, 3, 1, 0.5, true);
        check(&mailbox, 4, 1, 0.0, true);
        check(&mailbox, 5, 1, -0.5, true);
        publish(&mailbox, 4, 0, 10.0);
        check(&mailbox, 6, 4, 9.5, false);
        publish(&mailbox, 6, 0, 20.0);
        check(&mailbox, 9, 6, 19.0, false);
        check(&mailbox, 14, 6, 16.5, true);
        let stats = mailbox.stats();
        assert_eq!(
            [
                stats.age_one,
                stats.age_two,
                stats.age_older,
                stats.held_points
            ],
            [1, 1, 1, 4]
        );
        assert!(mailbox.drain(endpoint(15), STEP).is_empty());
        assert_eq!(mailbox.stats(), stats);
    }

    #[test]
    fn held_answer_belongs_to_the_world_body_and_device_that_made_it() {
        let mailbox = mailbox();
        publish(&mailbox, 1, 0, 1.0);
        check(&mailbox, 2, 1, 1.0, false);
        mailbox.post(
            endpoint(2),
            vec![BodyQuery {
                id: 1,
                generation: 1,
                points: Vec::new(),
            }],
        );
        assert!(mailbox.drain(endpoint(3), STEP).is_empty());
        publish(&mailbox, 3, 1, 1.0);
        check(&mailbox, 4, 3, 1.0, false);
        mailbox.reset_readback();
        assert!(mailbox.drain(endpoint(5), STEP).is_empty());
        assert!(mailbox.drain(endpoint(6), STEP).is_empty());
        publish(&mailbox, 6, 1, 2.0);
        check(&mailbox, 7, 6, 2.0, false);
        assert_eq!(mailbox.stats().age_one, 3);
        let restored = SourceTick {
            timeline: 1,
            ..endpoint(8)
        };
        assert!(mailbox.drain(restored, STEP).is_empty());
        assert_eq!(mailbox.stats().held_points, 0);
    }
}

// A wave component is a nested record in the foam and query settings, so the reflection compares its size and
// holds no scalar kind for it. Its own members are checked below.
impl records::Kind for Component {
    const KIND: Option<records::RecordScalar> = None;
}
// So is a cascade sample in the answer.
impl records::Kind for CascadeSample {
    const KIND: Option<records::RecordScalar> = None;
}

impl records::Kind for RiverReceiver {
    const KIND: Option<records::RecordScalar> = None;
}

impl records::Kind for Point {
    const KIND: Option<records::RecordScalar> = None;
}

fn query_point_layout() -> records::RecordLayout {
    record!(
        Point as "QueryPoint",
        position,
        radius,
        dt,
        level_current_bed,
        slope,
        body,
        flags,
        gains,
        dent,
        surf_stage,
        fine_bounds,
        coarse_grid,
        carry,
        track,
        current_gradient,
        track_gradient,
    )
}

#[test]
fn river_query_records_extend_the_plain_layout() {
    let source = Interaction::sample_source(0, 5, 6, true)
        + include_str!("../../gains.wgsl")
        + include_str!("../../column/records.wgsl")
        + include_str!("../../column.wgsl")
        + include_str!("../../carriage.wgsl")
        + &super::sampling_source(true, false)
        + super::PLAIN_QUERY;
    records::named(
        "river query shader",
        &source,
        &[
            query_point_layout(),
            record!(RiverPoint, point, receiver),
            record!(
                RiverReceiver,
                current_weight_gradient,
                track,
                current_gradient,
                track_gradient
            ),
        ],
    );
}

#[test]
fn query_records_match_the_shader_layout() {
    let source = Interaction::sample_source(0, 5, 6, true)
        + include_str!("../../gains.wgsl")
        + include_str!("../../column/records.wgsl")
        + include_str!("../../column.wgsl")
        + include_str!("../../carriage.wgsl")
        + &super::sampling_source(false, false)
        + super::PLAIN_QUERY;
    records::named(
        "water query shader",
        &source,
        &[
            query_point_layout(),
            record!(
                Component,
                vector,
                amplitude,
                phase,
                omega,
                inverse_k,
                shore_sector,
                padding
            ),
            record!(
                Settings,
                origin,
                count,
                components,
                lengths_chop,
                heights,
                swash_margin,
                coarse_jobs,
            ),
            record!(CascadeSample, height, slope, differential, velocity),
            record!(
                Answer,
                height_slope_rate,
                velocity_depth,
                layers,
                current_bed,
                body_flags,
                cascades,
                cut_fine_nodes,
                cut_coarse_nodes,
                advection_rate,
                alignment,
                wake,
            ),
        ],
    );
}

#[test]
fn surf_query_passes_parse_with_all_shared_bindings() {
    let source = Interaction::sample_source(0, 5, 6, true)
        + include_str!("../../column/records.wgsl")
        + include_str!("../../column.wgsl")
        + include_str!("../../carriage.wgsl")
        + &super::sampling_source(true, false)
        + &crate::map_gpu::declarations_at(super::SURF_GROUP)
        + &crate::surf::SurfGpu::query_source(super::SURF_GROUP, super::SURF_BINDING);
    records::named("surf body query passes", &source, &[]);
}
