mod field;

use super::super::{
    DriveReport, FrameConfig, FrameError, FrameHost, FrameSample, FrameWorld, IN_FLIGHT, SlotState,
    SurfaceOutput,
    tests::{StaticScene, StaticWorld, sample},
};
use super::TickSubmissionPermission;
use crate::{
    capabilities::DeviceCapabilities, extract::SourceTick, present::SurfaceProfile,
    synthetic::SyntheticScene,
};
use std::{
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};
use thrum_core::sim::{WorldAdvance, WorldPublication};
use thrum_test_support::wait::until;

const STEP: Duration = Duration::from_millis(16);
const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Rgba8Unorm,
    extent: [32; 2],
};
const START: SourceTick = SourceTick {
    tick: 0,
    epoch: 2,
    timeline: 3,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Post {
    endpoint: SourceTick,
    use_tick: u64,
    point: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Answer {
    post: Post,
    value: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Enter,
    Record,
    Confirm,
    Leave,
    Callback,
}

#[derive(Default)]
struct Mailbox {
    generation: u64,
    busy: [bool; IN_FLIGHT + 1],
    posts: Vec<Post>,
    answers: Vec<Answer>,
    events: Vec<Event>,
    retired_callbacks: usize,
}

struct World {
    core: StaticWorld,
    mail: Arc<Mutex<Mailbox>>,
    due: u32,
    points: Vec<u32>,
}

impl World {
    fn set_endpoint(&mut self, endpoint: SourceTick) {
        self.core.authority = endpoint;
        self.core.publication = endpoint;
    }

    fn post_from_world(&self, point: u32) -> Post {
        let endpoint = self.source();
        let post = Post {
            endpoint,
            use_tick: endpoint.tick + 1,
            point,
        };
        self.mail
            .lock()
            .expect("test field mailbox")
            .posts
            .push(post);
        post
    }
}

impl FrameWorld for World {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        self.core.source()
    }
    fn tick_duration(&self) -> Duration {
        STEP
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }

    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        let due = std::mem::take(&mut self.due);
        let points = std::mem::take(&mut self.points);
        for index in 0..due {
            self.set_endpoint(SourceTick {
                tick: self.source().tick + 1,
                ..self.source()
            });
            if let Some(&point) = points.get(index as usize) {
                self.post_from_world(point);
            }
        }
        let mut advance = self.core.advance(now)?;
        advance.completed_ticks = due;
        Ok(advance)
    }

    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        self.core.publication()
    }
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.core.presentation_alpha_at(lead)
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.core.display_phase_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

type Host = FrameHost<World, StaticScene>;

struct Fixture {
    host: Host,
    mail: Arc<Mutex<Mailbox>>,
    drawable: wgpu::TextureView,
    views: std::collections::VecDeque<FrameSample>,
    test_gpu: thrum_test_support::Gpu,
}

impl Fixture {
    async fn new(name: &str) -> Option<Self> {
        Self::with_reconstruction(name, false).await
    }

