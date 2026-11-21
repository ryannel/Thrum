//! The cooked game the proving ground's cooked tests build: a scratch store with the demo sources, its
//! cook, a device for a draw path and the game host on it. It shares code only: every call makes its
//! own folder, device and host, so no test waits for another.

use thrum_asset::Profile;
use thrum_asset_gpu::CookedDrawPath;
use thrum_cook::{CookConfig, CookReport, Cooker, Durability, Quality, demo};
use thrum_render::capabilities::{
    DeviceCapabilities, DeviceRecipe, DeviceRestriction, FIRST_DEVICE_GENERATION, OpenedDevice,
};
use thrum_render::runtime::{FrameConfig, FrameHost};
use thrum_test_support::{Gpu, Scratch};
use v1_game::{GameScene, GameSetup, GameWorld};

pub(crate) type Host = FrameHost<GameWorld, GameScene>;

/// The demo sources: a 128-cell terrain grid, a 512-texel checker and a cloud volume.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tree {
    /// Edge of the cubic cloud volume.
    pub(crate) volume: u32,
    /// The checker's colour.
    pub(crate) checker: [u8; 3],
}

/// A scratch folder holding the demo sources and the store they cook into, removed when dropped.
#[derive(Debug)]
pub(crate) struct Sources {
    /// Sources under `source/`, the store at `store/`, every profile, a fast process-durable cook.
    pub(crate) cook: CookConfig,
    _scratch: Scratch,
}

impl Sources {
    /// Write the sources; nothing is cooked and no store is opened.
    pub(crate) fn new(name: &str, tree: Tree) -> Self {
        let scratch = Scratch::new(name);
        let source = scratch.join("source");
        for directory in ["meshes", "textures"] {
            std::fs::create_dir_all(source.join(directory)).expect("source tree");
        }
        demo::write_grid_gltf(&source.join("meshes"), "terrain", 128, 0.25).expect("mesh");
        demo::write_checker_png(&source.join("textures/checker.png"), 512, tree.checker)
            .expect("texture");
        demo::write_cloud_volume(&source.join("cloud.tvol"), tree.volume, 1).expect("volume");
        let cook = CookConfig {
            source,
            store: scratch.join("store"),
            profiles: Profile::ALL.to_vec(),
            quality: Quality::Fast,
            durability: Durability::Process,
        };
        Self {
            cook,
            _scratch: scratch,
        }
    }

    /// Open the store's cooker.
    pub(crate) fn cooker(&self) -> Cooker {
        Cooker::open(self.cook.clone()).expect("cooker")
    }

    /// Open the cooker and cook every source once; the caller judges the report.
    pub(crate) fn cook_all(&self) -> (Cooker, CookReport) {
        let mut cooker = self.cooker();
        let report = cooker.cook_all().expect("cook");
        (cooker, report)
    }
}

/// How a test file opens its devices. The two files differ on purpose, and each case keeps the device
/// it had.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Opener {
    /// The recipe is given the case's generation; otherwise it keeps the first-device default and the
    /// generation reaches only the captured capabilities.
    pub(crate) recipe_generation: bool,
    /// The adapter must offer indirect execution and the path must be `Indirect` or `Direct`;
    /// otherwise any path other than `Direct` keeps the captured capabilities as they are.
    pub(crate) indexed_only: bool,
    /// What a refused device request reports.
    pub(crate) refused: &'static str,
}

/// An admitted device at default limits with every feature the profile enables (every compressed
/// family the adapter offers, as the game's platform layer asks, and timestamps), its capabilities
/// captured at `generation` and restricted to `path`.
pub(crate) async fn device(
    gpu: &Gpu,
    opener: Opener,
    generation: u64,
    path: CookedDrawPath,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let restriction = DeviceRestriction {
        limits: Some(wgpu::Limits::default()),
        features: None,
        ..Default::default()
    };
    let OpenedDevice { device, queue, .. } = DeviceRecipe {
        generation: if opener.recipe_generation {
            generation
        } else {
            FIRST_DEVICE_GENERATION
        },
        restriction: Some(&restriction),
        ..Default::default()
    }
    .open_admitted(&gpu.instance, &gpu.adapter)
    .await
    .expect(opener.refused);
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, generation);
    let capabilities = if opener.indexed_only {
        assert!(
            capabilities
                .downlevel()
                .expect("adapter facts")
                .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
        );
        match path {
            CookedDrawPath::Indirect => capabilities,
            CookedDrawPath::Direct => capabilities.without_indirect_execution(),
            CookedDrawPath::Auto | CookedDrawPath::MeshStages => {
                panic!("expected an indexed test path")
            }
        }
    } else if path == CookedDrawPath::Direct {
        capabilities.without_indirect_execution()
    } else {
        capabilities
    };
    (device, queue, capabilities)
}

/// The game scene prepared on a device.
pub(crate) async fn host(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    capabilities: &DeviceCapabilities,
    config: FrameConfig,
) -> Host {
    GameSetup::default()
        .prepare(device, queue, capabilities, config)
        .await
        .expect("prepared game scene")
}

/// A game host on its own device for a draw path. The host holds its own queue and capabilities.
pub(crate) struct Game {
    pub(crate) host: Host,
    pub(crate) device: wgpu::Device,
}

/// Open a device as `opener` does and prepare the game scene on it.
pub(crate) async fn game(
    gpu: &Gpu,
    opener: Opener,
    generation: u64,
    path: CookedDrawPath,
    config: FrameConfig,
) -> Game {
    let (device, queue, capabilities) = device(gpu, opener, generation, path).await;
    let host = host(&device, &queue, &capabilities, config).await;
    Game { host, device }
}
