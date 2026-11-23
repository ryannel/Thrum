//! Shaded panels use the ordinary main view at the pixel's plane crossing as
//! their reference. Stored aerial texels supply rounding, never an integrator.
mod support;
use glam::{Mat4, Vec3, Vec4};
use std::sync::{Arc, Mutex};
use support::*;
use thrum_render::{
    abi::InstanceRecord,
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::{SecondaryViewRequest, ViewAir, ViewClipPlane},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

fn camera(eye: Vec3, pitch: f32, aspect: f32) -> SyntheticView {
    let direction = Vec3::new(0.0, pitch.sin(), -pitch.cos());
    SyntheticView::perspective(
        eye.to_array(),
        (eye + direction * 8192.0).to_array(),
        Vec3::Y.to_array(),
        60.0_f32.to_radians(),
        aspect,
        0.1,
        10000.0,
    )
    .expect("camera")
}
fn light(height: f32, sun: Vec3) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new(sun.to_array(), 100_000.0, [1.0; 3], height, 1).expect("air"),
    )
}
fn pixel_direction(view: &SyntheticView, uv: [f32; 2]) -> Vec3 {
    let eye = Vec3::from_array(view.camera_position());
    let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    let far = inverse * Vec4::new(uv[0] * 2.0 - 1.0, 1.0 - uv[1] * 2.0, 0.0, 1.0);
    (far.truncate() - eye * far.w).normalize()
}
fn instance(slot: u32, center: Vec3, half: Vec3) -> InstanceRecord {
    let mut value = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        value.current[axis][axis] = half[axis];
        value.normal[axis][axis] = half[axis].recip();
        value.current[3][axis] = center[axis];
    }
    value.previous = value.current;
    value
}
fn step(value: f32) -> f32 {
    let bits = half::f16::from_f32(value.abs()).to_bits();
    half::f16::from_bits(bits + 1).to_f32() - half::f16::from_bits(bits).to_f32()
}
fn allowance(
    reference: [f32; 4],
    raw: &[f32],
    stretch: f32,
    uv: [f32; 2],
    radiance: f32,
    reads: u32,
) -> [f32; 3] {
    let mut scatter_q = [0.0_f32; 3];
    let mut trans_q = [0.0_f32; 3];
    for i in 1..=8 {
        let distance = stretch * (i as f32 / 8.0).powi(2);
        let (_, s) = sample(raw, 0, distance, uv, false);
        let (_, t) = sample(raw, 1, distance, uv, false);
        for c in 0..3 {
            scatter_q[c] = scatter_q[c].max(s[c]);
            trans_q[c] = trans_q[c].max(t[c]);
        }
    }
    std::array::from_fn(|c| {
        step(reference[c])
            + reads as f32 * (scatter_q[c] * 100_000.0 / 1024.0 + trans_q[c] * radiance) / 1024.0
    })
}
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "positive clamped distances and UVs address 32x32x33 texels"
)]
fn sample(
    volume: &[f32],
    texture: usize,
    distance: f32,
    uv: [f32; 2],
    empty: bool,
) -> ([f32; 3], [f32; 3]) {
    let low = (((distance / 32000.0).sqrt() * 32.0).floor() as usize).min(31);
    let first = 32000.0 * (low as f32 / 32.0).powi(2);
    let second = 32000.0 * ((low + 1) as f32 / 32.0).powi(2);
    let blend = ((distance - first) / (second - first)).clamp(0.0, 1.0);
    let xy = uv.map(|v| (v * 32.0 - 0.5).clamp(0.0, 31.0));
    let mut value = [0.0; 3];
    let mut quantum = [0.0_f32; 3];
    for (layer, wz) in [(low, 1.0 - blend), (low + 1, blend)] {
        for (y, wy) in [
            (xy[1] as usize, 1.0 - xy[1].fract()),
            ((xy[1] as usize + 1).min(31), xy[1].fract()),
        ] {
            for (x, wx) in [
                (xy[0] as usize, 1.0 - xy[0].fract()),
                ((xy[0] as usize + 1).min(31), xy[0].fract()),
            ] {
                for c in 0..3 {
                    let v =
                        volume[texture * 32 * 32 * 33 * 4 + (layer * 32 * 32 + y * 32 + x) * 4 + c];
                    quantum[c] = quantum[c].max(step(v));
                    if !empty || y < 16 {
                        value[c] += v * wx * wy * wz;
                    }
                }
            }
        }
    }
    (value, quantum)
}
struct PanelFixture<'a> {
    device: &'a wgpu::Device,
    cap: &'a DeviceCapabilities,
    queue: &'a wgpu::Queue,
    sun: Vec3,
    extent: [u32; 2],
    index: usize,
}
impl PanelFixture<'_> {
    async fn observe(
        &self,
        scene: SyntheticScene,
        request: Option<SecondaryViewRequest>,
    ) -> (Host, Arc<Mutex<Control>>) {
        let lighting = light(scene.view.camera_position()[1], self.sun);
        let mut host = host_with_extent(self.cap, self.queue, scene, lighting, self.extent).await;
        let control = if let Some(request) = request {
            let (observer, control) = Observer::new(1);
            control.lock().expect("control").requests[0] = Some(request);
            host.set_scene_contribution(Box::new(observer))
                .expect("view observer");
            control
        } else {
            let (observer, control) = MainObserver::new(self.extent);
            host.set_scene_contribution(Box::new(observer))
                .expect("ordinary observer");
            control
        };
        (host, control)
    }

    fn read(&self, host: &mut Host, control: &Mutex<Control>) -> [f32; 4] {
        // The observer supplies the readback. Skip the host's final colour readback.
        host.render_once().expect("panel draw");
        pixel(
            &output(
                self.device,
                self.queue,
                &control.lock().expect("control").outputs[0],
            ),
            self.index,
        )
        .0
    }

    async fn main_pixel(&self, scene: SyntheticScene) -> [f32; 4] {
        let (mut host, control) = self.observe(scene, None).await;
        self.read(&mut host, &control)
    }

    async fn volume(&self, scene: SyntheticScene) -> Vec<f32> {
        let view = scene.view;
        let lighting = light(view.camera_position()[1], self.sun);
        let mut host = host_with_extent(self.cap, self.queue, scene, lighting, AIR_EXTENT).await;
        let request = SecondaryViewRequest::new(&view, AIR_EXTENT, Some(plane()), ViewAir::Plane)
            .expect("volume request");
        let (probe, control) = AirProbe::new(Some(&request));
        host.set_scene_contribution(Box::new(probe))
            .expect("volume observer");
        host.render_once().expect("raw volume");
        air_output(
            self.device,
            self.queue,
            &control.lock().expect("control").outputs[1],
        )
    }
}