    async fn with_reconstruction(name: &str, reconstruction: bool) -> Option<Self> {
        let test_gpu = thrum_test_support::gpu(name, Some(wgpu::Backend::Metal))?;
        assert_eq!(test_gpu.adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = crate::test_device::portable(&test_gpu, None).await;
        let capabilities = DeviceCapabilities::capture(&test_gpu.adapter, &device, 1);
        let drawable = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("test drawable"),
                size: wgpu::Extent3d {
                    width: PROFILE.extent[0],
                    height: PROFILE.extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PROFILE.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mail = Arc::new(Mutex::new(Mailbox::default()));
        let mut scene = SyntheticScene::grid(1).expect("one-cell scene");
        scene.sun_shadows = false;
        scene.view = scene.view.with_reconstruction(reconstruction);
        let mut host = FrameHost::new(
            &capabilities,
            &queue,
            FrameConfig {
                render_extent: PROFILE.extent,
                surface: Some(PROFILE),
            },
            World {
                core: StaticWorld {
                    authority: START,
                    publication: START,
                },
                mail: Arc::clone(&mail),
                due: 0,
                points: Vec::new(),
            },
            StaticScene::with_reconstruction(reconstruction),
            scene,
            1.0,
        )
        .await
        .expect("prepare tick test host");
        host.set_timing_requested(false);
        Some(Self {
            host,
            mail,
            drawable,
            test_gpu,
            views: std::collections::VecDeque::new(),
        })
    }

    fn attach(&mut self, declare_tick: bool) {
        self.host
            .attach_scene_contribution(Box::new(field::Field::new(
                Arc::clone(&self.mail),
                declare_tick,
            )))
            .expect("attach test field");
    }

    fn schedule(&mut self, points: &[u32]) -> Vec<Post> {
        let world = self.host.world_mut();
        world.due = u32::try_from(points.len())
            .expect("bounded test points")
            .max(1);
        world.points = points.to_vec();
        let start = world.source();
        points
            .iter()
            .copied()
            .zip(start.tick + 1..)
            .map(|(point, tick)| Post {
                endpoint: SourceTick { tick, ..start },
                use_tick: tick + 1,
                point,
            })
            .collect()
    }

    fn draw(&mut self) -> DriveReport {
        let output = SurfaceOutput {
            view: &self.drawable,
            extent: PROFILE.extent,
        };
        let report = self
            .host
            .drive(Duration::ZERO, Some(&output))
            .expect("submit test view");
        assert!(report.submitted && report.surface_presented);
        report
    }

    fn expect_answers(&mut self, expected: &[Answer]) {
        self.until(|mail| mail.answers.len() >= expected.len());
        let answers = std::mem::take(&mut self.mail.lock().expect("test field mailbox").answers);
        assert_eq!(answers, expected);
    }

    fn until(&mut self, ready: impl Fn(&Mailbox) -> bool) {
        until("tick readback", Duration::from_secs(3), || {
            if let Some(sample) = self.host.poll_completed().expect("poll test device") {
                self.views.push_back(sample);
            }
            ready(&self.mail.lock().expect("test field mailbox")).then_some(())
        });
    }

    fn finish_view(&mut self) -> FrameSample {
        self.views.pop_front().unwrap_or_else(|| {
            until("view completion", Duration::from_secs(3), || {
                self.host.poll_completed().expect("poll test device")
            })
        })
    }

    fn hold_view_slots(&mut self) {
        for slot in &mut self.host.slots {
            assert!(matches!(slot.state, SlotState::Free));
            // Model completed slots withheld from the host by their owner.
            slot.render_done = Arc::new(AtomicBool::new(false));
            slot.state = SlotState::Rendering(sample(0));
        }
    }

    fn release_view_slots(&mut self) {
        for slot in &mut self.host.slots {
            slot.state = SlotState::Free;
        }
    }
}

struct Oracle {
    values: [u32; 4],
    tick: u64,
}

impl Oracle {
    fn seeded(tick: u64) -> Self {
        Self {
            values: [17, 20, 23, 26],
            tick,
        }
    }

    fn commit(&mut self, tick: u64) {
        for _ in self.tick..tick {
            for (value, stride) in self.values.iter_mut().zip(1..) {
                *value += stride;
            }
        }
        self.tick = tick;
    }

    fn answer(&self, post: Post) -> Answer {
        Answer {
            post,
            value: Some(self.values[post.point as usize]),
        }
    }
}

fn view_count(host: &Host) -> u64 {
    host.next_submission_order
}

#[test]
fn tick_stage_admission_answers_exactly_with_and_without_a_view() {
    pollster::block_on(async {
        let Some(mut f) = Fixture::new("tick_stage_admission").await else {
            return;
        };
        f.attach(false);
        let graph = f.host.scene_graph().unwrap();
        let storage = f.host.storage_report();
        f.attach(true);
        let with_tick = f.host.scene_graph().unwrap();
        assert!(graph.pass_names().eq(with_tick.pass_names()));
        assert_eq!(graph.pass_order, with_tick.pass_order);
        assert_eq!(graph.transient_slots, with_tick.transient_slots);
        assert_eq!(storage, f.host.storage_report());

        f.draw();
        f.finish_view();
        let mut oracle = Oracle::seeded(0);
        let post = f.schedule(&[1])[0];
        f.draw();
        f.expect_answers(&[oracle.answer(post)]);
        f.finish_view();
        oracle.commit(1);

        let views = view_count(&f.host);
        let dropped = f.host.dropped_views();
        f.hold_view_slots();
        let post = f.schedule(&[2])[0];
        let output = SurfaceOutput {
            view: &f.drawable,
            extent: PROFILE.extent,
        };
        assert!(
            f.host
                .drive(Duration::ZERO, Some(&output))
                .unwrap()
                .dropped_view
        );
        f.expect_answers(&[oracle.answer(post)]);
        assert_eq!(
            (view_count(&f.host), f.host.dropped_views()),
            (views, dropped + 1)
        );
        f.release_view_slots();

        let post = f.schedule(&[3])[0];
        assert!(f.host.drive(Duration::ZERO, None).unwrap().dropped_view);
        f.expect_answers(&[oracle.answer(post)]);
        assert_eq!(
            (view_count(&f.host), f.host.dropped_views()),
            (views, dropped + 2)
        );

        let post = f.schedule(&[1])[0];
        let token = f.host.advance(Duration::ZERO).unwrap();
        assert!(token.can_acquire_serial());
        drop(token);
        f.expect_answers(&[oracle.answer(post)]);
        assert_eq!(
            (view_count(&f.host), f.host.dropped_views()),
            (views, dropped + 3)
        );

        f.schedule(&[]);
        let token = f
            .host
            .advance(Duration::ZERO)
            .unwrap()
            .encode_scene()
            .unwrap();
        assert!(token.can_acquire_surface());
        drop(token);
        let post = f.schedule(&[2])[0];
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[oracle.answer(post)]);
        assert_eq!(view_count(&f.host), views);

        let submissions = f.host.tick_submission_stats().submitted;
        let posts = f.schedule(&[0, 3]);
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[oracle.answer(posts[0]), oracle.answer(posts[1])]);
        assert_eq!(f.host.tick_submission_stats().submitted, submissions + 1);
        let submissions = f.host.tick_submission_stats();
        drop(f.host.advance(Duration::ZERO).unwrap());
        assert_eq!(f.host.tick_submission_stats(), submissions);
    });
}

