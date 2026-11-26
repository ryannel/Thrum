//! A fixed optical path in the ordinary scene host, with scene-linear readback.
#![allow(
    dead_code,
    reason = "the three focused Metal test binaries use different arms of this fixture"
)]
use glam::Vec3;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_app::sky::StillWorld;
use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    atmosphere::{Aerosol, AtmosphereFrame, SunOpticalDepth},
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, FrameConfig, FrameError, FrameHost, FrameSample, SceneFrame, SceneSource,
        SurfaceOutput,
    },
    synthetic::{CaptureSource, FrameLighting, OUTPUT_FORMAT, SyntheticScene, SyntheticView},
};
use thrum_water::{BodyBounds, Optics, OpticsBodies, Profile, Water};
use thrum_water_model::{Bed, Body, BodyKind, Map, Stage};

pub(crate) const EXTENT: [u32; 2] = [192, 108];

pub(crate) struct Fixed {
    pub(crate) view: SyntheticView,
    pub(crate) light: FrameLighting,
    objects: Vec<([f64; 3], u32)>,
}

impl SceneSource<SourceTick> for Fixed {
    fn extract(&self, into: &mut RenderExtractor, tick: &SourceTick) -> ExtractReport {
        into.extract(*tick, [0.0; 3], |sink| {
            for (id, &(position, material)) in (0_u32..).zip(&self.objects) {
                sink.push(
                    id,
                    u64::from(id) + 1,
                    u128::from(id) + 1,
                    position,
                    u64::from(material),
                );
            }
        })
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.view,
            lighting: self.light,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Panel {
    pub(crate) centre: [f32; 3],
    pub(crate) half_size: [f32; 3],
    pub(crate) radiant: bool,
}

impl Panel {
    pub(crate) fn bounds(self) -> BodyBounds {
        let point = Vec3::from_array(self.centre);
        let half = Vec3::from_array(self.half_size) + 0.035;
        BodyBounds::new((point - half).to_array(), (point + half).to_array()).expect("panel bounds")
    }

    fn instance(self, id: u32) -> InstanceRecord {
        let mut item = InstanceRecord::identity(id, u32::from(self.radiant));
        for (axis, half) in self.half_size.into_iter().enumerate() {
            item.current[axis][axis] = half;
            item.normal[axis][axis] = half.recip();
            item.current[3][axis] = self.centre[axis];
        }
        item.previous = item.current;
        item
    }
}

pub(crate) struct Rig {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) host: FrameHost<StillWorld, Fixed>,
    pub(crate) witness: Option<wgpu::Texture>,
    output: wgpu::TextureView,
    submitted: u32,
    extent: [u32; 2],
}

impl Rig {
    pub(crate) fn new(
        gpu: &thrum_test_support::Gpu,
        view: &SyntheticView,
        light: FrameLighting,
        panels: &[Panel],
        cloud: Option<SunOpticalDepth>,
    ) -> Self {
        Self::with_extent(gpu, view, light, panels, cloud, EXTENT)
    }