const LUMINOUS_RADIANCE: f32 = 100.0;

fn plane() -> ViewClipPlane {
    ViewClipPlane::new([0.0, 1.0, 0.0, -50.0]).expect("plane")
}

#[expect(
    clippy::print_stderr,
    reason = "retained proof prints measured changes beside the rounding allowance"
)]
fn check_casters(results: &[([f32; 4], [f32; 4])], allowance: [f32; 3]) {
    for (c, bound) in allowance.into_iter().enumerate() {
        let view_change = results[0].0[c] - results[1].0[c];
        let reference_change = results[0].1[c] - results[1].1[c];
        let before_view = (results[0].0[c] - results[2].0[c]).abs();
        let before_reference = (results[0].1[c] - results[2].1[c]).abs();
        eprintln!(
            "caster darkening view={view_change} reference={reference_change}; before-crossing change view={before_view} reference={before_reference}; allowance={bound}"
        );
        assert!(
            view_change > bound && reference_change > bound,
            "caster beyond crossing darkens both"
        );
        assert!(
            before_view <= bound && before_reference <= bound,
            "caster before crossing leaves both alone"
        );
    }
}
#[expect(
    clippy::print_stderr,
    reason = "retained negative controls print their difference and allowance"
)]
fn check_wrong_stretch(
    actual: [f32; 4],
    wrong: [f32; 4],
    before: Option<[f32; 4]>,
    limit: [f32; 3],
) {
    for (c, bound) in limit.into_iter().enumerate() {
        let difference = (actual[c] - wrong[c]).abs();
        eprintln!(
            "wrong whole-camera stretch channel={c}: difference={difference} allowance={bound}"
        );
        assert!(
            difference > bound,
            "whole-camera air must miss: channel {c}"
        );
        if let Some(shaded) = before {
            let change = (wrong[c] - shaded[c]).abs();
            eprintln!(
                "before-only caster darkens the whole-camera reference by {change}; crossing allowance={bound}"
            );
            assert!(
                change > bound,
                "the before-only caster must shade the stretch omitted by the view"
            );
        }
    }
}
#[derive(Clone, Copy, Debug)]
enum Caster {
    None,
    BeyondCrossing,
    BeforeCrossing,
}

#[derive(Clone, Copy)]
enum Controls {
    CrossingOnly,
    WholeCamera,
    Casters,
    GrazingHorizon,
}