#[derive(Clone, Copy)]
enum Decision {
    Grant,
    RefuseEntry,
    RefuseConfirmation,
}

struct Permission {
    decision: Arc<Mutex<Decision>>,
    mail: Arc<Mutex<Mailbox>>,
}

impl TickSubmissionPermission for Permission {
    fn with_entry(&self, record: &mut dyn FnMut(&dyn Fn() -> bool)) {
        let decision = *self.decision.lock().expect("test permission decision");
        self.mail
            .lock()
            .expect("test permission events")
            .events
            .push(Event::Enter);
        if matches!(decision, Decision::RefuseEntry) {
            return;
        }
        record(&|| {
            self.mail
                .lock()
                .expect("test permission events")
                .events
                .push(Event::Confirm);
            matches!(decision, Decision::Grant)
        });
        self.mail
            .lock()
            .expect("test permission events")
            .events
            .push(Event::Leave);
    }
}

#[test]
fn tick_stage_lifecycle_permission_capture_without_rendering_and_timing() {
    pollster::block_on(async {
        crate::timestamp_permit::hold();
        let Some(mut f) =
            Fixture::with_reconstruction("tick_stage_lifecycle_permission", true).await
        else {
            return;
        };
        f.attach(true);
        f.draw();
        f.finish_view();
        let oracle = Oracle::seeded(0);

        let permission = Arc::new(Mutex::new(Decision::Grant));
        f.host.set_tick_submission_permission(Box::new(Permission {
            decision: Arc::clone(&permission),
            mail: Arc::clone(&f.mail),
        }));
        let post = f.schedule(&[0])[0];
        assert!(f.host.drive(Duration::ZERO, None).unwrap().dropped_view);
        assert_eq!(
            f.mail.lock().unwrap().events,
            [
                Event::Enter,
                Event::Record,
                Event::Confirm,
                Event::Leave,
                Event::Callback
            ]
        );
        f.expect_answers(&[oracle.answer(post)]);

        let post = f.schedule(&[1])[0];
        let previous = f.host.tick_submission_stats();
        for (attempt, (decision, events)) in [
            (Decision::RefuseEntry, &[Event::Enter][..]),
            (
                Decision::RefuseConfirmation,
                &[Event::Enter, Event::Record, Event::Confirm, Event::Leave][..],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            f.mail.lock().unwrap().events.clear();
            *permission.lock().unwrap() = decision;
            drop(f.host.advance(Duration::ZERO).unwrap());
            let stats = f.host.tick_submission_stats();
            assert_eq!(
                stats.refused_permission,
                previous.refused_permission + attempt as u64 + 1
            );
            assert_eq!(stats.refused_bound, previous.refused_bound);
            assert_eq!(stats.submitted, previous.submitted);
            let mail = f.mail.lock().unwrap();
            assert_eq!(mail.events, events);
            assert_eq!(mail.posts, [post]);
        }
        *permission.lock().unwrap() = Decision::Grant;
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[oracle.answer(post)]);

        let post = f.schedule(&[2])[0];
        let stats = f.host.tick_submission_stats();
        f.mail.lock().unwrap().events.clear();
        f.host.advance_without_rendering(Duration::ZERO).unwrap();
        drop(f.host.begin_capture().unwrap());
        assert_eq!(f.host.tick_submission_stats(), stats);
        assert_eq!(f.mail.lock().unwrap().posts, [post]);
        assert_eq!(f.mail.lock().unwrap().events, []);

        f.host.set_timing_requested(true);
        f.draw();
        let timed = f.finish_view();
        let gpu_time = timed
            .gpu_pass_ns
            .as_ref()
            .and_then(|passes| passes.get(field::PASS));
        assert!(
            gpu_time.is_some_and(|ns| ns >= 0.0),
            "named tick duration missing: {timed:?}"
        );
        f.expect_answers(&[oracle.answer(post)]);
        let mut oracle = oracle;
        oracle.commit(3);

        f.host.set_timing_requested(false);
        let post = f.schedule(&[3])[0];
        let gathered = f.host.timing_stats().gathered;
        f.draw();
        assert_eq!(f.host.timing_stats().gathered, gathered);
        assert!(
            f.host
                .slots
                .iter()
                .all(|slot| slot.timing.queries.pending_frame().is_none())
        );
        let untimed = f.finish_view();
        assert!(!untimed.timing_requested && untimed.gpu_pass_ns.is_none());
        f.expect_answers(&[oracle.answer(post)]);
        oracle.commit(4);

        assert!(f.host.enable_governor(crate::governor::DisplayRecord {
            interval: STEP,
            thermal: crate::governor::ThermalState::Nominal,
        }));
        for requested in [false, true] {
            f.host.set_timing_requested(requested);
            let post = f.schedule(&[0])[0];
            f.draw();
            let governed = f.finish_view();
            let passes = governed
                .gpu_pass_ns
                .expect("governor still needs frame timings");
            assert!(
                passes
                    .passes()
                    .iter()
                    .any(|pass| pass.name.as_ref() != field::PASS),
                "missing governed view timings: {passes:?}"
            );
            assert_eq!(passes.get(field::PASS).is_some(), requested);
            f.expect_answers(&[oracle.answer(post)]);
            oracle.commit(post.endpoint.tick);
        }
    });
}