    pub(crate) fn with_extent(
        gpu: &thrum_test_support::Gpu,
        view: &SyntheticView,
        light: FrameLighting,
        panels: &[Panel],
        cloud: Option<SunOpticalDepth>,
        extent: [u32; 2],
    ) -> Self {
        let (device, queue, caps) = crate::support::device(&gpu.fresh_adapter());
        let mut scene =
            SyntheticScene::grid(u32::try_from(panels.len().max(1)).expect("small fixture"))
                .expect("optical fixture");
        scene.view = *view;
        scene.sun_optical_depth = cloud;
        scene.materials = vec![
            MaterialRecord {
                base_rgba: [0.09, 0.09, 0.09, 1.0],
                surface: [1.0, 1.0, 0.0, 1.0],
                ..MaterialRecord::default()
            };
            2
        ];
        // An added isotropic material radiance lets paired frames measure
        // transport with precisely the same geometry and received illumination.
        scene.materials[1].emissive_cutoff = [24_000.0, 24_000.0, 24_000.0, 0.0];
        for (index, (record, &panel)) in (0_u32..).zip(scene.instances.iter_mut().zip(panels)) {
            *record = panel.instance(index);
        }
        if panels.is_empty() {
            scene.instances[0].deactivate();
        }
        let source = Fixed {
            view: *view,
            light,
            objects: panels
                .iter()
                .map(|p| (p.centre.map(f64::from), u32::from(p.radiant)))
                .collect(),
        };
        let host = pollster::block_on(FrameHost::new(
            &caps,
            &queue,
            FrameConfig {
                render_extent: extent,
                surface: Some(SurfaceProfile {
                    format: OUTPUT_FORMAT,
                    extent,
                }),
            },
            StillWorld::default(),
            source,
            scene,
            1.0,
        ))
        .expect("fixed optical host");
        let output = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("optics measurement surface"),
                size: wgpu::Extent3d {
                    width: extent[0],
                    height: extent[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: OUTPUT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            device,
            queue,
            host,
            witness: None,
            output,
            submitted: 0,
            extent,
        }
    }

    pub(crate) fn frame(&mut self) -> FrameSample {
        self.submitted += 1;
        self.host
            .drive(
                Duration::from_secs_f64(f64::from(self.submitted) / 60.0),
                Some(&SurfaceOutput {
                    view: &self.output,
                    extent: self.extent,
                }),
            )
            .expect("drive optical fixture");
        self.witness = self.host.submitted_temporal_witness().cloned();
        thrum_test_support::wait::until("fixed optical frame", Duration::from_secs(30), || {
            self.device.poll(wgpu::PollType::Poll).expect("device");
            self.host.poll_completed().expect("completion")
        })
    }

    pub(crate) fn scene_colour(&mut self) -> (f32, Vec<[f32; 3]>) {
        let (_, scale, rgb) = self.scene_colour_with_sample();
        (scale, rgb)
    }

    pub(crate) fn scene_colour_with_sample(&mut self) -> (FrameSample, f32, Vec<[f32; 3]>) {
        let limits = CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            ..CaptureLimits::default()
        };
        self.host
            .enable_submitted_capture_from(1, limits, CaptureSource::SceneLinear)
            .expect("scene-linear tap");
        let sample = self.frame();
        let image =
            thrum_test_support::wait::until("optics readback", Duration::from_secs(30), || {
                self.device.poll(wgpu::PollType::Poll).expect("device");
                self.host.try_submitted_image().expect("image")
            });
        assert_eq!(sample.submission_order, image.sample.submission_order);
        let scale = f32::from_bits(image.exposure_gpu_words.expect("exposure provenance")[0]);
        let rgb = image
            .rgba
            .as_chunks::<8>()
            .0
            .iter()
            .map(|p| {
                std::array::from_fn(|lane| {
                    crate::support::half_value(u16::from_le_bytes([p[2 * lane], p[2 * lane + 1]]))
                })
            })
            .collect();
        self.host.recycle_submitted_image(image);
        self.host.disable_submitted_capture();
        (sample, scale, rgb)
    }
}

pub(crate) fn sun(altitude: f32, aerosol: f32) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.1, 0.93, 0.3], 80_000.0, [1.0; 3], altitude, 1)
            .expect("physical Sun")
            .with_aerosol(Aerosol::new(aerosol, 0.0).expect("aerosol")),
    )
}

pub(crate) fn camera(eye: [f32; 3], target: [f32; 3], fov: f32, exposure: f32) -> SyntheticView {
    SyntheticView::perspective(eye, target, [0.0, 1.0, 0.0], fov, 16.0 / 9.0, 0.1, 30_000.0)
        .expect("camera")
        .with_exposure(exposure)
        .expect("fixed exposure")
}

pub(crate) fn water(bottom: Option<(f64, [f64; 2], f64)>, optics: bool) -> Water {
    water_with_profile(bottom, optics, false)
}

pub(crate) fn water_with_profile(
    bottom: Option<(f64, [f64; 2], f64)>,
    optics: bool,
    underwater: bool,
) -> Water {
    let (depth, origin, step) = bottom.unwrap_or((-1000.0, [-2.0; 2], 4.0));
    let map = Map::prepare(
        Bed::new(origin, step, [2, 2], vec![depth; 4]).expect("bed"),
        vec![Body {
            kind: BodyKind::Sea,
            name: "fixture water".into(),
            level: 0.0,
            stage: Stage {
                min: 0.0,
                max: 0.0,
                value: 0.0,
            },
            fetch: 100.0,
            gains: [1.0; 4],
        }],
        thrum_water_model::Profile {
            margin_height: 1.0,
            swash_ratio: 0.7,
        },
    )
    .expect("map");
    let spectrum = thrum_sea_model::Spectrum {
        long: Vec::new(),
        cascades: std::array::from_fn(|i| {
            thrum_sea_model::Cascade::new([160.0, 37.0, 9.0][i], &[]).expect("empty cascade")
        }),
        tail_slope_variance: 0.0,
    };
    let profile = Profile {
        optics: optics.then(Optics::default),
        underwater,
        deep_scatter: [0.004, 0.035, 0.065],
        ..Profile::default()
    };
    let map = Arc::new(Mutex::new(map));
    if bottom.is_some() {
        Water::mapped(map, spectrum, profile).expect("mapped fixture")
    } else {
        Water::new(map, 0, spectrum, profile).expect("open water fixture")
    }
}

pub(crate) fn attach(rig: &mut Rig, water: Water) -> OpticsBodies {
    let opaque = water.optics_bodies();
    rig.host
        .set_scene_contribution(Box::new(water))
        .expect("water prepared");
    opaque
}
