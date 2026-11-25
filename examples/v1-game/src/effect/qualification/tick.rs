//! Exact browser witness for the diagnostic's independent tick submission.

use super::{
    DeviceCapabilities, DiagnosticEffect, FrameConfig, FrameHost, Scene, SourceTick,
    SurfaceProfile, SyntheticScene, World, advance, error, replacement_device,
};
use crate::effect::tick::{Answer, Mailbox, Post};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use thrum_core::sim::{WorldAdvance, WorldPublication};
use thrum_render::runtime::{FrameError, FrameWorld};

type Host = FrameHost<ScriptWorld, Scene>;

struct ScriptWorld {
    endpoint: World,
    mailbox: Arc<Mutex<Mailbox>>,
    next_point: Option<[u32; 2]>,
}

impl FrameWorld for ScriptWorld {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        self.endpoint.source()
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }

    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        let previous = self.source().tick;
        self.endpoint.0.tick = now.as_secs();
        let endpoint = self.source();
        if let Some(point) = self.next_point.take() {
            let post = Post {
                endpoint,
                use_tick: endpoint.tick + 1,
                point,
            };
            let mut mailbox = self
                .mailbox
                .lock()
                .map_err(|error| FrameError::World(error.to_string()))?;
            if !mailbox.post(post) {
                return Err(FrameError::World("trail post mailbox is full".into()));
            }
        }
        let mut report = self.endpoint.advance(now)?;
        report.completed_ticks = u32::try_from(endpoint.tick.saturating_sub(previous))
            .map_err(|error| FrameError::World(error.to_string()))?;
        Ok(report)
    }

    fn publication(&mut self) -> Result<WorldPublication<SourceTick>, FrameError> {
        self.endpoint.publication()
    }

    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.endpoint.presentation_alpha_at(lead)
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.endpoint.display_phase_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

const PROFILE: SurfaceProfile = SurfaceProfile {
    format: wgpu::TextureFormat::Bgra8Unorm,
    extent: [32, 24],
};
const POINT: [u32; 2] = [3, 16];

pub(crate) async fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    instance: &wgpu::Instance,
) -> Result<String, String> {
    let (effect, _) = DiagnosticEffect::new();
    let mailbox = Arc::clone(&effect.queries);
    let mut host = FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: PROFILE.extent,
            surface: Some(PROFILE),
        },
        ScriptWorld {
            endpoint: World(SourceTick {
                epoch: 7,
                timeline: 3,
                tick: 10,
            }),
            mailbox: Arc::clone(&mailbox),
            next_point: None,
        },
        Scene,
        SyntheticScene::grid(1).map_err(error)?,
        1.0,
    )
    .await
    .map_err(error)?;
    host.set_scene_contribution(Box::new(effect))?;
    draw(&mut host, 10)?;
    let mut field = vec![0; 1024];
    draw(&mut host, 11)?;
    retire_views(&mut host).await?;
    field = advance(&field, 10, 11, true);
    let submitted_post = post_on_advance(&mut host, 12, POINT);
    let before = submitted_views(&host);
    draw(&mut host, 12)?;
    let submitted = drain(&mut host, &mailbox, false).await?;
    verify(submitted, submitted_post, &field)?;
    if submitted_views(&host) != before + 1 {
        return Err("the advance carrying the post did not submit its view".into());
    }

    field = advance(&field, 11, 12, true);
    let dropped_post = post_on_advance(&mut host, 13, POINT);
    let at_post = submitted_views(&host);
    let frame = host.advance(Duration::from_secs(13)).map_err(error)?;
    if !frame.can_acquire_serial() {
        return Err("drop witness did not admit a drawable opportunity".into());
    }
    drop(frame);
    let dropped = drain(&mut host, &mailbox, true).await?;
    verify(dropped, dropped_post, &field)?;
    let at_drain = submitted_views(&host);
    if at_drain != at_post {
        return Err("the dropped view was needed to drain the answer".into());
    }

    let lost = Arc::new(AtomicBool::new(false));
    let notification = Arc::clone(&lost);
    device.set_device_lost_callback(move |_, _| notification.store(true, Ordering::Release));
    let inputs = host.device_prepare_inputs().map_err(error)?;
    device.destroy();
    for _ in 0..100 {
        if lost.load(Ordering::Acquire) {
            break;
        }
        browser_turn().await?;
    }
    if !lost.load(Ordering::Acquire) {
        return Err("Chrome did not report loss of the original device".into());
    }
    let (_, queue, replacement) = replacement_device(
        instance,
        caps.generation() + 1,
        caps.recipe().map_err(error)?,
    )
    .await?;
    let mut candidate = inputs
        .prepare(&replacement, &queue, Some(PROFILE))
        .await
        .map_err(error)?;
    host.publish_device(&mut candidate).map_err(error)?;
    draw(&mut host, 14)?;
    draw(&mut host, 15)?;
    retire_views(&mut host).await?;
    field = advance(&[0; 1024], 14, 15, true);
    let replacement_post = post_on_advance(&mut host, 16, [5, POINT[1]]);
    draw(&mut host, 16)?;
    let recovered = drain(&mut host, &mailbox, false).await?;
    // Use the moving source's point on the replacement generation.
    verify(recovered, replacement_post, &field)?;

    Ok(format!(
        "adapter={:?}; generation {} -> {}; submitted={submitted:?}; dropped={dropped:?}; views at dropped post={at_post} and drain={at_drain}; recovered={recovered:?}",
        caps.adapter(),
        caps.generation(),
        replacement.generation(),
    ))
}