#[test]
fn tick_stage_lifecycle_outstanding_submissions_refuse_at_the_bound_and_retry() {
    pollster::block_on(async {
        let Some(mut f) = Fixture::new("tick_stage_lifecycle_bound").await else {
            return;
        };
        f.attach(true);
        f.draw();
        f.finish_view();
        let oracle = Oracle::seeded(0);
        let bound = Host::MAX_TICK_SUBMISSIONS;
        let mut expected = Vec::new();
        let mut completed_on_device = Vec::new();
        for point in 0..bound {
            let post = f
                .host
                .world()
                .post_from_world(u32::try_from(point).unwrap());
            expected.push(oracle.answer(post));
            f.host.ask_tick_stage().unwrap();
            // Hold delivery of these two real completion notifications so the
            // advance's nonblocking poll cannot empty the bound mid-test.
            let done = f.host.ticks.outstanding.last_mut().unwrap();
            completed_on_device.push(std::mem::take(done));
        }
        assert_eq!(f.host.ticks.outstanding.len(), bound);
        assert_eq!(f.host.tick_submission_stats().submitted, bound as u64);
        let post = f.schedule(&[3])[0];
        let recorded = f.mail.lock().unwrap().events.len();
        drop(f.host.advance(Duration::ZERO).unwrap());
        assert_eq!(f.mail.lock().unwrap().events.len(), recorded);
        assert_eq!(f.mail.lock().unwrap().posts, [post]);
        assert_eq!(f.host.tick_submission_stats().refused_bound, 1);
        assert_eq!(f.host.tick_submission_stats().refused_permission, 0);
        assert_eq!(f.host.tick_submission_stats().submitted, bound as u64);
        f.expect_answers(&expected);
        f.host.ticks.outstanding = completed_on_device;
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[oracle.answer(post)]);
        assert_eq!(f.host.tick_submission_stats().submitted, bound as u64 + 1);
    });
}