struct Crossing {
    eye: Vec3,
    point: Vec3,
    direction: Vec3,
    distance: f32,
    extent: [u32; 2],
    index: usize,
    uv: [f32; 2],
    view: SyntheticView,
    reference: SyntheticView,
    request: SecondaryViewRequest,
}
impl Crossing {
    fn new(below: f32, distance: f32, controls: Controls) -> Self {
        // Retain the old centre ray or grazing row and the projections that
        // define the aerial columns. Only the observed column needs pixels.
        let (height, row, aspect, pitch) = if matches!(controls, Controls::GrazingHorizon) {
            (331, 164_u32, 65.0 / 331.0, 0.0)
        } else {
            (1, 0, 1.0, 10.0_f32.to_radians())
        };
        let extent = [1, height];
        let uv = [0.5, (row as f32 + 0.5) / height as f32];
        let eye = Vec3::new(0.0, 50.0 - below, 0.0);
        let view = camera(eye, pitch, aspect);
        let direction = pixel_direction(&view, uv);
        let point = eye + direction * (below / direction.y);
        let reference = camera(point, pitch, aspect);
        let request = SecondaryViewRequest::new(&view, extent, Some(plane()), ViewAir::Plane)
            .expect("request")
            .with_shadows(0, 128.0)
            .expect("profile");
        Self {
            eye,
            point,
            direction,
            distance,
            extent,
            index: row as usize,
            uv,
            view,
            reference,
            request,
        }
    }

    fn scene(&self) -> SyntheticScene {
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = self.view;
        scene.instances[0] = instance(
            0,
            self.point + self.direction * self.distance,
            Vec3::new(self.distance * 0.1, self.distance * 0.1, 0.01),
        );
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].surface[2] = 1.0;
        scene.materials[1] = scene.materials[0];
        scene.materials[1].emissive_cutoff =
            [LUMINOUS_RADIANCE, LUMINOUS_RADIANCE, LUMINOUS_RADIANCE, 0.0];
        scene
    }

    fn add_caster(&self, scene: &mut SyntheticScene, sun: Vec3, caster: Caster) {
        // The long stretch lets the 64 m shadow-height cells lie wholly on
        // either side of the crossing.
        let (midpoint, half) = match caster {
            Caster::None => return,
            Caster::BeyondCrossing => (
                self.point + self.direction * 1500.0,
                Vec3::new(128.0, 80.0, 200.0),
            ),
            Caster::BeforeCrossing => (
                self.eye + self.direction * 126.0,
                Vec3::new(64.0, 16.0, 40.0),
            ),
        };
        scene.sun_shadows = true;
        scene
            .instances
            .push(instance(1, midpoint + sun * 512.0, half));
    }

    fn allowance(&self, reference: [f32; 4], raw: &[f32], radiance: f32, reads: u32) -> [f32; 3] {
        allowance(reference, raw, self.stretch(), self.uv, radiance, reads)
    }

    fn stretch(&self) -> f32 {
        // The front surface is 1 cm before the panel centre in Z.
        self.distance - 0.01 / self.direction.z.abs()
    }
}

async fn check_panels(
    fixture: &PanelFixture<'_>,
    crossing: &Crossing,
    raw: &[f32],
    caster: Caster,
) -> ([f32; 4], [f32; 4]) {
    let mut scene = crossing.scene();
    crossing.add_caster(&mut scene, fixture.sun, caster);
    let (mut view_host, view_control) =
        fixture.observe(scene.clone(), Some(crossing.request)).await;
    scene.view = crossing.reference;
    let (mut main_host, main_control) = fixture.observe(scene, None).await;

    let actual = fixture.read(&mut view_host, &view_control);
    let reference = fixture.read(&mut main_host, &main_control);
    check_match(fixture, crossing, raw, caster, 0.0, actual, reference);

    // Keep the same geometry and pipelines and resubmit with the emissive material.
    // A radiance response on each path witnesses the switch in the readback.
    view_host.source_mut().instances[0].1 = 1;
    main_host.source_mut().instances[0].1 = 1;
    let luminous = fixture.read(&mut view_host, &view_control);
    let luminous_reference = fixture.read(&mut main_host, &main_control);
    check_match(
        fixture,
        crossing,
        raw,
        caster,
        LUMINOUS_RADIANCE,
        luminous,
        luminous_reference,
    );
    for c in 0..3 {
        assert!(
            luminous[c] > actual[c] && luminous_reference[c] > reference[c],
            "both panels respond to radiance in channel {c}"
        );
    }
    (actual, reference)
}

