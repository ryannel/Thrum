use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use thrum_asset::{FontProduct, FontStrike, Profile};
use thrum_asset_gpu::{
    CookedAssets, CookedConfig, CookedStats, FontCandidate, FontScale, ProductLease,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, SceneFrame, SceneSource, SpriteSceneRequests,
        SubmittedImage, SurfaceOutput,
    },
    sprite::{
        EmbeddedSheet, NamedSheet, SpriteFrame, SpriteRect, SpriteRow, SpriteScene, SpriteView,
    },
    synthetic::FrameLighting,
};
use thrum_ui::{
    Alignment, Elision, InterfaceConfig, InterfaceSheets, PaintList, PaintListBuilder, TextOptions,
    TextRun,
};

use super::sprite_sheet_fixture::World;

pub(super) const EXTENT: [u32; 2] = [160, 96];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

pub(super) fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/game-ui/font-source")
}

pub(super) fn cook(root: &Path, edited: bool) -> Cooker {
    let name = if edited { "edited" } else { "live" };
    let source = root.join(name);
    for folder in ["type", "pixel"] {
        let directory = source.join(folder);
        std::fs::create_dir_all(&directory).expect("font source");
        for entry in std::fs::read_dir(fixture().join(folder)).expect("font fixture") {
            let entry = entry.expect("file");
            std::fs::copy(entry.path(), directory.join(entry.file_name())).expect("copy font");
        }
    }
    if edited {
        edit_pixel(&source, 12, true);
    }
    let mut cooker = Cooker::open(CookConfig {
        source,
        store: root.join(format!("{name}-store")),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("open cooker");
    let report = cooker.cook_all().expect("cook font");
    assert!(report.failed.is_empty(), "{report:?}");
    cooker
}

pub(super) fn edit_pixel(source: &Path, advance: u64, recolour: bool) {
    let directory = source.join("pixel");
    let file = directory.join("font.json");
    let mut description: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).expect("description")).expect("JSON");
    for cell in description["characters"].as_array_mut().expect("cells") {
        if matches!(cell["character"].as_str(), Some("A" | "B")) {
            cell["advance"] = advance.into();
        }
    }
    if recolour {
        let file = directory.join("grid.png");
        let original = image::open(&file).expect("grid").into_rgba8();
        let characters = description["characters"].as_array().expect("cells");
        let glyph_cell = |ch: &str| {
            characters
                .iter()
                .find(|cell| cell["character"].as_str() == Some(ch))
                .and_then(|cell| cell["cell"][0].as_u64())
                .and_then(|index| u32::try_from(index).ok())
                .expect("authored cell")
        };
        let widened = [glyph_cell("A"), glyph_cell("B")];
        let grid = image::RgbaImage::from_fn(original.width() / 8 * 12, 8, |x, y| {
            let cell = x / 12;
            let within = x % 12;
            let alpha = if widened.contains(&cell) && y == 3 {
                255
            } else {
                original.get_pixel(cell * 8 + within * 8 / 12, y).0[3]
            };
            image::Rgba([130, 222, 71, alpha])
        });
        grid.save(file).expect("save edited texels");
        description["cell"][0] = 12.into();
    }
    std::fs::write(file, serde_json::to_vec(&description).expect("description"))
        .expect("save advances");
}

pub(super) fn font(cooker: &Cooker, name: &str) -> FontProduct {
    let store = cooker.store();
    let head = store.read_head().expect("HEAD").expect("publication");
    let manifest = store.load_manifest(&head).expect("manifest");
    let entry = manifest
        .resolve(name, &[Profile::Portable])
        .expect("font entry");
    FontProduct::open(store.map(&entry.product).expect("map font")).expect("verified font")
}

pub(super) fn open(recipe: &str) -> thrum_test_support::Opened {
    let mut opened = thrum_test_support::open(
        "submitted interface font rows",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    )
    .expect("Mac Metal target");
    opened.capabilities = opened
        .capabilities
        .with_recipe_override(recipe)
        .expect("native recipe");
    assert_eq!(opened.capabilities.adapter().backend, wgpu::Backend::Metal);
    opened
}

pub(super) struct Rig {
    pub(super) host: FrameHost<World, Scene>,
    pub(super) lease: ProductLease,
    pub(super) scale: FontScale,
    pub(super) stats: Arc<Mutex<CookedStats>>,
    clock: Duration,
    drawable: wgpu::TextureView,
}