#[test]
fn tick_stage_world_change_rejects_old_epoch_and_timeline_state() {
    pollster::block_on(async {
        let Some(mut f) = Fixture::new("tick_stage_world_change").await else {
            return;
        };
        f.attach(true);
        f.draw();
        f.finish_view();
        f.schedule(&[]);
        f.draw();
        f.finish_view();
        let mut oracle = Oracle::seeded(0);
        oracle.commit(1);

        for edit in [
            |old: SourceTick| SourceTick {
                timeline: old.timeline + 1,
                ..old
            },
            |old: SourceTick| SourceTick {
                epoch: old.epoch + 1,
                ..old
            },
        ] {
            let old = f.host.world().source();
            let new = edit(old);
            f.host.world_mut().set_endpoint(new);
            assert_eq!(old.tick, new.tick);
            let point = 2;
            let post = f.host.world().post_from_world(point);
            assert_ne!(
                oracle.answer(post).value,
                Oracle::seeded(new.tick).answer(post).value
            );
            let views = view_count(&f.host);
            drop(f.host.advance(Duration::ZERO).unwrap());
            f.expect_answers(&[Answer { post, value: None }]);
            assert_eq!(view_count(&f.host), views);
            f.draw();
            f.finish_view();
            let post = f.host.world().post_from_world(point);
            drop(f.host.advance(Duration::ZERO).unwrap());
            oracle = Oracle::seeded(new.tick);
            f.expect_answers(&[oracle.answer(post)]);
            f.schedule(&[]);
            let next = f.draw();
            f.finish_view();
            oracle.commit(next.tick);
        }
    });
}

#[test]
fn tick_stage_lifecycle_and_tick_stage_world_change_across_device_replacement() {
    pollster::block_on(async {
        let Some(mut f) = Fixture::new("tick_stage_world_change_replacement").await else {
            return;
        };
        f.attach(true);
        f.draw();
        f.finish_view();
        f.schedule(&[]);
        f.draw();
        f.finish_view();
        let old_device = f.host.device.clone();
        // The shell retains the old queue through handoff, so replacing it
        // cannot run its completion callbacks from the queue destructor.
        let old_queue = f.host.queue.clone();
        let (new_device, queue) = crate::test_device::portable(&f.test_gpu, None).await;
        let caps = DeviceCapabilities::capture(&f.test_gpu.adapter, &new_device, 2);
        let mut candidate = f
            .host
            .device_prepare_inputs()
            .unwrap()
            .prepare(&caps, &queue, Some(PROFILE))
            .await
            .unwrap();
        let old_post = f.host.world().post_from_world(3);
        f.host.ask_tick_stage().unwrap();
        assert_eq!(f.mail.lock().unwrap().answers, []);
        assert!(f.mail.lock().unwrap().busy.iter().any(|busy| *busy));
        f.host.publish_device(&mut candidate).unwrap();
        until("retired map callback", Duration::from_secs(3), || {
            old_device.poll(wgpu::PollType::Poll).unwrap();
            (f.mail.lock().unwrap().retired_callbacks > 0).then_some(())
        });
        drop(old_queue);
        let post = f.host.world().post_from_world(1);
        assert_eq!(post.endpoint, old_post.endpoint);
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[Answer { post, value: None }]);
        f.drawable = f
            .host
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("replacement drawable"),
                size: wgpu::Extent3d {
                    width: PROFILE.extent[0],
                    height: PROFILE.extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PROFILE.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let frame = f.draw();
        f.finish_view();
        let post = f.host.world().post_from_world(1);
        drop(f.host.advance(Duration::ZERO).unwrap());
        f.expect_answers(&[Oracle::seeded(frame.tick).answer(post)]);
    });
}
