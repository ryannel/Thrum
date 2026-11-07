use super::*;
use glam::{Mat4, Vec3, Vec4};

const CUBE: MeshRecord = MeshRecord {
    first_index: 0,
    index_count: 36,
    base_vertex: 0,
    bounds_min: [-1.0; 3],
    bounds_max: [1.0; 3],
};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[expect(clippy::cast_precision_loss, reason = "24 random bits fit f32 exactly")]
        let unit = (self.0 >> 40) as f32 / (1u64 << 24) as f32;
        unit
    }
    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
    }
}

#[derive(Clone, Copy)]
struct Instance {
    current: Mat4,
    previous: Vec3,
    hidden: bool,
}

fn instance_bytes(instance: &Instance) -> Vec<u8> {
    let mut previous = instance.current;
    previous.w_axis = instance.previous.extend(1.0);
    let mut floats = Vec::with_capacity(48);
    floats.extend(instance.current.to_cols_array());
    floats.extend(previous.to_cols_array());
    floats.extend([0.0; 12]);
    let mut bytes: Vec<u8> = floats
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    for id in [0_u32, 0, 0, u32::from(!instance.hidden)] {
        bytes.extend(id.to_le_bytes());
    }
    bytes
}

/// Corners of the instance's interpolated cube, in clip space.
fn clip_corners(view_proj: Mat4, instance: &Instance, alpha: f32) -> [Vec4; 8] {
    let mut model = instance.current;
    model.w_axis = instance
        .previous
        .lerp(instance.current.w_axis.truncate(), alpha)
        .extend(1.0);
    std::array::from_fn(|corner| {
        let local = Vec3::new(
            if corner & 1 == 0 { -1.0 } else { 1.0 },
            if corner & 2 == 0 { -1.0 } else { 1.0 },
            if corner & 4 == 0 { -1.0 } else { 1.0 },
        );
        view_proj * model * local.extend(1.0)
    })
}

/// Corners of the world-axis box around the interpolated cube, in clip space.
/// This is the bound the shader tests, so efficiency is judged against it.
fn aabb_clip_corners(view_proj: Mat4, instance: &Instance, alpha: f32) -> [Vec4; 8] {
    let center = instance
        .previous
        .lerp(instance.current.w_axis.truncate(), alpha);
    let extent = instance.current.x_axis.truncate().abs()
        + instance.current.y_axis.truncate().abs()
        + instance.current.z_axis.truncate().abs();
    std::array::from_fn(|corner| {
        let sign = Vec3::new(
            if corner & 1 == 0 { -1.0 } else { 1.0 },
            if corner & 2 == 0 { -1.0 } else { 1.0 },
            if corner & 4 == 0 { -1.0 } else { 1.0 },
        );
        view_proj * (center + sign * extent).extend(1.0)
    })
}

/// True when any corner lies strictly inside the reverse-Z clip volume by a
/// margin: rasterization can cover a pixel, so culling it would be wrong.
fn surely_visible(corners: &[Vec4; 8]) -> bool {
    corners.iter().any(|clip| {
        let margin = 1.0e-3 * clip.w.abs();
        clip.w > 0.0
            && clip.x.abs() < clip.w - margin
            && clip.y.abs() < clip.w - margin
            && clip.z > margin
            && clip.z < clip.w - margin
    })
}

/// True when every corner lies beyond one plane by a margin.
fn surely_outside(corners: &[Vec4; 8]) -> bool {
    let planes: [fn(Vec4) -> f32; 6] = [
        |c| c.w + c.x,
        |c| c.w - c.x,
        |c| c.w + c.y,
        |c| c.w - c.y,
        |c| c.w - c.z,
        |c| c.z,
    ];
    planes.iter().any(|plane| {
        corners
            .iter()
            .all(|&clip| plane(clip) < -1.0e-2 * clip.w.abs().max(1.0))
    })
}

fn view_bytes(view_proj: Mat4) -> Vec<u8> {
    let mut floats = Vec::with_capacity(40);
    floats.extend(view_proj.to_cols_array());
    floats.extend(view_proj.to_cols_array());
    floats.extend([0.0; 8]);
    floats
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn serial_bytes(alpha: f32) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes.extend(alpha.to_le_bytes());
    bytes.extend([0; 12]);
    bytes
}

