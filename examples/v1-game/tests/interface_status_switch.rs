//! The live game switches sheets and boxes on one submission and preserves every intervening frame.

mod status_metal;
mod status_store;

use status_metal as metal;
use status_store as store;
use v1_game::interface_status::{self as status, proof};

use std::time::Duration;

use thrum_asset::{FontProduct, Head, SheetProduct};
use thrum_cook::Cooker;
use thrum_render::runtime::{FrameConfig, SubmittedImage};
use thrum_test_support::{Opened, Scratch};
use thrum_ui::{Font, Image, PaintInputs, PaintStamp, Viewport};
use v1_game::{GameHost, GameScene, GameSetup, GameWorld, InterfaceText};

const EXTENT: [u32; 2] = [704, 280];

struct Rig {
    runtime: thrum_render::runtime::FrameHost<GameWorld, GameScene>,
    clock: Duration,
    drawable: wgpu::TextureView,
}

impl Rig {
    fn new(gpu: &Opened, cooker: &Cooker, scale: u8) -> Self {
        let setup = GameSetup {
            sprite_scale: Some(2),
            sprite_rows: Some(4),
            interface_scale: Some(scale),
            interface_text: Some(InterfaceText::Status),
            ..GameSetup::default()
        };
        let config = FrameConfig {
            render_extent: EXTENT,
            surface: Some(thrum_render::present::SurfaceProfile {
                extent: EXTENT,
                format: metal::FORMAT,
            }),
        };
        let mut runtime = pollster::block_on(setup.prepare_with_sheet_source(
            &gpu.device,
            &gpu.queue,
            &gpu.capabilities,
            config,
            cooker.store().root().to_owned().into(),
        ))
        .expect("V1 game with status display");
        runtime.continuous_source_mut().measure_status_updates();
        runtime.enable_submitted_capture(1).expect("capture");
        Self {
            runtime,
            clock: Duration::ZERO,
            drawable: metal::drawable(gpu, EXTENT),
        }
    }

    fn draw(&mut self) -> SubmittedImage {
        self.clock = Duration::from_secs_f64(self.runtime.route_step_ms(0.5) / 1000.0);
        metal::submit(&mut self.runtime, &self.drawable, EXTENT, self.clock)
    }
}

struct Publication {
    font: FontProduct,
    art: SheetProduct,
    head: Head,
    scale: u32,
}

impl Publication {
    fn from(cooker: &Cooker, scale: u32) -> Self {
        let (font, art) = proof::open(cooker.store());
        let head = cooker
            .store()
            .read_head()
            .expect("read HEAD")
            .expect("publication");
        Self {
            font,
            art,
            head,
            scale,
        }
    }

    const fn stamp(&self) -> PaintStamp {
        PaintStamp {
            head: self.head,
            scale: self.scale,
        }
    }

    fn difference(&self, frame: &SubmittedImage) -> Option<[u32; 4]> {
        let view = Viewport {
            extent: EXTENT,
            scale: self.scale,
            insets: [0.0; 4],
        };
        let tick = frame.sample.tick;
        let (reference, region) = proof::compose_with(view, &self.font, &self.art, |tree| {
            let phase = u32::try_from(tick % 240).expect("cycle fits u32");
            let remaining = 120_u32.abs_diff(phase);
            tree.set_fraction(status::BAR, f64::from(remaining) / 120.0);
            for (id, value) in [status::TIMER]
                .into_iter()
                .chain(status::COUNTS)
                .zip(expected_labels(tick))
            {
                tree.set_text(id, &value);
            }
        });
        reference.first_difference(&frame.rgba, region)
    }
}

fn expected_labels(tick: u64) -> [String; 4] {
    let minutes = (tick % 6000) / 60;
    let seconds = tick % 60;
    let counter = |step| format!("{:03}", (tick % 1000) * step % 1000);
    [
        format!("{minutes:02}:{seconds:02}"),
        counter(1),
        counter(3),
        counter(7),
    ]
}

fn stamp(frame: &SubmittedImage) -> Option<PaintStamp> {
    let witness = frame.sample.interface.expect("interface stage");
    assert!(!witness.stamp_mismatch && !witness.extent_mismatch);
    assert_eq!(witness.dropped_rows, 0);
    witness.stamp
}

