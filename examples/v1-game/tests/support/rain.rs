//! The checked-in populated exit assets on the same native recipe as the V1 tests.
use std::{fs, path::Path};

use thrum_asset::{AssetStore, Profile};
use thrum_asset_gpu::rows::CookedRows;
use thrum_cook::{CookConfig, Cooker, Durability, Quality};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameHost},
};
use thrum_test_support::{Opened, Scratch};
use v1_game::{GameHost, GameScene, GameSetup, GameWorld};

pub(crate) const EXTENT: [u32; 2] = [288, 162];

pub(crate) type Host = FrameHost<GameWorld, GameScene>;

pub(crate) fn open(label: &str) -> Option<Opened> {
    thrum_test_support::open(
        label,
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
}

pub(crate) struct Store {
    store: AssetStore,
    scratch: Scratch,
}

impl Store {
    pub(crate) fn cook() -> Self {
        let scratch = Scratch::new("rain-valley");
        let source = scratch.join("source");
        fs::create_dir_all(&source).expect("exit source");
        for folder in ["assets/showcase", "assets/exit", "assets/rain"] {
            let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join(folder);
            for file in fs::read_dir(assets).expect("authored products") {
                let file = file.expect("authored file");
                if file
                    .path()
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(|extension| matches!(extension, "gltf" | "bin" | "png"))
                {
                    fs::copy(file.path(), source.join(file.file_name())).expect("source copy");
                }
            }
        }
        let store = scratch.join("store");
        Cooker::open(CookConfig {
            source,
            store: store.clone(),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        })
        .expect("exit cooker")
        .cook_all()
        .expect("cooked exit");
        Self {
            store: AssetStore::new(store),
            scratch,
        }
    }

    pub(crate) async fn host(&self, gpu: &Opened, setup: GameSetup) -> Host {
        let rows = CookedRows::from_store(
            &self.store,
            [
                "exit-boulder",
                "exit-scrub",
                "exit-cactus",
                "exit-standing-rock",
                "exit-balloon",
            ]
            .map(str::to_owned)
            .to_vec(),
            gpu.device.limits().max_storage_buffer_binding_size,
        )
        .expect("cooked rows");
        let mut host = setup
            .prepare_with_rows(
                &gpu.device,
                &gpu.queue,
                &gpu.capabilities,
                FrameConfig {
                    render_extent: EXTENT,
                    surface: Some(SurfaceProfile {
                        extent: EXTENT,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                    }),
                },
                Some(rows),
            )
            .await
            .expect("prepared exit");
        let source = self.scratch.join("store").into();
        let (camera, assets) = if setup.rain == v1_game::RainSelection::Off {
            v1_game::prepare_cooked_valley(source).expect("authored valley")
        } else {
            v1_game::prepare_rain_cooked_valley(source, thrum_asset_gpu::CookedDrawPath::Auto)
                .expect("authored valley and rain patch")
        };
        let assets = assets
            .with_rows(host.cooked_rows().expect("host rows"))
            .expect("coupled rows");
        let stats = assets.stats();
        host.set_game_contribution(Box::new(assets))
            .expect("cooked contribution");
        host.set_scene_camera(camera).expect("authored camera");
        host.set_time_of_day(0.5).expect("noon");
        host.world_mut().pause().expect("hold initial endpoint");
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rain test surface"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let surface = thrum_render::runtime::SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        thrum_asset_gpu::await_drawn_on_surface(
            &mut host,
            &stats,
            thrum_asset_gpu::Drawn::Any,
            &surface,
        )
        .expect("cooked exit terrain and props submitted");
        assert_eq!(host.mesh_rows().and_then(|rows| rows.generation()), Some(1));
        host
    }
}