#[expect(
    clippy::print_stderr,
    reason = "the proof reports differences and stored-step allowances"
)]
fn check_match(
    fixture: &PanelFixture<'_>,
    crossing: &Crossing,
    raw: &[f32],
    caster: Caster,
    radiance: f32,
    actual: [f32; 4],
    reference: [f32; 4],
) {
    assert_eq!(actual[3], 1.0, "covered panel");
    let reads = if matches!(caster, Caster::None) {
        4
    } else {
        18
    };
    let limit = crossing.allowance(reference, raw, radiance, reads);
    for (c, bound) in limit.into_iter().enumerate() {
        let difference = (actual[c] - reference[c]).abs();
        eprintln!(
            "eye={} distance={} sun={} radiance={radiance} caster={caster:?} channel={c}: difference={difference} allowance={bound}",
            crossing.eye.y, crossing.distance, fixture.sun
        );
        assert!(difference <= bound, "ordinary crossing reference");
    }
}

#[expect(
    clippy::print_stderr,
    reason = "the proof reports the grazing horizon negative control"
)]
fn check_horizon(crossing: &Crossing, raw: &[f32], actual: [f32; 4], limit: [f32; 3]) {
    let (empty, _) = sample(raw, 0, crossing.stretch(), crossing.uv, true);
    for (c, bound) in limit.into_iter().enumerate() {
        let wrong = empty[c] * 100_000.0 / (1024.0 * 1024.0);
        let difference = (actual[c] - wrong).abs();
        eprintln!(
            "emptied never-crossing row channel={c}: difference={difference} allowance={bound}"
        );
        assert!(difference > bound, "empty never-crossing row must miss");
    }
}

async fn check_crossing(fixture: &PanelFixture<'_>, crossing: &Crossing, controls: Controls) {
    // The plane's aerial product depends on the view and Sun, so both surface
    // radiances and all caster comparisons use the same stored-step evidence.
    let raw = fixture.volume(crossing.scene()).await;
    let (actual, reference) = check_panels(fixture, crossing, &raw, Caster::None).await;
    let before_camera = if matches!(controls, Controls::Casters) {
        let beyond = check_panels(fixture, crossing, &raw, Caster::BeyondCrossing).await;
        let before = check_panels(fixture, crossing, &raw, Caster::BeforeCrossing).await;
        check_casters(
            &[(actual, reference), beyond, before],
            crossing.allowance(reference, &raw, 0.0, 18),
        );

        let mut before_scene = crossing.scene();
        crossing.add_caster(&mut before_scene, fixture.sun, Caster::BeforeCrossing);
        Some(fixture.main_pixel(before_scene).await)
    } else {
        None
    };
    let limit = crossing.allowance(reference, &raw, 0.0, 4);
    if !matches!(controls, Controls::CrossingOnly) {
        let wrong = fixture.main_pixel(crossing.scene()).await;
        check_wrong_stretch(actual, wrong, before_camera, limit);
    }
    if matches!(controls, Controls::GrazingHorizon) {
        check_horizon(crossing, &raw, actual, limit);
    }
}

fn panels(below: f32, distance: f32, controls: Controls) {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "plane shaded air",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let crossing = Crossing::new(below, distance, controls);
    let cap = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        for sun in [
            Vec3::new(1.0, 1.0, 0.0).normalize(),
            Vec3::new(
                0.0,
                10.0_f32.to_radians().sin(),
                10.0_f32.to_radians().cos(),
            ),
        ] {
            let fixture = PanelFixture {
                device: &device,
                cap: &cap,
                queue: &queue,
                sun,
                extent: crossing.extent,
                index: crossing.index,
            };
            check_crossing(&fixture, &crossing, controls).await;
        }
    });
}

#[test]
fn near_panel_two_metres_below_matches_crossing() {
    panels(2.0, 20.0, Controls::WholeCamera);
}

#[test]
fn far_panel_two_metres_below_matches_crossing() {
    panels(2.0, 3000.0, Controls::CrossingOnly);
}

#[test]
fn near_panel_forty_metres_below_matches_crossing() {
    panels(40.0, 20.0, Controls::WholeCamera);
}

#[test]
fn far_panel_forty_metres_below_matches_crossing_with_casters() {
    panels(40.0, 3000.0, Controls::Casters);
}

#[test]
fn grazing_panel_matches_crossing_through_never_crossing_rows() {
    panels(2.0, 1000.0, Controls::GrazingHorizon);
}
