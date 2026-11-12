use super::*;
use glam::{Mat4, Vec3, Vec4};

impl VisibilityPrepared {
    /// Allocate one frustum target's outputs.
    pub(crate) fn target(
        &self,
        device: &wgpu::Device,
        view: &wgpu::Buffer,
        instances: &wgpu::Buffer,
        serials: &wgpu::Buffer,
    ) -> VisibilityTarget {
        self.member_target(device, (view, instances, serials), None)
    }
}

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

/// The final cull's clip magnitude for a unit-cube instance: |row| times the
/// largest |position| the vertex shader or the corners can form.
fn clip_magnitude(view_proj: Mat4, instance: &Instance) -> [f32; 4] {
    let spread = instance.current.x_axis.truncate().abs()
        + instance.current.y_axis.truncate().abs()
        + instance.current.z_axis.truncate().abs();
    let world = spread + instance.previous.abs() + instance.current.w_axis.truncate().abs();
    [0, 1, 2, 3].map(|row| {
        let row = view_proj.row(row).abs();
        row.truncate().dot(world) + row.w
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
        let prepared = VisibilityPrepared::new(
            &crate::runtime::DeviceSite::untracked(wgpu::Device::clone(&device)),
            &queue,
            KEY,
            capacity,
            &[CUBE],
            capacity,
            CullMode::Occlusion,
        );
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
        self.compact_as(target, 0)
    }

    /// Compact with the target's phase record, writing history side `write`.
    fn compact_as(&self, target: &VisibilityTarget, write: usize) -> Vec<u32> {
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
                .encode_phase(
                    &mut encoder,
                    target,
                    write,
                    (0, KEY.device_generation),
                    pass,
                    None,
                )
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
    prepass: VisibilityTarget,
    last: VisibilityTarget,
    levels: Vec<occlusion_reference::Level>,
    /// The history side this frame writes; `commit` flips it, as a display
    /// host's committed frame flips the transform history.
    write: usize,
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
        let [prepass, last] = fixture.prepared.occlusion_targets(
            &fixture.device,
            &fixture.view,
            &fixture.instances,
            &fixture.serials,
            &pyramid,
            None,
        );
        Self {
            fixture,
            prepass,
            last,
            levels: occlusion_reference::build(depth, DEPTH_EXTENT),
            write: 0,
        }
    }

    /// The committed history, which the next frame reads.
    fn history(&self) -> std::collections::BTreeSet<u32> {
        self.side(1 - self.write)
    }

    fn side(&self, side: usize) -> std::collections::BTreeSet<u32> {
        let fixture = &self.fixture;
        let words = read(
            &fixture.device,
            &fixture.queue,
            &fixture.prepared.history[side],
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

    /// One frame: frustum oracle, then the prepass list and the final cull.
    /// Checks each list against the history before and after, and returns
    /// the number of slots in view that the final cull hid.
    fn frame(&mut self, view_proj: Mat4, instances: &[Instance], alpha: f32) -> usize {
        type Set = std::collections::BTreeSet<u32>;
        // The side this frame reads may hold rows past a shrunk prefix from
        // an older, larger frame; they are not read until they return.
        let active = u32::try_from(instances.len()).expect("count");
        let prefix = |set: Set| -> Set { set.into_iter().filter(|&slot| slot < active).collect() };
        let before = prefix(self.history());
        self.fixture.upload(view_proj, instances, alpha);
        let framed = self.fixture.compact(&self.fixture.target);
        check(view_proj, instances, alpha, &framed);
        let prepass = self.prepass();
        assert_eq!(
            prefix(self.history()),
            before,
            "the prepass list never writes history"
        );
        let last = self.last();
        assert_eq!(
            prefix(self.history()),
            before,
            "the final cull writes the other side"
        );
        self.commit();
        assert!(
            self.history().iter().all(|&slot| slot < active),
            "no history word past the prefix survives publication"
        );
        let after = prefix(self.history());
        for list in [&prepass, &last] {
            assert!(list.windows(2).all(|pair| pair[0] < pair[1]), "order");
        }
        let framed: Set = framed.into_iter().collect();
        let prepass: Set = prepass.into_iter().collect();
        let last: Set = last.into_iter().collect();
        let expected_prepass: Set = framed.intersection(&before).copied().collect();
        assert_eq!(
            prepass, expected_prepass,
            "the prepass lists exactly last frame's visible in view"
        );
        assert_eq!(last, after, "the final list is the next history");
        assert!(after.is_subset(&framed), "history holds only slots in view");
        // Nothing in view is lost: every framed slot is drawn by the final
        // list or hidden by depth, and the hidden decision matches the
        // reference.
        let mut marginal = 0;
        for &slot in &framed {
            let gpu_hidden = !last.contains(&slot);
            let instance = &instances[slot as usize];
            let decide = |scale: f32| {
                let mut scaled = *instance;
                scaled.current = instance.current * Mat4::from_scale(Vec3::splat(scale));
                let corners =
                    aabb_clip_corners(view_proj, &scaled, alpha).map(|corner| corner.to_array());
                let magnitude = clip_magnitude(view_proj, &scaled);
                occlusion_reference::occluded(&corners, &self.levels, magnitude)
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

    fn prepass(&self) -> Vec<u32> {
        self.fixture.compact_as(&self.prepass, self.write)
    }

    fn last(&self) -> Vec<u32> {
        self.fixture.compact_as(&self.last, self.write)
    }

    fn commit(&mut self) {
        self.write = 1 - self.write;
    }
}

/// The final cull's clip magnitude, on the real path: one box at the field's
/// far extent, in front of a constant depth placed half its derived slack
/// behind it (kept), then twice its slack (hidden). Here the w term dominates
/// the slack, so a shader whose magnitude drops the translation, or reads the
/// z row for w, hides the first box; one that inflates it keeps the second.
#[test]
fn the_final_cull_uses_the_full_clip_magnitude() {
    let origin = Vec3::new(-400.37, 1.13, 400.71);
    let instance = Instance {
        current: Mat4::from_scale_rotation_translation(
            Vec3::splat(0.5),
            glam::Quat::from_rotation_y(0.3),
            origin,
        ),
        previous: origin,
        hidden: false,
    };
    let view_proj = perspective(origin + Vec3::new(1.0, 0.5, 6.0), origin, 1.0);
    let corners = aabb_clip_corners(view_proj, &instance, 1.0);
    let nearest = corners
        .iter()
        .map(|c| c.z / c.w)
        .fold(f32::NEG_INFINITY, f32::max);
    let w_min = corners.iter().map(|c| c.w).fold(f32::INFINITY, f32::min);
    let magnitude = clip_magnitude(view_proj, &instance);
    let slack = |magnitude| occlusion_reference::slack(nearest, w_min, magnitude).expect("w > 0");
    let full = slack(magnitude);
    // The test only fails those shaders if their slack leaves the band.
    let world = instance.current.w_axis.truncate().abs() + instance.previous.abs();
    let w_row = view_proj.row(3).abs();
    let mut without_translation = magnitude;
    without_translation[3] -= w_row.truncate().dot(world);
    assert!(slack(without_translation) * 2.0 < full, "{magnitude:?}");
    assert!(
        slack([magnitude[0], magnitude[1], magnitude[2], magnitude[2]]) * 2.0 < full,
        "{magnitude:?}"
    );
    for (scale, kept) in [(0.5, true), (2.0, false)] {
        let wall = nearest + scale * full;
        let depth = vec![wall; (DEPTH_EXTENT[0] * DEPTH_EXTENT[1]) as usize];
        let mut occlusion = Occlusion::new(1, &depth);
        occlusion.frame(view_proj, &[instance], 1.0);
        assert_eq!(
            occlusion.history().contains(&0),
            kept,
            "depth {wall} at {scale} slack ({full}) behind nearest {nearest}"
        );
    }
}

/// The footprint widening on the real path (design, "Footprint"): a small
/// box close to the camera at the field's far extent, where the x / y clip
/// terms widen its read rectangle by several level-0 texels. A wall covers
/// it; open sky starts just past the block its unwidened rectangle reads (kept),
/// then just past the block the widened rectangle reads (hidden). A shader
/// whose magnitude drops the x / y rows hides the first; one that inflates
/// the widening keeps the second.
#[test]
fn the_final_cull_widens_the_footprint_by_the_clip_bound() {
    let origin = Vec3::new(-400.37, 1.13, 400.71);
    let instance = Instance {
        current: Mat4::from_scale_rotation_translation(
            Vec3::splat(0.003),
            glam::Quat::from_rotation_y(0.3),
            origin,
        ),
        previous: origin,
        hidden: false,
    };
    let view_proj = perspective(
        origin + Vec3::new(0.004, 0.002, 0.12),
        origin - Vec3::new(0.0025, 0.0, 0.0),
        1.0,
    );
    let corners = aabb_clip_corners(view_proj, &instance, 1.0);
    let magnitude = clip_magnitude(view_proj, &instance);
    let size = DEPTH_EXTENT.map(occlusion_reference::previous_pow2);
    // The read rectangle in level-0 texels, with and without the widening,
    // and the largest widening on x, as the reference forms them.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "texels of small extents"
    )]
    let rectangle = |widen: bool| {
        let mut low = [u32::MAX; 2];
        let mut high = [0; 2];
        let mut most = 0.0_f32;
        for c in &corners {
            let ndc = [c.x / c.w, c.y / c.w];
            let uv = [0.5 + 0.5 * ndc[0], 0.5 - 0.5 * ndc[1]];
            let w_low = c.w - 2.0 * occlusion_reference::CLIP_GAMMA * magnitude[3];
            for axis in 0..2 {
                let extent = size[axis] as f32;
                let widening = occlusion_reference::footprint_widening(
                    ndc[axis],
                    w_low,
                    magnitude[axis],
                    magnitude[3],
                    extent,
                );
                if axis == 0 {
                    most = most.max(widening * extent);
                }
                let widening = if widen { widening } else { 0.0 };
                let texel = |uv: f32| (uv * extent).floor().clamp(0.0, extent - 1.0) as u32;
                low[axis] = low[axis].min(texel(uv[axis] - widening));
                high[axis] = high[axis].max(texel(uv[axis] + widening));
            }
        }
        // The first level-0 texel past the block the test reads on x.
        let level = (0..16)
            .find(|&level| (0..2).all(|axis| (high[axis] >> level) - (low[axis] >> level) <= 1))
            .expect("a level fits");
        (((high[0] >> level) + 1) << level, high[0], most)
    };
    let (unwidened_end, unwidened_high, widening) = rectangle(false);
    let (widened_end, _, _) = rectangle(true);
    // Sky at the end of the unwidened read is seen only through the widening.
    assert!(
        f64::from(unwidened_end) <= f64::from(unwidened_high) + f64::from(widening),
        "the widening must reach past the unwidened read: ends {unwidened_end}, \
         high {unwidened_high}, widening {widening}"
    );
    let [width, height] = DEPTH_EXTENT;
    for (sky, kept) in [(unwidened_end, true), (widened_end, false)] {
        // Pixels from the first one texel `sky` covers alone are open.
        let open = (sky * width).div_ceil(size[0]);
        assert!(open < width, "the sky starts on screen");
        let depth: Vec<f32> = (0..height)
            .flat_map(|_| (0..width).map(move |x| if x < open { 1.0 } else { 0.0 }))
            .collect();
        let mut occlusion = Occlusion::new(1, &depth);
        occlusion.frame(view_proj, &[instance], 1.0);
        assert_eq!(
            occlusion.history().contains(&0),
            kept,
            "sky from texel {sky}, unwidened high {unwidened_high}, widening {widening}"
        );
    }
}

#[test]
fn occlusion_phases_never_lose_a_slot_in_view() {
    let depth = wall_depth();
    let mut occlusion = Occlusion::new(4_000, &depth);
    let mut random = Lcg(29);
    let mut instances = scatter(&mut random, 4_000, 40.0);
    let near = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    // First frame: empty history, so the prepass draws nothing and the final
    // list holds everything visible.
    let hidden = occlusion.frame(near, &instances, 0.5);
    assert!(hidden > 300, "the wall hides part of the cloud: {hidden}");
    let first = occlusion.history();
    // Same view: the prepass draws last frame's set, and the final list is
    // the same set.
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
    // changes which slots the prepass draws.
    for slot in (0..instances.len()).step_by(7) {
        instances[slot] = scatter(&mut random, 1, 40.0)[0];
    }
    occlusion.frame(near, &instances, 0.0);
    // A cancelled frame: the prepass list runs, the final cull never does;
    // the next frame is whole.
    occlusion.fixture.upload(near, &instances, 0.0);
    occlusion.prepass();
    occlusion.frame(near, &instances, 0.25);
    // A shrinking active prefix never lists rows past the end, and rows that
    // return with new objects only reach the prepass on stale history.
    let returning = instances.split_off(1_000);
    occlusion.frame(near, &instances, 0.25);
    instances.extend(returning.iter().map(|_| scatter(&mut random, 1, 40.0)[0]));
    occlusion.frame(inside, &instances, 0.25);
}

#[test]
fn a_capture_between_displays_keeps_the_displayed_history() {
    let depth = wall_depth();
    let mut occlusion = Occlusion::new(3_000, &depth);
    let mut random = Lcg(23);
    let instances = scatter(&mut random, 3_000, 40.0);
    let display = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    occlusion.frame(display, &instances, 0.5);
    occlusion.frame(display, &instances, 0.5);
    let displayed = occlusion.history();
    // A capture from another camera runs both lists on the uncommitted side
    // and never commits; both list only slots in view.
    let capture = perspective(Vec3::new(30.0, 10.0, 45.0), Vec3::new(-5.0, 0.0, 0.0), 0.9);
    occlusion.fixture.upload(capture, &instances, 1.0);
    let framed = occlusion.fixture.compact(&occlusion.fixture.target);
    let prepass = occlusion.prepass();
    let last = occlusion.last();
    assert!(
        prepass
            .iter()
            .chain(&last)
            .all(|slot| framed.contains(slot)),
        "only slots in view"
    );
    assert_ne!(
        occlusion.side(occlusion.write),
        displayed,
        "the capture wrote its own visibility"
    );
    assert_eq!(occlusion.history(), displayed, "displayed history kept");
    // The next display frame reads the displayed history: a still view, so
    // the prepass replays it.
    occlusion.fixture.upload(display, &instances, 0.5);
    let prepass = occlusion.prepass();
    assert_eq!(
        prepass
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        displayed,
        "the prepass replays the displayed set"
    );
    occlusion.frame(display, &instances, 0.5);
    assert_eq!(occlusion.history(), displayed);
}

/// Slots at or past `bound` in the committed history.
fn past(occlusion: &Occlusion, bound: u32) -> Vec<u32> {
    occlusion
        .history()
        .into_iter()
        .filter(|&slot| slot >= bound)
        .collect()
}

#[test]
fn history_words_never_outlive_a_shrink_or_capture() {
    let depth = wall_depth();
    let mut occlusion = Occlusion::new(600, &depth);
    let mut random = Lcg(41);
    let instances = scatter(&mut random, 600, 40.0);
    let short = &instances[..200];
    let display = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    let capture = perspective(Vec3::new(30.0, 10.0, 45.0), Vec3::new(-5.0, 0.0, 0.0), 0.9);
    // Two full displays leave visible words above slot 255 on both sides.
    occlusion.frame(display, &instances, 0.5);
    occlusion.frame(display, &instances, 0.5);
    assert!(!past(&occlusion, 256).is_empty(), "high words to outlive");
    // A full capture writes high words on the unpublished side; a short
    // display (one workgroup) then writes and publishes that same side.
    occlusion.fixture.upload(capture, &instances, 1.0);
    occlusion.prepass();
    occlusion.last();
    occlusion.frame(display, short, 0.5);
    assert!(
        past(&occlusion, 200).is_empty(),
        "capture words outlived a shrink"
    );
    // A regular short frame publishes the other side, last written full.
    occlusion.frame(display, short, 0.5);
    assert!(
        past(&occlusion, 200).is_empty(),
        "display words outlived a shrink"
    );
    // Regrowth reads only what the short frames published: the returning
    // rows never reach the prepass from stale words.
    occlusion.fixture.upload(display, &instances, 0.5);
    assert!(occlusion.prepass().iter().all(|&slot| slot < 200));
    occlusion.frame(display, &instances, 0.5);
    occlusion.frame(display, &instances, 0.5);
    assert!(!past(&occlusion, 256).is_empty(), "high words again");
}

#[test]
fn unready_depth_hides_nothing() {
    // A cleared reverse-Z target (all far) is the pyramid of an empty prepass.
    let depth = vec![0.0; (DEPTH_EXTENT[0] * DEPTH_EXTENT[1]) as usize];
    let mut occlusion = Occlusion::new(2_000, &depth);
    let mut random = Lcg(5);
    let instances = scatter(&mut random, 2_000, 40.0);
    let camera = perspective(Vec3::new(0.0, 0.0, 60.0), Vec3::ZERO, 1.0);
    assert_eq!(occlusion.frame(camera, &instances, 0.5), 0);
}

/// Member-indexed buckets (M3b, experimental): each bucket's segment of the
/// member list compacts into its own aligned range of the visible list with
/// its own indirect arguments, and lists exactly what the flat frustum list
/// holds for its members, in the same order.
#[test]
fn member_indexed_buckets_match_the_flat_list() {
    let count = 3_000_u32;
    let mut fixture = Fixture::new(count);
    let mut random = Lcg(29);
    let instances = scatter(&mut random, count as usize, 60.0);
    let camera = perspective(Vec3::new(0.0, 10.0, 90.0), Vec3::ZERO, 1.0);
    let flat = fixture.run(camera, &instances, 0.5);
    assert!(flat.len() > 500 && flat.len() < 2_900, "{}", flat.len());
    // Bucket 0 holds most slots (several workgroups), bucket 1 every third,
    // bucket 2 none; every seventh slot is in no bucket.
    let bucket_of = |slot: u32| -> Option<usize> {
        if slot.is_multiple_of(7) {
            None
        } else if slot.is_multiple_of(3) {
            Some(1)
        } else {
            Some(0)
        }
    };
    let shifted = MeshRecord {
        first_index: 36,
        index_count: 72,
        base_vertex: 24,
        ..CUBE
    };
    let bucket_mesh = [0_u32, 1, 0];
    let mut lists = vec![Vec::new(); 3];
    for slot in 0..count {
        if let Some(bucket) = bucket_of(slot) {
            lists[bucket].push(slot);
        }
    }
    let mut members = Vec::new();
    let mut segments = Vec::new();
    for (list, mesh) in lists.iter().zip(bucket_mesh) {
        let start = u32::try_from(members.len()).expect("small");
        let groups = u32::try_from(list.len())
            .expect("small")
            .max(1)
            .div_ceil(WORKGROUP);
        members.extend(list);
        members.resize((start + groups * WORKGROUP) as usize, u32::MAX);
        segments.push([start / WORKGROUP, groups, mesh, 0]);
    }
    let total_groups = u32::try_from(members.len()).expect("small") / WORKGROUP;
    // Uniform bucket table: count, mask base, padding, then 64 segments.
    let mut table = vec![3, total_groups, 0, 0];
    table.extend(segments.concat());
    table.resize(4 + 64 * 4, 0);
    let device = &fixture.device;
    let words =
        |values: &[u32]| -> Vec<u8> { values.iter().flat_map(|v| v.to_le_bytes()).collect() };
    let filled = |label: &str, bytes: &[u8]| {
        let buffer = storage(device, label, bytes.len() as u64);
        fixture.queue.write_buffer(&buffer, 0, bytes);
        buffer
    };
    let member_buffer = filled("test members", &words(&members));
    let segment_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test bucket table"),
        size: table.len() as u64 * 4,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    fixture
        .queue
        .write_buffer(&segment_buffer, 0, &words(&table));
    let mesh_buffer = filled("test meshes", &[CUBE.pack(), shifted.pack()].concat());
    // Counts, then the member path's masks.
    let group_offsets = storage(
        device,
        "test group offsets",
        u64::from(total_groups * (1 + WORDS)) * 4,
    );
    let visible = storage(device, "test visible", members.len() as u64 * 4);
    let args = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test bucket args"),
        size: 3 * ARGS_BYTES,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let (layout, [count_members, offsets_members, scatter_members]) = member_pipelines(device);
    let prepared = &fixture.prepared;
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("test member visibility"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &fixture.view,
                    offset: 0,
                    size: wgpu::BufferSize::new(VIEW_BYTES),
                }),
            },
            binding(1, &fixture.instances),
            binding(2, &fixture.serials),
            binding(3, &mesh_buffer),
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &prepared.params,
                    offset: 0,
                    size: wgpu::BufferSize::new(PARAMS_LEN as u64),
                }),
            },
            binding(5, &group_offsets),
            binding(6, &visible),
            binding(7, &args),
            binding(9, &prepared.history[1]),
            wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(&prepared.idle_pyramid),
            },
            binding(11, &prepared.history[0]),
            binding(12, &member_buffer),
            binding(13, &segment_buffer),
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        // Phase 0 (frustum) parameters, at offset 0.
        pass.set_bind_group(0, &group, &[0, 0]);
        pass.set_pipeline(&count_members);
        pass.dispatch_workgroups(total_groups, 1, 1);
        pass.set_pipeline(&offsets_members);
        pass.dispatch_workgroups(3, 1, 1);
        pass.set_pipeline(&scatter_members);
        pass.dispatch_workgroups(total_groups, 1, 1);
    }
    fixture.queue.submit([encoder.finish()]);
    let args = read(device, &fixture.queue, &args, 3 * ARGS_BYTES);
    let listed = read(device, &fixture.queue, &visible, members.len() as u64 * 4);
    let meshes = [CUBE, shifted];
    for (bucket, segment) in segments.iter().enumerate() {
        let expected: Vec<u32> = flat
            .iter()
            .copied()
            .filter(|&slot| bucket_of(slot) == Some(bucket))
            .collect();
        let record = &args[bucket * 5..bucket * 5 + 5];
        let mesh = meshes[segment[2] as usize];
        assert_eq!(
            record,
            [
                mesh.index_count,
                u32::try_from(expected.len()).expect("small"),
                mesh.first_index,
                mesh.base_vertex.cast_unsigned(),
                0
            ],
            "bucket {bucket} arguments"
        );
        let start = (segment[0] * WORKGROUP) as usize;
        assert_eq!(
            listed[start..start + expected.len()],
            expected,
            "bucket {bucket} list"
        );
    }
    assert_eq!(args[11], 0, "the empty bucket draws nothing");
}