impl Rig {
    pub(super) fn new(gpu: &thrum_test_support::Opened, cooker: &Cooker, scale: u32) -> Self {
        let mut config = CookedConfig::new(cooker.store().root().to_path_buf(), vec![]);
        config.lease_font("type");
        config.lease_font("pixel");
        config.upload_budget = 512 << 10;
        config.poll_interval = Duration::from_millis(1);
        let request = config.font_scale_request();
        assert!(request.set(scale));
        let assets = CookedAssets::new(config).expect("residency");
        let mut sprites = SpriteScene::new(solid(), 32);
        sprites.add_indexed_sheet("pixel").expect("world glyphs");
        sprites.set_sheet_source(assets.lease());
        let mut host = pollster::block_on(FrameHost::new_2d(
            &gpu.capabilities,
            &gpu.queue,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(SurfaceProfile {
                    extent: EXTENT,
                    format: FORMAT,
                }),
            },
            World(0),
            Scene {
                lease: assets.lease(),
                origin: [14.0, 4.0],
                value: 0,
                next: None,
                world_rows: Vec::new(),
                drawing: true,
            },
            sprites,
            SpriteSceneRequests::default(),
        ))
        .expect("font host");
        let lease = assets.lease();
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("font lease");
        host.enable_submitted_capture(1).expect("capture");
        let drawable = gpu
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("font test drawable"),
                size: wgpu::Extent3d {
                    width: EXTENT[0],
                    height: EXTENT[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            host,
            lease,
            scale: request,
            stats,
            clock: Duration::ZERO,
            drawable,
        }
    }

    pub(super) fn drawn_scale(&self) -> u32 {
        self.lease
            .committed_font()
            .map_or(0, |font| font.stamp.scale)
    }

    pub(super) fn ready(&mut self) {
        thrum_test_support::wait::until("first font submitted", Duration::from_secs(8), || {
            let image = self.draw();
            image.sample.interface?.stamp.map(|_| ())
        });
    }

    pub(super) fn draw(&mut self) -> SubmittedImage {
        self.record(false).expect("submitted image")
    }

    pub(super) fn record(&mut self, discard: bool) -> Option<SubmittedImage> {
        self.record_when(|_| discard)
    }

    pub(super) fn record_when(
        &mut self,
        discard: impl FnOnce(&CookedStats) -> bool,
    ) -> Option<SubmittedImage> {
        self.clock += Duration::from_millis(16);
        let frame = self
            .host
            .advance(self.clock)
            .expect("advance")
            .encode_scene()
            .expect("record");
        if discard(&self.stats.lock().expect("stats")) {
            frame.skip();
            return None;
        }
        let report = frame
            .submit(Some(&SurfaceOutput {
                view: &self.drawable,
                extent: EXTENT,
            }))
            .expect("submit");
        assert!(report.submitted);
        let image = thrum_test_support::wait::until(
            "read submitted font image",
            Duration::from_secs(8),
            || {
                while self.host.poll_completed().expect("poll").is_some() {}
                self.host.try_submitted_image().expect("capture")
            },
        );
        let stats = image.sample.interface.expect("interface sample");
        assert!(!stats.stamp_mismatch);
        assert_eq!(stats.dropped_rows, 0);
        Some(image)
    }
}

fn solid() -> EmbeddedSheet {
    EmbeddedSheet {
        extent: [1, 1],
        rgba: Arc::from([255; 4]),
        frames: Arc::from([SpriteRect {
            origin: [0; 2],
            size: [1; 2],
        }]),
        texels_per_unit: 1.0,
    }
}

pub(super) struct Scene {
    lease: ProductLease,
    pub(super) origin: [f64; 2],
    pub(super) value: usize,
    pub(super) next: Option<PaintList>,
    pub(super) world_rows: Vec<SpriteRow>,
    pub(super) drawing: bool,
}

impl SceneSource<SourceTick> for Scene {
    fn extract(&self, _: &mut RenderExtractor, _: &SourceTick) -> ExtractReport {
        unreachable!("only sprite rows")
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        unreachable!("only sprite scene")
    }