fn device() -> (wgpu::Device, wgpu::Queue) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for visibility regression");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("visibility device")
    })
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, source: &wgpu::Buffer, bytes: u64) -> Vec<u32> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("visibility readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(source, 0, &staging, 0, bytes);
    queue.submit([encoder.finish()]);
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.expect("map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let words = staging
        .slice(..)
        .get_mapped_range()
        .expect("mapped visibility")
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect();
    staging.unmap();
    words
}

struct Fixture {
    device: wgpu::Device,
    queue: wgpu::Queue,
    prepared: VisibilityPrepared,
    target: VisibilityTarget,
    view: wgpu::Buffer,
    instances: wgpu::Buffer,
    serials: wgpu::Buffer,
    compiled: crate::graph::CompiledGraph,
}

const KEY: PipelineKey = PipelineKey {
    device_generation: 7,
    revision: 1,
    role: ROLE,
};

impl Fixture {
    fn new(capacity: u32) -> Self {
        let (device, queue) = device();
        let uniform = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let view = uniform("test view", VIEW_BYTES);
        let serials = uniform("test serials", 32);
        let instances = storage(
            &device,
            "test instances",
            u64::from(capacity) * crate::abi::INSTANCE_BYTES as u64,
        );
        let prepared = VisibilityPrepared::new(&device, &queue, KEY, capacity, &[CUBE], capacity);
        let target = prepared.target(&device, &view, &instances, &serials);
        let mut graph = FrameGraph::default();
        prepared
            .append_graph(&mut graph, Vec::new())
            .expect("graph");
        let compiled = graph
            .compile(
                KEY.device_generation,
                &std::collections::BTreeSet::from([KEY]),
            )
            .expect("compile");
        Self {
            device,
            queue,
            prepared,
            target,
            view,
            instances,
            serials,
            compiled,
        }
    }

    /// Run one frustum compaction and return the visible slots in list order.
    fn run(&mut self, view_proj: Mat4, instances: &[Instance], alpha: f32) -> Vec<u32> {
        self.upload(view_proj, instances, alpha);
        self.compact(&self.target)
    }

    fn upload(&mut self, view_proj: Mat4, instances: &[Instance], alpha: f32) {
        let bytes: Vec<u8> = instances.iter().flat_map(instance_bytes).collect();
        self.queue.write_buffer(&self.instances, 0, &bytes);
        self.queue
            .write_buffer(&self.view, 0, &view_bytes(view_proj));
        self.queue
            .write_buffer(&self.serials, 0, &serial_bytes(alpha));
        let active = u32::try_from(instances.len()).expect("test count");
        self.prepared.set_active(&self.queue, active);
    }

    /// Run one target's compaction on the uploaded frame.
    fn compact(&self, target: &VisibilityTarget) -> Vec<u32> {
        let active = self.prepared.active;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let (_, pass) = self
            .compiled
            .pass_bindings()
            .next()
            .expect("visibility pass");
        assert!(
            self.prepared
                .encode(&mut encoder, target, 0, KEY.device_generation, pass, None)
                .expect("encode")
        );
        self.queue.submit([encoder.finish()]);
        let args = read(&self.device, &self.queue, target.args(), ARGS_BYTES);
        assert_eq!(args[0], 36, "index count");
        assert_eq!(
            [args[2], args[3], args[4]],
            [0, 0, 0],
            "stable mesh offsets"
        );
        let count = args[1];
        assert!(count <= active);
        let bytes = u64::from(count.max(1)) * 4;
        let mut visible = read(&self.device, &self.queue, target.visible(), bytes);
        visible.truncate(count as usize);
        visible
    }
}

fn check(view_proj: Mat4, instances: &[Instance], alpha: f32, visible: &[u32]) {
    assert!(
        visible.windows(2).all(|pair| pair[0] < pair[1]),
        "instance order is preserved"
    );
    let listed: std::collections::BTreeSet<u32> = visible.iter().copied().collect();
    for (slot, instance) in instances.iter().enumerate() {
        let slot = u32::try_from(slot).expect("slot");
        let corners = clip_corners(view_proj, instance, alpha);
        if instance.hidden {
            assert!(!listed.contains(&slot), "hidden slot {slot} culled");
        } else if surely_visible(&corners) {
            assert!(listed.contains(&slot), "visible slot {slot} kept");
        } else if surely_outside(&aabb_clip_corners(view_proj, instance, alpha)) {
            assert!(!listed.contains(&slot), "outside slot {slot} culled");
        }
    }
}

fn scatter(random: &mut Lcg, count: usize, spread: f32) -> Vec<Instance> {
    (0..count)
        .map(|_| {
            let position = Vec3::new(
                random.range(-spread, spread),
                random.range(-spread, spread),
                random.range(-spread, spread),
            );
            let rotation = glam::Quat::from_euler(
                glam::EulerRot::XYZ,
                random.range(0.0, 6.0),
                random.range(0.0, 6.0),
                random.range(0.0, 6.0),
            );
            let scale = Vec3::new(
                random.range(0.1, 3.0),
                random.range(0.1, 3.0),
                random.range(0.1, 3.0),
            );
            Instance {
                current: Mat4::from_scale_rotation_translation(scale, rotation, position),
                previous: position + Vec3::new(random.range(-2.0, 2.0), 0.0, 0.0),
                hidden: random.next() < 0.05,
            }
        })
        .collect()
}

fn perspective(eye: Vec3, target: Vec3, fov: f32) -> Mat4 {
    glam::camera::rh::proj::directx::perspective_infinite_reverse(fov, 16.0 / 9.0, 0.1)
        * glam::camera::rh::view::look_at_mat4(eye, target, Vec3::Y)
}

#[test]
fn compaction_matches_reference_across_changed_cameras() {
    let mut fixture = Fixture::new(10_000);
    let mut random = Lcg(17);
    let instances = scatter(&mut random, 10_000, 120.0);
    let cameras = [
        perspective(Vec3::new(0.0, 0.0, 200.0), Vec3::ZERO, 1.0),
        perspective(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.2, 0.0), 0.6),
        perspective(
            Vec3::new(30.0, 80.0, -20.0),
            Vec3::new(-10.0, 0.0, 40.0),
            1.4,
        ),
        perspective(Vec3::new(500.0, 0.0, 0.0), Vec3::new(1000.0, 0.0, 0.0), 0.5),
        glam::camera::rh::proj::directx::orthographic(-60.0, 60.0, -34.0, 34.0, 400.0, 0.1)
            * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 200.0, 0.0), Vec3::ZERO, Vec3::Z),
    ];
    let mut kept = Vec::new();
    for (camera, alpha) in cameras.into_iter().zip([0.0, 0.25, 0.5, 1.0, 0.75]) {
        let visible = fixture.run(camera, &instances, alpha);
        check(camera, &instances, alpha, &visible);
        kept.push(visible.len());
    }
    assert!(kept[0] > 5_000, "wide view keeps most: {kept:?}");
    assert!(kept[1] < 5_000, "interior view culls many: {kept:?}");
    assert_eq!(kept[3], 0, "empty view draws nothing: {kept:?}");
}