/// The member-indexed entry points under a layout of at most 8 storage
/// buffers (binding 8 unbound, binding 13 a uniform bucket table).
fn member_pipelines(device: &wgpu::Device) -> (wgpu::BindGroupLayout, [wgpu::ComputePipeline; 3]) {
    let entry = |binding, ty, dynamic| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: dynamic,
            min_binding_size: None,
        },
        count: None,
    };
    let read_only = wgpu::BufferBindingType::Storage { read_only: true };
    let write = wgpu::BufferBindingType::Storage { read_only: false };
    let uniform = wgpu::BufferBindingType::Uniform;
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("test member visibility"),
        entries: &[
            entry(0, uniform, true),
            entry(1, read_only, false),
            entry(2, uniform, false),
            entry(3, read_only, false),
            entry(4, uniform, true),
            entry(5, write, false),
            entry(6, write, false),
            entry(7, write, false),
            entry(9, read_only, false),
            entry(11, write, false),
            entry(12, read_only, false),
            entry(13, uniform, false),
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("test member visibility"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("test member visibility"),
        source: wgpu::ShaderSource::Wgsl(
            [include_str!("../visibility.wgsl"), pyramid::OCCLUSION_WGSL]
                .concat()
                .into(),
        ),
    });
    let pipeline = |entry: &str| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(entry),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    let pipelines = ["count_members", "offsets_members", "scatter_members"].map(pipeline);
    (layout, pipelines)
}