    fn sprites(&self, _: SourceTick, _: f32, _: [u32; 2]) -> Result<SpriteFrame<'_>, FrameError> {
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [0.0; 2],
                pixels_per_unit: 2.0,
            },
            rows: &self.world_rows,
        })
    }

    fn interface_config(&self) -> Option<InterfaceConfig> {
        Some(InterfaceConfig {
            sheets: vec![NamedSheet::pixel("control", solid())],
            row_capacity: 64,
            cooked: Some(InterfaceSheets {
                source: Arc::new(self.lease.clone()),
                names: vec!["type".into(), "pixel".into()],
            }),
        })
    }

    fn paint_lists(&mut self, extent: [u32; 2]) -> [Option<PaintList>; 2] {
        self.next = self.lease.font_candidate().and_then(|candidate| {
            match self.layout(&candidate, extent) {
                Ok(list) => {
                    self.lease.answer_font(candidate.stamp, Ok(()));
                    Some(list)
                }
                Err(error) => {
                    self.lease.answer_font(candidate.stamp, Err(error));
                    None
                }
            }
        });
        let active = self
            .lease
            .committed_font()
            .and_then(|font| self.layout(&font, extent).ok());
        [active, self.next.clone()]
    }
}

impl Scene {
    fn layout(&self, fonts: &FontCandidate, extent: [u32; 2]) -> Result<PaintList, String> {
        let mut list = PaintListBuilder::new(extent);
        list.stamp(fonts.stamp);
        let scale = fonts.stamp.scale;
        if !self.drawing {
            return Ok(list.finish());
        }
        let font = fonts.font("type").expect("type");
        let run = TextRun::new(font, "AV", options(12.0, scale, [100, 50]))
            .map_err(|error| error.to_string())?;
        run.paint(&mut list, 1, self.origin, [1.0; 4], [0.0; 4]);
        let pixel = fonts.font("pixel").expect("pixel");
        let variant = ["AA AB", "AA BA"][self.value];
        let tail = ["ABABAB", "BABABA"][self.value];
        for (value, y, opts) in [
            (
                variant,
                40.0,
                TextOptions {
                    wrap: true,
                    ..options(8.0, scale, [16 * scale, 16 * scale])
                },
            ),
            (
                tail,
                67.0,
                TextOptions {
                    elision: Elision::Middle,
                    ..options(8.0, scale, [32 * scale, 8 * scale])
                },
            ),
        ] {
            let run = TextRun::new(pixel, value, opts).map_err(|error| error.to_string())?;
            run.paint(&mut list, 2, [5.0, y], [1.0; 4], [0.0; 4]);
        }
        Ok(list.finish())
    }
}

pub(super) const fn options(points: f32, scale: u32, extent: [u32; 2]) -> TextOptions {
    TextOptions {
        points,
        scale,
        extent,
        alignment: Alignment::Left,
        elision: Elision::End,
        wrap: false,
        tabular_digits: false,
        outlined: false,
    }
}

pub(super) fn resident_bytes(font: &FontProduct, scale: u32) -> u64 {
    let texture = font.sheet(scale).expect("sheet").texture();
    texture
        .levels()
        .iter()
        .map(|level| texture.level_data(level).len() as u64)
        .sum()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "known small cooked metrics use integer pen offsets"
)]
pub(super) fn outline_av(image: &mut Reference, strike: FontStrike<'_>, [x, y]: [i32; 2]) {
    let a = strike.glyph('A').expect("A");
    let v = strike.glyph('V').expect("V");
    let kern = strike
        .kerning
        .iter()
        .find(|entry| entry.pair == [65, 86])
        .map_or(0, |pair| pair.pixels);
    let baseline = y + strike.metrics.ascent.ceil() as i32;
    glyph(image, strike, a.frame, [x, baseline], 1);
    glyph(image, strike, v.frame, [x + a.advance + kern, baseline], 1);
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "reference scales and metrics are small integers"
)]
pub(super) fn outline_at_wrong_scale(image: &mut Reference, font: &FontProduct, scale: u32) {
    let strike = font.strike(12.0, 1).expect("unscaled sheet");
    let m = i32::try_from(scale).expect("small scale");
    let baseline = 4 + strike.metrics.ascent.ceil() as i32 * m;
    let a = strike.glyph('A').expect("A");
    let v = strike.glyph('V').expect("V");
    let kern = strike
        .kerning
        .iter()
        .find(|entry| entry.pair == [65, 86])
        .map_or(0, |pair| pair.pixels);
    glyph(image, strike, a.frame, [14, baseline], m);
    glyph(
        image,
        strike,
        v.frame,
        [14 + (a.advance + kern) * m, baseline],
        m,
    );
}