#[expect(
    clippy::print_stdout,
    reason = "retains switch coverage in the Metal proof log"
)]
fn check_transition(
    rig: &mut Rig,
    before: &Publication,
    after: &Publication,
    prior: SubmittedImage,
) -> SubmittedImage {
    let mut frame = prior;
    let first_tick = frame.sample.tick;
    let mut switched = false;
    let mut old_frames = 0;
    let mut new_frames = 0;
    for _ in 0..160 {
        let applied = stamp(&frame).expect("last good remain resident");
        let old_diff = before.difference(&frame);
        let new_diff = after.difference(&frame);
        if applied == before.stamp() {
            assert!(!switched, "old publication returned");
            assert_eq!(old_diff, None, "old layout on tick {}", frame.sample.tick);
            assert!(new_diff.is_some(), "new reference must differ");
            old_frames += 1;
        } else {
            assert_eq!(applied, after.stamp());
            assert_eq!(new_diff, None, "new layout on tick {}", frame.sample.tick);
            assert!(old_diff.is_some(), "old reference must differ");
            switched = true;
            new_frames += 1;
            if new_frames == 4 {
                assert!(old_frames > 0);
                println!(
                    "{} → {} at scale {} → {}: {old_frames} old and {new_frames} new submissions; ticks {first_tick}–{}, timer and all counters compared on each",
                    before.head.generation,
                    after.head.generation,
                    before.scale,
                    after.scale,
                    frame.sample.tick
                );
                return frame;
            }
        }
        let next = rig.draw();
        assert_eq!(
            next.sample.submission_order,
            frame.sample.submission_order + 1,
            "every submitted frame compared"
        );
        assert_eq!(
            next.sample.tick,
            frame.sample.tick + 1,
            "every measured status tick advances once"
        );
        for (before, after) in expected_labels(frame.sample.tick)
            .into_iter()
            .zip(expected_labels(next.sample.tick))
        {
            assert_ne!(before, after, "every displayed value changes");
        }
        frame = next;
    }
    panic!("publication did not switch");
}

fn caption_height(publication: &Publication) -> f64 {
    let frame = publication.art.frame_at(0).expect("health badge");
    let mut tree = status::tree(Image {
        sheet: 2,
        frame: 0,
        texels: frame.size,
    })
    .expect("status tree");
    tree.prepare_layout(
        Viewport {
            extent: EXTENT,
            scale: publication.scale,
            insets: [0.0; 4],
        },
        PaintInputs {
            stamp: None,
            fonts: &[Font {
                product: &publication.font,
                sheet: 1,
            }],
            white: status::WHITE_TILE,
        },
    )
    .expect("layout from this publication");
    tree.bounds(25).expect("Gems Found caption").size[1]
}

fn ink_width(font: &FontProduct) -> u32 {
    let strike = font.strike(12.0, 1).expect("display strike");
    let glyph = strike.glyph('6').expect("timer digit");
    strike.frame(glyph.frame).expect("ink frame").size[0]
}

#[test]
#[cfg_attr(
    not(target_os = "macos"),
    ignore = "requires Mac Metal submitted images"
)]
fn font_art_and_scale_switch_only_with_their_fresh_status_layout() {
    let scratch = Scratch::new("status-switch");
    let source = scratch.join("source");
    for folder in ["type", "status"] {
        let directory = source.join(folder);
        std::fs::create_dir_all(&directory).expect("source directory");
        for entry in std::fs::read_dir(store::fixture().join(folder)).expect("fixture") {
            let entry = entry.expect("file");
            std::fs::copy(entry.path(), directory.join(entry.file_name())).expect("copy source");
        }
    }
    let mut cooker = store::cook(source.clone(), scratch.join("store"));
    let gpu = metal::open();
    let mut rig = Rig::new(&gpu, &cooker, 1);
    let before = Publication::from(&cooker, 1);
    let frame = thrum_test_support::wait::until(
        "resident first status display",
        Duration::from_secs(8),
        || {
            let frame = rig.draw();
            (stamp(&frame) == Some(before.stamp())).then_some(frame)
        },
    );
    let edited = store::fixture()
        .with_file_name("status-wide")
        .join("Roboto-StatusWide.ttf");
    std::fs::copy(edited, source.join("type/Roboto-Regular.ttf"))
        .expect("save wide outline and advances");
    assert_eq!(
        cooker.cook_all().expect("widened font publication").failed,
        []
    );
    let after_font = Publication::from(&cooker, 1);
    assert!(
        ink_width(&after_font.font) > ink_width(&before.font),
        "the edit widens the rasterized outline"
    );
    assert!(
        caption_height(&after_font) > caption_height(&before),
        "the widened font moves the second word to a new line"
    );
    let frame = check_transition(&mut rig, &before, &after_font, frame);

    let art = source.join("status/health.png");
    let original = image::open(&art).expect("badge").into_rgba8();
    let widened = image::imageops::resize(
        &original,
        original.width() + 5,
        original.height(),
        image::imageops::FilterType::Nearest,
    );
    widened.save(art).expect("save badge edit");
    assert_eq!(
        cooker.cook_all().expect("widened art publication").failed,
        []
    );
    let after_art = Publication::from(&cooker, 1);
    assert_ne!(
        after_art.art.frame_at(0).expect("new badge").size,
        after_font.art.frame_at(0).expect("old badge").size
    );
    let frame = check_transition(&mut rig, &after_font, &after_art, frame);

    assert!(rig.runtime.source().set_interface_scale(2));
    let after_scale = Publication::from(&cooker, 2);
    check_transition(&mut rig, &after_art, &after_scale, frame);
}