fn draw(host: &mut Host, tick: u64) -> Result<(), String> {
    let at_tick = |error: FrameError| format!("draw at tick {tick}: {error:?}");
    let report = host
        .advance(Duration::from_secs(tick))
        .map_err(at_tick)?
        .encode_scene()
        .map_err(at_tick)?
        .submit(None)
        .map_err(at_tick)?;
    if !report.submitted {
        return Err(format!("no view submitted at tick {tick}"));
    }
    Ok(())
}

fn post_on_advance(host: &mut Host, tick: u64, point: [u32; 2]) -> Post {
    host.world_mut().next_point = Some(point);
    Post {
        endpoint: SourceTick {
            tick,
            ..host.world().source()
        },
        use_tick: tick + 1,
        point,
    }
}

fn verify(answer: Answer, post: Post, oracle: &[u32]) -> Result<(), String> {
    let [x, y] = post.point;
    let value = oracle[(32 * y + x) as usize];
    if value == 0
        || answer
            != (Answer {
                post,
                value: Some(value),
            })
    {
        return Err(format!(
            "answer {answer:?} differed from post {post:?}, oracle {value}"
        ));
    }
    Ok(())
}

fn submitted_views(host: &Host) -> u64 {
    host.last_submitted_clock()
        .map_or(0, |view| view.submission_order + 1)
}

async fn drain(
    host: &mut Host,
    mailbox: &Mutex<Mailbox>,
    keep_dropping: bool,
) -> Result<Answer, String> {
    for _ in 0..100 {
        browser_turn().await?;
        drop(host.poll_completed().map_err(error)?);
        if keep_dropping {
            let tick = host.world().source().tick;
            drop(host.advance(Duration::from_secs(tick)).map_err(error)?);
        }
        let answers = mailbox.lock().map_err(error)?.drain(host.world().source());
        // Also let the view slots retire before the next scripted step.
        match answers.as_slice() {
            [] => {}
            [answer] => {
                retire_views(host).await?;
                return Ok(*answer);
            }
            _ => return Err("one post yielded more than one trail answer".into()),
        }
    }
    Err("trail answer did not arrive at a drain".into())
}

async fn retire_views(host: &mut Host) -> Result<(), String> {
    for _ in 0..100 {
        drop(host.poll_completed().map_err(error)?);
        if host.pending_frames() == 0 {
            return Ok(());
        }
        browser_turn().await?;
    }
    Err("diagnostic view did not retire".into())
}

async fn browser_turn() -> Result<(), String> {
    let window = web_sys::window().ok_or("no window for WebGPU qualification")?;
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(fault) =
            window.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 4)
        {
            drop(reject.call1(&wasm_bindgen::JsValue::UNDEFINED, &fault));
        }
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(error)?;
    Ok(())
}
