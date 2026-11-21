//! Observe the indirect submissions as well as the image: clipping an unculled
//! draw can produce the right pixels while doing all of its vertex work.
use super::sun::box_instance;
use super::*;
use crate::synthetic::{CullMode, FrameLighting, SyntheticMeshChoice, SyntheticMeshes};
use crate::visibility::casters::CasterCulling;

const SIDE: u32 = 64;

struct Asks {
    slots: Vec<SecondaryView>,
    requests: [Option<SecondaryViewRequest>; 2],
    order: [usize; 2],
}

impl SceneContribution for Asks {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), String> {
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        for i in self.order {
            if let Some(request) = self.requests[i] {
                self.slots[i].request(&request).expect("admitted camera");
            }
        }
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    capabilities: crate::capabilities::DeviceCapabilities,
}

impl Gpu {
    fn read(&self, source: &wgpu::Buffer) -> Vec<u8> {
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("caster submission proof"),
            size: source.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_buffer_to_buffer(source, 0, &output, 0, source.size());
        encoder.map_buffer_on_submit(&output, wgpu::MapMode::Read, .., |r| r.expect("map"));
        self.queue.submit([encoder.finish()]);
        self.wait();
        output.slice(..).get_mapped_range().expect("bytes").to_vec()
    }

    fn wait(&self) {
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("submitted proof");
    }

    fn members(&self, prepared: &PreparedSynthetic, culling: &CasterCulling) -> Vec<Vec<u32>> {
        culling
            .lists()
            .map(|(visible, args)| self.list_members(prepared, visible, args))
            .collect()
    }

    fn list_members(
        &self,
        prepared: &PreparedSynthetic,
        visible: &wgpu::Buffer,
        args: &wgpu::Buffer,
    ) -> Vec<u32> {
        let route = prepared.bucket_route.as_ref().expect("bucket route");
        let args = self.read(args);
        let visible = self.read(visible);
        route
            .caster_draws()
            .iter()
            .flat_map(|draw| {
                let at = draw.bucket as usize * 20 + 4;
                let count = u32::from_le_bytes(args[at..at + 4].try_into().expect("count"));
                let start = draw.member_offset as usize;
                visible[start..start + count as usize * 4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|word| u32::from_le_bytes(*word))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    async fn prepare(&self, scene: &SyntheticScene) -> (PreparedSynthetic, OffscreenTarget, Asks) {
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &self.capabilities,
            &self.queue,
            scene,
            FrameLighting::Diagnostic(SyntheticLight {
                direction_to_light: [0.5, 0.75, 0.3],
                irradiance: [4.0; 3],
            }),
            1.0,
        )
        .await
        .expect("scene");
        assert_eq!(prepared.camera_culling(), scene.camera_culling);
        prepared
            .prepare_secondary_views(
                &self.queue,
                &[SecondaryViewSlot::new(1.0).expect("slot"); 2],
                [SIDE; 2],
                &[],
            )
            .expect("views");
        prepared.set_contribution_passes(ContributionPasses {
            upload: true,
            compute: false,
        });
        let target = OffscreenTarget::new(&self.device, &prepared, SIDE, SIDE).expect("target");
        let asks = Asks {
            slots: prepared.secondary_handles.clone(),
            requests: [None; 2],
            order: [0, 1],
        };
        (prepared, target, asks)
    }

    fn frame(
        &self,
        prepared: &PreparedSynthetic,
        target: &OffscreenTarget,
        asks: &mut Asks,
    ) -> Frame {
        let mut main = target.readback(&self.device);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_contributed(
                &mut encoder,
                1,
                target,
                None,
                Some(&mut main),
                Some(asks),
                None,
                &mut Vec::new(),
            )
            .expect("frame");
        let colours: Vec<_> = prepared
            .secondary
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                asks.requests[index].map(|_| {
                    let output = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("secondary caster colour proof"),
                        size: u64::from(SIDE * SIDE * 8),
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });
                    encoder.copy_texture_to_buffer(
                        slot.handle.colour.as_image_copy(),
                        wgpu::TexelCopyBufferInfo {
                            buffer: &output,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(SIDE * 8),
                                rows_per_image: Some(SIDE),
                            },
                        },
                        wgpu::Extent3d {
                            width: SIDE,
                            height: SIDE,
                            depth_or_array_layers: 1,
                        },
                    );
                    encoder.map_buffer_on_submit(&output, wgpu::MapMode::Read, .., |r| {
                        r.expect("colour map");
                    });
                    output
                })
            })
            .collect();
        self.queue.submit([encoder.finish()]);
        self.wait();
        Frame {
            main: main.try_collect(1).expect("main").expect("main bytes"),
            main_visible: target
                .visibility
                .as_ref()
                .map(|target| self.list_members(prepared, target.visible(), target.args())),
            colours: colours
                .iter()
                .map(|b| {
                    b.as_ref()
                        .map(|b| b.slice(..).get_mapped_range().expect("colour").to_vec())
                })
                .collect(),
            main_casters: prepared
                .caster_culling
                .as_ref()
                .map(|(c, _)| self.members(prepared, c)),
            casters: prepared
                .secondary
                .iter()
                .map(|slot| {
                    slot.sun
                        .as_ref()
                        .expect("Sun")
                        .culling
                        .as_ref()
                        .map(|(c, _)| self.members(prepared, c))
                })
                .collect(),
        }
    }
}

