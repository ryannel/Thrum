use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use image::{Rgba, RgbaImage};
use thrum_asset::{Profile, SheetProduct};
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedStats, FaultProbe, ProductLease};
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource,
        SpriteSceneRequests, SubmittedImage, SurfaceOutput,
    },
    sprite::{
        EmbeddedSheet, SpriteFrame, SpriteRect, SpriteRow, SpriteScene, SpriteSink, SpriteView,
    },
    synthetic::FrameLighting,
};
use thrum_test_support::{Opened, Scratch};

pub(super) const EXTENT: [u32; 2] = [64, 48];
pub(super) const WHITE: [u8; 4] = [255; 4];
pub(super) const RED: [u8; 4] = [241, 31, 17, 255];
pub(super) const GREEN: [u8; 4] = [35, 221, 54, 255];
pub(super) const BLUE: [u8; 4] = [15, 68, 231, 255];
pub(super) const EMBEDDED: [i32; 2] = [-24, 15];
pub(super) const ACTOR: [i32; 2] = [-10, 4];
pub(super) const CENTRE: [i32; 2] = [11, -6];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

static CASES: Mutex<()> = Mutex::new(());

pub(super) struct TestDevice {
    opened: Opened,
    serial: MutexGuard<'static, ()>,
}

pub(super) fn open(test: &str) -> Option<TestDevice> {
    let serial = CASES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let backend = if cfg!(target_os = "macos") {
        Some(wgpu::Backend::Metal)
    } else {
        None
    };
    let opened = thrum_test_support::open(
        test,
        backend,
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )?;
    Some(TestDevice { opened, serial })
}

pub(super) struct Rig {
    pub(super) opened: Opened,
    pub(super) host: FrameHost<World, Scene>,
    pub(super) cooker: Cooker,
    pub(super) folder: PathBuf,
    pub(super) lease: ProductLease,
    pub(super) stats: Arc<Mutex<CookedStats>>,
    pub(super) fault: FaultProbe,
    pub(super) handles: [u32; 2],
    pub(super) slot: u32,
    clock: Duration,
    drawable: wgpu::TextureView,
    _scratch: Scratch,
    _serial: MutexGuard<'static, ()>,
}