pub(super) fn grid_rows(
    image: &mut Reference,
    font: &FontProduct,
    scale: u32,
    edited: bool,
    value: usize,
) {
    let m = i32::try_from(scale).expect("small scale");
    let strike = font.strike(8.0, 1).expect("pixel grid");
    let pairs = if edited {
        [["…", ""], ["A…B", "B…A"]][value]
    } else {
        [["AA", "AB"], ["AA", "BA"]][value]
    };
    for (line, y) in [
        (pairs[0], 40),
        (pairs[1], 40 + 8 * m),
        (["AB…B", "BA…A"][value], 67),
    ] {
        for (index, ch) in line.chars().enumerate() {
            let frame = strike.glyph(ch).expect("grid glyph").frame;
            glyph(
                image,
                strike,
                frame,
                [
                    5 + i32::try_from(index).expect("small line") * 8 * m,
                    y + 7 * m,
                ],
                m,
            );
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded reference pixels and rounded sRGB codes"
)]
pub(super) fn glyph(
    image: &mut Reference,
    strike: FontStrike<'_>,
    index: u32,
    pen: [i32; 2],
    block: i32,
) {
    let frame = strike.frame(index).expect("atlas frame");
    let texture = strike.sheet.texture();
    let level = &texture.levels()[0];
    let texels = texture.level_data(level);
    for y in 0..frame.size[1] {
        for x in 0..frame.size[0] {
            let at = ((frame.page * level.height + frame.origin[1] + y) * level.width
                + frame.origin[0]
                + x) as usize
                * 4;
            let source = &texels[at..at + 4];
            if source[3] == 0 {
                continue;
            }
            let alpha = f64::from(source[3]) / 255.0;
            let [left, top] = [x, y].map(|value| i32::try_from(value).expect("small frame"));
            let left = pen[0] + (left - frame.pivot[0] as i32) * block;
            let top = pen[1] + (top - frame.pivot[1] as i32) * block;
            for dy in 0..block {
                for dx in 0..block {
                    let (Ok(x), Ok(y)) = (u32::try_from(left + dx), u32::try_from(top + dy)) else {
                        continue;
                    };
                    if x >= EXTENT[0] || y >= EXTENT[1] {
                        continue;
                    }
                    let index = (y * EXTENT[0] + x) as usize * 4;
                    image.blended[index / 4] = source[3] != 255;
                    let pixels = &mut image.rgba;
                    for channel in 0..3 {
                        let colour = decode(source[channel])
                            + decode(pixels[index + channel]) * (1.0 - alpha);
                        pixels[index + channel] = (encode(colour) * 255.0).round() as u8;
                    }
                    pixels[index + 3] =
                        ((alpha + f64::from(pixels[index + 3]) / 255.0 * (1.0 - alpha)) * 255.0)
                            .round() as u8;
                }
            }
        }
    }
}

fn decode(code: u8) -> f64 {
    let code = f64::from(code) / 255.0;
    if code <= 0.04045 {
        code / 12.92
    } else {
        ((code + 0.055) / 1.055).powf(2.4)
    }
}

fn encode(linear: f64) -> f64 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

pub(super) struct Reference {
    pub(super) rgba: Vec<u8>,
    blended: Vec<bool>,
}

impl Reference {
    pub(super) fn new(rgba: Vec<u8>) -> Self {
        Self {
            blended: vec![false; rgba.len() / 4],
            rgba,
        }
    }

    pub(super) fn first_difference(&self, actual: &[u8]) -> Option<usize> {
        assert_eq!(actual.len(), self.rgba.len());
        actual
            .iter()
            .zip(&self.rgba)
            .enumerate()
            .find_map(|(at, (&actual, &expected))| {
                let allowed = u8::from(at % 4 != 3 && self.blended[at / 4]);
                (actual.abs_diff(expected) > allowed).then_some(at)
            })
    }
}