#[test]
fn partial_workgroups_and_shrinking_active_prefix() {
    let mut fixture = Fixture::new(1_000);
    let mut random = Lcg(3);
    let camera = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    // Zero active rows still runs the offsets dispatch and writes zero instances.
    for count in [1, 255, 256, 257, 999, 1_000, 0, 3] {
        let instances = scatter(&mut random, count, 40.0);
        let visible = fixture.run(camera, &instances, 1.0);
        check(camera, &instances, 1.0, &visible);
        assert!(visible.iter().all(|&slot| (slot as usize) < count));
    }
}

/// Opaque wall depth for the occlusion tests: nearer than most of the cloud
/// (reverse-Z, so larger is nearer) over the left 60% of the target.
const WALL: f32 = 0.002;
const DEPTH_EXTENT: [u32; 2] = [320, 180];

fn wall_depth() -> Vec<f32> {
    let [width, height] = DEPTH_EXTENT;
    (0..height)
        .flat_map(|_| (0..width).map(move |x| if x * 10 < width * 6 { WALL } else { 0.0 }))
        .collect()
}

struct Occlusion {
    fixture: Fixture,
    early: VisibilityTarget,
    late: VisibilityTarget,
    levels: Vec<occlusion_reference::Level>,
}

impl Occlusion {
    fn new(capacity: u32, depth: &[f32]) -> Self {
        let fixture = Fixture::new(capacity);
        let view =
            pyramid::tests::depth_target(&fixture.device, &fixture.queue, depth, DEPTH_EXTENT);
        let pyramid = pyramid::tests::build(&fixture.device, &fixture.queue, &view, DEPTH_EXTENT);
        let pyramid = pyramid
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let [early, late] = fixture.prepared.occlusion_targets(
            &fixture.device,
            &fixture.view,
            &fixture.instances,
            &fixture.serials,
            &pyramid,
        );
        Self {
            fixture,
            early,
            late,
            levels: occlusion_reference::build(depth, DEPTH_EXTENT),
        }
    }

    fn history(&self) -> std::collections::BTreeSet<u32> {
        let fixture = &self.fixture;
        let words = read(
            &fixture.device,
            &fixture.queue,
            &fixture.prepared.history,
            bits_bytes(fixture.prepared.capacity),
        );
        words
            .iter()
            .enumerate()
            .flat_map(|(index, word)| {
                (0..32)
                    .filter(move |bit| word >> bit & 1 == 1)
                    .map(move |bit| u32::try_from(index).expect("word") * 32 + bit)
            })
            .collect()
    }