struct Frame {
    main: Vec<u8>,
    main_visible: Option<Vec<u32>>,
    colours: Vec<Option<Vec<u8>>>,
    main_casters: Option<Vec<Vec<u32>>>,
    casters: Vec<Option<Vec<Vec<u32>>>>,
}

fn camera(height: f32, forward: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, height, 0.0],
        [0.0, 0.5, forward],
        [0.0, 1.0, 0.0],
        0.8,
        1.0,
        0.1,
        160.0,
    )
    .expect("camera")
}

fn scene() -> SyntheticScene {
    let mut scene = SyntheticScene::grid(68).expect("scene");
    scene.view = camera(6.0, 20.0);
    scene.sun_shadows = true;
    scene.instances[0] = box_instance(0, [0.0, -0.5, -18.0], [10.0, 0.5, 10.0]);
    scene.instances[1] = box_instance(1, [0.0, 2.0, -15.0], [0.8, 2.0, 0.8]);
    scene.instances[2] = box_instance(2, [0.0, -0.5, 30.0], [12.0, 0.5, 12.0]);
    scene.instances[3] = box_instance(3, [2.0, 3.0, 30.0], [-1.0, 3.0, 1.0]);
    // Every outlier is far outside both light volumes, but still inside the
    // shared height field, so admission cannot silently remove it first.
    for (slot, instance) in scene.instances.iter_mut().enumerate().take(68).skip(4) {
        *instance = box_instance(
            u32::try_from(slot).expect("small scene"),
            [4000.0 + slot as f32 * 10.0, 0.0, 4000.0],
            [0.5; 3],
        );
    }
    scene.meshes = Some(Arc::new(SyntheticMeshes {
        updates: None,
        meshes: Vec::new(),
        slots: vec![SyntheticMeshChoice::Cube; 68],
    }));
    scene
}

