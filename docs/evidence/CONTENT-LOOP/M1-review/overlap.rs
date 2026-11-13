//! CONTENT-LOOP leases exercised through the game's ordinary frame host.
#![cfg(target_os = "macos")]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thrum_asset::Profile;
use thrum_asset_gpu::{
    COPY_UNIT, CookedAssets, CookedConfig, CookedStats, LeaseRequest, ProductLease,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::{capabilities::DeviceCapabilities, contribution::SceneContributions};
use v1_game::{
    FrameConfig, FrameHost,
    lent::{LentDiagnostic, LentWitness},
};

fn presenting() -> FrameConfig {
    FrameConfig {
        surface: Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: [640, 360],
        }),
        ..FrameConfig::default()
    }
}

fn fixture(root: &std::path::Path) -> CookConfig {
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("source");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 8, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("foam.linear.png"), 32, [200, 40, 20]).expect("texture");
    for index in 0..12 {
        demo::write_cloud_volume(&source.join(format!("cloud{index}.tvol")), 128, index + 1)
            .expect("volume");
    }
    CookConfig {
        source,
        store: root.join("store"),
        profiles: vec![Profile::Portable, Profile::DesktopBc],
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

fn config(cook: &CookConfig) -> CookedConfig {
    let mut config = CookedConfig::new(cook.store.clone(), vec![]);
    config.upload_budget = COPY_UNIT;
    config.texture_preference = Some(vec![Profile::Portable]);
    config.poll_interval = Duration::from_millis(1);
    config.leased = (0..12)
        .map(|index| LeaseRequest {
            name: format!("cloud{index}"),
            dimension: wgpu::TextureDimension::D3,
        })
        .collect();
    config.leased.push(LeaseRequest {
        name: "foam".into(),
        dimension: wgpu::TextureDimension::D2,
    });
    config
}

async fn gpu(
    instance: &wgpu::Instance,
    generation: u64,
    compressed: bool,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Metal adapter");
    let features = if compressed {
        DeviceCapabilities::requested_content_features(&adapter)
    } else {
        wgpu::Features::empty()
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await
        .expect("device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
    (device, queue, capabilities)
}

fn finish(host: &mut FrameHost, device: &wgpu::Device) {
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU complete");
    while host.poll_completed().expect("retire").is_some() {}
}

fn frame(host: &mut FrameHost, device: &wgpu::Device, clock: &mut Duration) {
    *clock += Duration::from_millis(16);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lent test drawable"),
        size: wgpu::Extent3d {
            width: 640,
            height: 360,
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
    host.drive(
        *clock,
        Some(&v1_game::SurfaceOutput {
            view: &view,
            extent: [640, 360],
        }),
    )
    .expect("frame");
    finish(host, device);
}

fn until(
    lease: &ProductLease,
    host: &mut FrameHost,
    device: &wgpu::Device,
    clock: &mut Duration,
    predicate: impl Fn() -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(90);
    while !predicate() {
        assert!(Instant::now() < deadline, "publication timed out");
        // Discard every upload recording, including the one carrying the last copy. None
        // may offer an unsubmitted set; the following ordinary frame re-records its copies.
        let before = lease.offered();
        *clock += Duration::from_millis(16);
        host.advance(*clock)
            .expect("advance discard")
            .encode_scene()
            .expect("encode discard")
            .skip();
        let after = lease.offered();
        assert!(
            match (&before, &after) {
                (None, None) => true,
                (Some(before), Some(after)) => Arc::ptr_eq(before, after),
                _ => false,
            },
            "discarded last-copy recording must keep the previous offer"
        );
        frame(host, device, clock);
    }
}

fn attach(
    host: &mut FrameHost,
    config: CookedConfig,
    consumer_first: bool,
) -> (
    ProductLease,
    Arc<Mutex<CookedStats>>,
    Arc<Mutex<LentWitness>>,
) {
    let assets = CookedAssets::new(config).expect("assets");
    let lease = assets.lease();
    let stats = assets.stats();
    let (consumer, witness) = LentDiagnostic::new(lease.clone());
    let entries: Vec<Box<dyn thrum_render::contribution::SceneContribution>> = if consumer_first {
        vec![Box::new(consumer), Box::new(assets)]
    } else {
        vec![Box::new(assets), Box::new(consumer)]
    };
    host.set_scene_contribution(Box::new(SceneContributions::new(entries)))
        .expect("attach");
    (lease, stats, witness)
}

fn main() {
 let root=std::env::temp_dir().join(format!("thrum-review-overlap-{}",std::process::id()));
 let cook=fixture(&root);
 let mut cooker=Cooker::open(cook.clone()).unwrap();
 assert!(cooker.cook_all().unwrap().failed.is_empty());
 pollster::block_on(async {
  let instance=wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
  let (device, _queue, caps)=gpu(&instance,1,false).await;
  let mut host=FrameHost::new_with_capabilities(&device,&_queue,&caps,presenting()).await.unwrap();
  let (lease,stats,_witness)=attach(&mut host,config(&cook),false);
  let mut clock=Duration::ZERO;
  until(&lease,&mut host,&device,&mut clock,||stats.lock().unwrap().drawn==Some(1));
  demo::write_cloud_volume(&cook.source.join("cloud0.tvol"),128,98).unwrap();
  assert!(cooker.cook_all().unwrap().failed.is_empty());
  until(&lease,&mut host,&device,&mut clock,||lease.offered().is_some_and(|s|s.generation==2));
  assert_eq!(stats.lock().unwrap().drawn,Some(1));
  let ready=lease.offered().unwrap();
  // No switch frame yet. The next source edit changes only cloud1.
  demo::write_cloud_volume(&cook.source.join("cloud1.tvol"),128,99).unwrap();
  assert!(cooker.cook_all().unwrap().failed.is_empty());
  let deadline=Instant::now()+Duration::from_secs(30);
  while stats.lock().unwrap().staged!=Some(3) {
   assert!(Instant::now()<deadline);
   std::thread::park_timeout(Duration::from_millis(1));
  }
  frame(&mut host,&device,&mut clock);
  assert_eq!(stats.lock().unwrap().drawn,Some(2));
  until(&lease,&mut host,&device,&mut clock,||stats.lock().unwrap().drawn==Some(3));
  let next=lease.offered().unwrap();
  let a=&ready.products[0]; let b=&next.products[0];
  println!("cloud0 unchanged hash={}, texture={}, view={}, serial={}",a.hash==b.hash,a.texture==b.texture,a.view==b.view,a.serial==b.serial);
  let stats=stats.lock().unwrap();
  println!("generation2 upload={} generation3 upload={}",stats.swaps[1].uploaded_bytes,stats.swaps[2].uploaded_bytes);
  assert_eq!(a.view,b.view,"M1.3: unchanged product must retain its view");
 });
 std::fs::remove_dir_all(root).unwrap();
}