impl Rig {
    pub(super) fn new(TestDevice { opened, serial }: TestDevice) -> Self {
        let scratch = Scratch::new("sprite-sheets-gpu");
        let root = scratch.join("source");
        let folder = root.join("actors");
        let retained = root.join("retained");
        std::fs::create_dir_all(&folder).expect("actors folder");
        std::fs::create_dir_all(&retained).expect("retained folder");
        std::fs::write(
            folder.join("sheet.json"),
            r#"{"kind":"pixel","density":1,"page_size":64,"pivots":{"a":[4,4],"b":[4,4]}}"#,
        )
        .expect("description");
        std::fs::write(
            retained.join("sheet.json"),
            r#"{"kind":"pixel","density":1}"#,
        )
        .expect("retained description");
        save(
            &retained.join("still.png"),
            &RgbaImage::from_pixel(8, 8, Rgba(WHITE)),
        );
        save(&folder.join("a.png"), &art(RED, false));
        save(&folder.join("b.png"), &art(RED, true));
        save(&folder.join("centre.png"), &centred());
        let mut cooker = Cooker::open(CookConfig {
            source: root,
            store: scratch.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("cooker");
        assert_eq!(cooker.cook_all().expect("initial publication").failed, []);

        let mut config = CookedConfig::new(scratch.join("store"), vec![]);
        config.lease_sheet("actors");
        config.lease_sheet("retained");
        config.poll_interval = Duration::from_millis(2);
        let fault = FaultProbe::default();
        config.fault = Some(fault.clone());
        let assets = CookedAssets::new(config).expect("placement-free residency");
        let lease = assets.lease();
        let stats = assets.stats();
        let mut sheets = SpriteScene::new(
            EmbeddedSheet {
                extent: [1, 1],
                rgba: WHITE.into(),
                frames: vec![SpriteRect {
                    origin: [0, 0],
                    size: [1, 1],
                }]
                .into(),
                texels_per_unit: 1.0,
            },
            2,
        );
        sheets.world_capacity = 1;
        sheets.set_sheet_source(lease.clone());
        let actors = sheets.add_cooked_sheet("actors").expect("actors slot");
        let handles = ["a", "b"].map(|name| actors.frame(name));
        let slot = actors.slot();
        assert_ne!(handles[0], handles[1]);
        let mut host = pollster::block_on(FrameHost::new_2d(
            &opened.capabilities,
            &opened.queue,
            FrameConfig {
                render_extent: EXTENT,
                surface: Some(profile()),
            },
            World(0),
            Scene {
                actor: row(slot, handles[0], ACTOR),
                fixed: [
                    row(0, 0, EMBEDDED),
                    row(slot, actors.frame("centre"), CENTRE),
                ],
            },
            sheets,
            SpriteSceneRequests::default(),
        ))
        .expect("2D host");
        host.set_scene_contribution(Box::new(
            thrum_render::contribution::SceneContributions::new(vec![Box::new(assets)]),
        ))
        .expect("attach leases without 3D passes");
        host.enable_submitted_capture(1).expect("submitted capture");
        let drawable = drawable(&opened.device);
        Self {
            opened,
            host,
            cooker,
            folder,
            lease,
            stats,
            fault,
            handles,
            slot,
            clock: Duration::ZERO,
            drawable,
            _scratch: scratch,
            _serial: serial,
        }
    }

    pub(super) fn record(&mut self, discard: bool) -> Option<SubmittedImage> {
        self.clock += Duration::from_millis(16);
        let frame = self
            .host
            .advance(self.clock)
            .expect("advance")
            .encode_scene()
            .expect("encode");
        if discard {
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
        Some(thrum_test_support::wait::until(
            "submitted sprite image",
            Duration::from_secs(8),
            || {
                while self.host.poll_completed().expect("poll frame").is_some() {}
                self.host.try_submitted_image().expect("capture")
            },
        ))
    }

    pub(super) fn draw(&mut self) -> SubmittedImage {
        self.record(false).expect("submitted image")
    }

    pub(super) fn ready(&mut self, generation: u64) -> SubmittedImage {
        thrum_test_support::wait::until("sheet became drawable", Duration::from_secs(10), || {
            let image = self.draw();
            let used = image.sample.sprites.expect("sprite stats").sheet_generation;
            (used == Some(generation)).then_some(image)
        })
    }

    pub(super) fn publish(&mut self) -> u64 {
        let report = self.cooker.cook_all().expect("cook");
        assert!(report.failed.is_empty(), "{report:?}");
        report.published.expect("new publication").generation
    }

    pub(super) fn product(&self) -> SheetProduct {
        let store = self.cooker.store();
        let head = store.read_head().expect("HEAD").expect("publication");
        let manifest = store.load_manifest(&head).expect("manifest");
        let hash = manifest
            .resolve("actors", &[Profile::Portable])
            .expect("actors")
            .product;
        SheetProduct::open(store.map(&hash).expect("map actors")).expect("open actors")
    }

    pub(super) fn replacement(&mut self) {
        let (device, queue, caps) = self
            .opened
            .gpu
            .fresh_adapter()
            .device_with_hardware(
                thrum_test_support::Profile::Portable,
                &thrum_test_support::Restriction::default(),
                2,
                &[],
            )
            .expect("replacement device");
        let mut candidate = pollster::block_on(
            self.host
                .device_prepare_inputs()
                .expect("device preparation inputs")
                .prepare(&caps, &queue, Some(profile())),
        )
        .expect("prepare replacement");
        self.host.disable_submitted_capture();
        self.opened.device.destroy();
        self.host
            .publish_device(&mut candidate)
            .expect("publish replacement");
        self.host
            .enable_submitted_capture(2)
            .expect("capture on replacement");
        self.opened.device = device;
        self.opened.queue = queue;
        self.drawable = drawable(&self.opened.device);
    }
}

fn profile() -> SurfaceProfile {
    SurfaceProfile {
        extent: EXTENT,
        format: FORMAT,
    }
}

fn drawable(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite sheet test drawable"),
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
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(super) fn save(path: &std::path::Path, image: &RgbaImage) {
    image.save(path).expect("save fixture PNG");
}

pub(super) fn art(colour: [u8; 4], alternate_trim: bool) -> RgbaImage {
    let mut image = RgbaImage::new(8, 8);
    image.put_pixel(4, 4, Rgba(colour));
    image.put_pixel(5, 4, Rgba(WHITE));
    image.put_pixel(4, 5, Rgba(BLUE));
    if alternate_trim {
        image.put_pixel(1, 2, Rgba(colour));
        image.put_pixel(2, 3, Rgba(WHITE));
    }
    image
}

pub(super) fn centred() -> RgbaImage {
    let mut image = RgbaImage::new(4, 4);
    image.put_pixel(1, 1, Rgba(BLUE));
    image.put_pixel(2, 1, Rgba(WHITE));
    image
}

pub(super) fn row(sheet: u32, frame: u32, point: [i32; 2]) -> SpriteRow {
    SpriteRow {
        sheet,
        ..SpriteRow::new(frame, point.map(f64::from))
    }
}

pub(super) struct World(pub(super) u64);

impl FrameWorld for World {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            timeline: 0,
            tick: self.0,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        self.0 += 1;
        Ok(WorldAdvance {
            tick: self.0,
            completed_ticks: 1,
            overload: None,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<SourceTick>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: self.0,
            },
            timeline: 0,
            page: self.source(),
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

pub(super) struct Scene {
    pub(super) actor: SpriteRow,
    fixed: [SpriteRow; 2],
}

impl SceneSource<SourceTick> for Scene {
    fn extract(&self, _: &mut RenderExtractor, _: &SourceTick) -> ExtractReport {
        unreachable!("sprite-only scene")
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        unreachable!("sprite-only scene")
    }
    fn extract_sprites(&self, sink: &mut SpriteSink<'_>, _: &SourceTick) {
        sink.push(0, 1, 1, self.actor);
    }
    fn sprites(&self, _: SourceTick, _: f32, _: [u32; 2]) -> Result<SpriteFrame<'_>, FrameError> {
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [0.0; 2],
                pixels_per_unit: 1.0,
            },
            rows: &self.fixed,
        })
    }
}

pub(super) fn reference(actor: Option<(&RgbaImage, bool, bool)>, cooked_centre: bool) -> Vec<u8> {
    let mut pixels = [0, 0, 0, 255].repeat((EXTENT[0] * EXTENT[1]) as usize);
    let mut stamp =
        |image: &RgbaImage, point: [i32; 2], pivot: [i32; 2], quarter: bool, flip: bool| {
            for (x, y, colour) in image
                .enumerate_pixels()
                .filter(|(_, _, texel)| texel.0[3] != 0)
            {
                let u = i32::try_from(x).expect("bounded fixture") - pivot[0];
                let v = i32::try_from(y).expect("bounded fixture") - pivot[1];
                let u = if flip { -u - 1 } else { u };
                let [dx, dy] = if quarter { [v, -u - 1] } else { [u, v] };
                let x = 32 + point[0] + dx;
                let y = 24 - point[1] + dy;
                let at = usize::try_from(y * 64 + x).expect("bounded fixture") * 4;
                pixels[at..at + 4].copy_from_slice(&colour.0);
            }
        };
    stamp(
        &RgbaImage::from_pixel(1, 1, Rgba(WHITE)),
        EMBEDDED,
        [0, 0],
        false,
        false,
    );
    if cooked_centre {
        stamp(&centred(), CENTRE, [2, 2], false, false);
    }
    if let Some((image, quarter, flip)) = actor {
        stamp(image, ACTOR, [4, 4], quarter, flip);
    }
    pixels
}