#[test]
fn secondary_caster_lists_preserve_off_camera_members_and_match_direct() {
    let Some(admitted) =
        thrum_test_support::gpu("secondary caster compaction", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    let (device, queue) = admitted.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    );
    let gpu = Gpu {
        capabilities: crate::capabilities::DeviceCapabilities::capture(
            &admitted.adapter,
            &device,
            1,
        )
        .with_recipe_override(
            &std::env::var("THRUM_VIEW_RECIPE").unwrap_or_else(|_| "portable".into()),
        )
        .expect("requested recipe"),
        device,
        queue,
    };
    pollster::block_on(async {
        let mut scene = scene();
        let requests = [
            SecondaryViewRequest::new(&camera(6.0, -18.0), [SIDE; 2], None, ViewAir::Camera)
                .expect("request")
                .with_shadows(2, 64.0)
                .expect("profile"),
            SecondaryViewRequest::new(&camera(12.0, 30.0), [SIDE; 2], None, ViewAir::Camera)
                .expect("request")
                .with_shadows(2, 128.0)
                .expect("profile"),
        ];
        let (direct, target, mut asks) = gpu.prepare(&scene).await;
        assert!(
            direct
                .secondary
                .iter()
                .all(|slot| slot.sun.as_ref().expect("Sun").culling.is_none())
        );
        asks.requests = requests.map(Some);
        let oracle = gpu.frame(&direct, &target, &mut asks);
        for mode in [CullMode::Frustum, CullMode::Occlusion] {
            scene.camera_culling = mode;
            let (mut prepared, target, mut asks) = gpu.prepare(&scene).await;
            assert_eq!(
                prepared.instance_upload.active_count(),
                68,
                "all casters admitted before culling"
            );
            let idle = gpu.frame(&prepared, &target, &mut asks);
            let mut alone = Vec::new();
            for (slot, request) in requests.iter().enumerate() {
                asks.requests = [None; 2];
                asks.requests[slot] = Some(*request);
                let frame = gpu.frame(&prepared, &target, &mut asks);
                assert_eq!(frame.main, idle.main);
                assert_eq!(frame.main_casters, idle.main_casters);
                assert_eq!(
                    frame.colours[slot], oracle.colours[slot],
                    "{mode:?} slot {slot}: unculled image"
                );
                let lists = frame.casters[slot].as_ref().expect("own compaction");
                assert!(
                    lists[0].len() < lists[1].len(),
                    "near and far use different light volumes"
                );
                for (layer, members) in lists.iter().enumerate() {
                    assert_ne!(members.as_slice(), &[0_u32; 0]);
                    assert!(
                        members.iter().all(|&member| member < 4),
                        "outside light volume: {members:?}"
                    );
                    eprintln!(
                        "{mode:?} slot {slot} layer {layer}: {} / 68 casters {members:?}",
                        members.len()
                    );
                }
                if slot == 0 {
                    assert!(
                        !frame
                            .main_visible
                            .as_ref()
                            .expect("camera culling")
                            .contains(&1),
                        "caster is outside the main camera's actual submissions"
                    );
                    assert!(lists[1].contains(&1), "off-main-frustum caster survives");
                }
                alone.push(frame);
            }
            assert_ne!(
                alone[0].casters[0], alone[1].casters[1],
                "different view volumes"
            );
            for order in [[0, 1], [1, 0]] {
                asks.requests = requests.map(Some);
                asks.order = order;
                let pair = gpu.frame(&prepared, &target, &mut asks);
                assert_eq!(pair.main, idle.main);
                assert_eq!(pair.main_casters, idle.main_casters);
                for (slot, single) in alone.iter().enumerate() {
                    assert_eq!(pair.colours[slot], single.colours[slot]);
                    assert_eq!(pair.casters[slot], single.casters[slot]);
                }
            }
            asks.requests = [None; 2];
            let idle_after = gpu.frame(&prepared, &target, &mut asks);
            for (slot, single) in alone.iter().enumerate() {
                assert_eq!(
                    idle_after.casters[slot], single.casters[slot],
                    "idle lists retain bytes"
                );
            }
            asks.requests[0] = Some(requests[0].with_shadows(0, 64.0).expect("height only"));
            let zero = gpu.frame(&prepared, &target, &mut asks);
            assert_eq!(
                zero.casters, idle_after.casters,
                "zero cascades do no compaction"
            );
            asks.requests[0] = Some(requests[0].with_shadows(1, 64.0).expect("far only"));
            let one = gpu.frame(&prepared, &target, &mut asks);
            assert_eq!(
                one.casters[0].as_ref().expect("lists")[0],
                idle_after.casters[0].as_ref().expect("lists")[0],
                "one cascade skips near work"
            );
            // Rebuild the member groups after crossing a workgroup boundary.
            let changed: Vec<_> = (0..68).collect();
            for instance in &mut scene.instances {
                instance.ids[3] = 0;
            }
            prepared
                .upload_instances(&gpu.queue, &scene.instances, &changed)
                .expect("activity update");
            asks.requests = requests.map(Some);
            let empty = gpu.frame(&prepared, &target, &mut asks);
            assert!(
                empty.casters.iter().all(|slots| slots
                    .as_ref()
                    .expect("lists")
                    .iter()
                    .all(Vec::is_empty))
            );
            scene = self::scene();
        }
    });
}