    /// One frame: frustum oracle, then early and late. Checks every phase
    /// invariant against the history before and after, and returns the
    /// number of slots in view that the late phase hid.
    fn frame(&mut self, view_proj: Mat4, instances: &[Instance], alpha: f32) -> usize {
        type Set = std::collections::BTreeSet<u32>;
        // Rows past the active prefix keep stale bits until they return; a
        // returning row is then at most drawn early, never hidden.
        let active = u32::try_from(instances.len()).expect("count");
        let prefix = |set: Set| -> Set { set.into_iter().filter(|&slot| slot < active).collect() };
        let before = prefix(self.history());
        self.fixture.upload(view_proj, instances, alpha);
        let framed = self.fixture.compact(&self.fixture.target);
        check(view_proj, instances, alpha, &framed);
        let early = self.fixture.compact(&self.early);
        assert_eq!(prefix(self.history()), before, "early never writes history");
        let late = self.fixture.compact(&self.late);
        let after = prefix(self.history());
        for list in [&early, &late] {
            assert!(list.windows(2).all(|pair| pair[0] < pair[1]), "order");
        }
        let framed: Set = framed.into_iter().collect();
        let early: Set = early.into_iter().collect();
        let late: Set = late.into_iter().collect();
        let expected_early: Set = framed.intersection(&before).copied().collect();
        assert_eq!(
            early, expected_early,
            "early lists exactly last frame's visible in view"
        );
        assert!(after.is_subset(&framed), "history holds only slots in view");
        let expected_late: Set = after.difference(&before).copied().collect();
        assert_eq!(late, expected_late, "late lists exactly the newly visible");
        // Nothing in view is lost: every framed slot is drawn by a phase or
        // hidden by depth, and the hidden decision matches the reference.
        let mut marginal = 0;
        for &slot in &framed {
            let gpu_hidden = !after.contains(&slot);
            assert!(early.contains(&slot) || late.contains(&slot) || gpu_hidden);
            let instance = &instances[slot as usize];
            let decide = |scale: f32| {
                let mut scaled = *instance;
                scaled.current = instance.current * Mat4::from_scale(Vec3::splat(scale));
                let corners =
                    aabb_clip_corners(view_proj, &scaled, alpha).map(|corner| corner.to_array());
                occlusion_reference::occluded(&corners, &self.levels)
            };
            if gpu_hidden != decide(1.0) {
                assert_ne!(
                    decide(1.001),
                    decide(0.999),
                    "slot {slot} differs off the margin"
                );
                marginal += 1;
            }
            // Soundness against the reference proof: hidden only when an
            // inflated box is hidden too.
            if gpu_hidden {
                assert!(
                    decide(0.999),
                    "slot {slot} hidden while its shrunk box is visible"
                );
            }
        }
        assert!(
            marginal * 200 <= framed.len().max(1),
            "{marginal} marginal decisions"
        );
        framed.len() - after.len()
    }
}

#[test]
fn occlusion_phases_never_lose_a_slot_in_view() {
    let depth = wall_depth();
    let mut occlusion = Occlusion::new(4_000, &depth);
    let mut random = Lcg(29);
    let mut instances = scatter(&mut random, 4_000, 40.0);
    let near = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    // First frame: empty history, so early draws nothing and late everything visible.
    let hidden = occlusion.frame(near, &instances, 0.5);
    assert!(hidden > 300, "the wall hides part of the cloud: {hidden}");
    let first = occlusion.history();
    // Same view: early draws last frame's set, late nothing new.
    occlusion.frame(near, &instances, 0.5);
    assert_eq!(
        occlusion.history(),
        first,
        "a still frame keeps its history"
    );
    // A cut, from inside the cloud (corners behind the near plane stay visible).
    let inside = perspective(Vec3::new(2.0, 1.0, 5.0), Vec3::new(-20.0, 0.0, -30.0), 1.2);
    occlusion.frame(inside, &instances, 1.0);
    // Recycled slots and hidden rows: history from the previous objects only
    // moves them between phases.
    for slot in (0..instances.len()).step_by(7) {
        instances[slot] = scatter(&mut random, 1, 40.0)[0];
    }
    occlusion.frame(near, &instances, 0.0);
    // A cancelled frame: early runs, late never does; the next frame is whole.
    occlusion.fixture.upload(near, &instances, 0.0);
    occlusion.fixture.compact(&occlusion.early);
    occlusion.frame(near, &instances, 0.25);
    // A shrinking active prefix never lists rows past the end, and rows that
    // return with new objects are only drawn early on stale history.
    let returning = instances.split_off(1_000);
    occlusion.frame(near, &instances, 0.25);
    instances.extend(returning.iter().map(|_| scatter(&mut random, 1, 40.0)[0]));
    occlusion.frame(inside, &instances, 0.25);
}

#[test]
fn unready_depth_hides_nothing() {
    // A cleared reverse-Z target (all far) is the pyramid before any early draw.
    let depth = vec![0.0; (DEPTH_EXTENT[0] * DEPTH_EXTENT[1]) as usize];
    let mut occlusion = Occlusion::new(2_000, &depth);
    let mut random = Lcg(5);
    let instances = scatter(&mut random, 2_000, 40.0);
    let camera = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    assert_eq!(occlusion.frame(camera, &instances, 0.5), 0);
}
