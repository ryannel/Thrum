//! Focused lighting diagnosis on a real Metal scene contribution.
mod fixture;
use super::*;
use std::{path::Path, time::Duration};
use thrum_render::contribution::EffectTime;

struct Observed {
    probe: Option<crate::layer::diagnostic::Probe>,
    shade: Shade,
    output: Arc<Mutex<Option<Vec<wgpu::Texture>>>>,
    buffers: Arc<Mutex<Option<Vec<wgpu::Buffer>>>>,
}

impl SceneContribution for Observed {
    fn passes(&self) -> ContributionPasses {
        self.shade.passes()
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.shade.rebuild(surface)?;
        let gpu = self.shade.gpu.as_mut().expect("prepared clouds");
        let sky = gpu.sky.as_mut().expect("visible clouds");
        let probe = crate::layer::diagnostic::Probe::new(sky, surface.device);
        *self.buffers.lock().expect("buffers") =
            Some(vec![probe.output.clone(), sky.products().hemisphere_light]);
        let mut textures = gpu.weather.textures()[..2].to_vec();
        textures.push(sky.products().fresh_transport);
        *self.output.lock().expect("textures") = Some(textures);
        self.probe = Some(probe);
        Ok(())
    }
    fn effect_graph(&self) -> EffectGraph {
        self.shade.effect_graph()
    }
    fn effect_runs(&self, index: usize, frame: &ContributionFrame) -> bool {
        self.shade.effect_runs(index, frame)
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        self.shade.encode_effect(index, pass, frame);
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.shade.encode_upload(encoder, frame);
    }
    fn encode_after_opaque(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        self.shade.encode_after_opaque(index, pass, frame);
        if index == 1 {
            self.probe.as_ref().expect("prepared observer").encode(pass);
        }
    }
    fn after_opaque_runs(&self, index: usize, frame: &ContributionFrame) -> bool {
        self.shade.after_opaque_runs(index, frame)
    }
    fn draw_after_opaque(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        self.shade.draw_after_opaque(index, pass, frame);
    }
    fn submitted(&mut self, history: bool) {
        self.shade.submitted(history);
    }
}

fn config() -> ShadeConfig {
    ShadeConfig {
        seed: 39,
        model: Profile {
            pitch: 3000.0,
            turret_pitch: 850.0,
            turret_least: 0.18,
            turret_fade: 0.12,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.7,
        },
        types: vec![
            Type {
                base: 1600.0,
                depth: [300.0, 600.0],
                liquid_water: 0.00025,
                droplet_radius: 0.00001,
                deck: 1.0,
                profile: [0.2, 0.2],
            },
            Type {
                base: 1600.0,
                depth: [1100.0, 3000.0],
                liquid_water: 0.0004,
                droplet_radius: 0.000_012,
                deck: 0.0,
                profile: [0.08, 0.45],
            },
        ],
        weather: WeatherProfile {
            size: 256,
            texels: vec![20.0, 80.0, 320.0, 3200.0],
            maximum_age: 0.5,
        },
        sun: SunProfile {
            size: 256,
            levels: [80.0, 320.0, 1600.0]
                .into_iter()
                .zip([48, 32, 16])
                .map(|(texel, slices)| crate::SunLevel {
                    texel,
                    slices,
                    maximum_age: 0.5,
                })
                .collect::<Vec<_>>()
                .try_into()
                .expect("three levels"),
            ground_size: 64,
            lowest_elevation: 2.0_f64.to_radians(),
            border_fraction: 0.1,
        },
    }
}

#[test]
#[ignore = "focused diagnosis writes stage snapshots; run explicitly under the GPU lock"]
#[expect(
    clippy::cast_possible_truncation,
    reason = "fixed fixture coordinates narrow to the production shader precision"
)]
fn lighting_stage_snapshots() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "cloud lighting diagnosis",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let caps = thrum_render::capabilities::DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let state = State::new(
        thrum_cloud_model::Parameters {
            coverage: 0.45,
            kind: 1.0,
            wind: [0.0; 2],
            time_scale: 1.0,
        },
        5.0,
        0,
    )
    .expect("static weather");
    let c = config();
    let field = Field::new(c.seed, c.model, c.types.clone()).expect("field");
    let mut centre = [0.0; 2];
    let mut thickest = 0.0;
    for x in -12..12 {
        for z in -12..12 {
            let p = [f64::from(x) * 250.0, f64::from(z) * 250.0];
            let column = field.column(p, 0.0, state.parameters());
            if column.optical_depth > thickest {
                thickest = column.optical_depth;
                centre = p;
            }
        }
    }
    assert!(thickest > 10.0, "occupied cloud fixture");
    let column = field.column(centre, 0.0, state.parameters());
    let root = Path::new("../../tmp/clouds-lighting-diagnosis");
    std::fs::create_dir_all(root).expect("diagnostic directory");
    for elevation in [60.0_f64, 10.0] {
        let sun = [
            0.0,
            elevation.to_radians().sin(),
            elevation.to_radians().cos(),
        ];
        for (name, dx, height, offset) in [
            ("side", 0.0, 0.45, 2400.0),
            ("base", 0.0, 0.04, 800.0),
            ("edge", 600.0, 0.45, 2400.0),
        ] {
            let target = [
                centre[0] + dx,
                1600.0 + (column.top - 1600.0) * height,
                centre[1],
            ];
            let camera = [target[0], 1200.0, target[2] + offset];
            let shade = Shade::new(config())
                .expect("shade")
                .with_sky(crate::SkyProfile {
                    extent: [258, 258],
                    step: 15.0,
                    light_samples: 3,
                    refresh_stride: 1,
                })
                .expect("sky");
            shade
                .input()
                .publish(Arc::new(state.clone()))
                .expect("weather");
            let textures = Arc::new(Mutex::new(None));
            let buffers = Arc::new(Mutex::new(None));
            let observed = Observed {
                probe: None,
                shade,
                output: Arc::clone(&textures),
                buffers: Arc::clone(&buffers),
            };
            pollster::block_on(fixture::run(
                &device,
                &caps,
                &queue,
                Box::new(observed),
                camera.map(|v| v as f32),
                target.map(|v| v as f32),
                sun.map(|v| v as f32),
            ));
            let path = root.join(format!("{elevation}-{name}"));
            std::fs::create_dir_all(&path).expect("case directory");
            for (i, t) in textures
                .lock()
                .expect("textures")
                .as_ref()
                .expect("prepared")
                .iter()
                .enumerate()
            {
                let packed = thrum_test_support::readback::texture(
                    &device,
                    &queue,
                    t,
                    thrum_test_support::readback::Mip::level(0),
                    ..,
                );
                std::fs::write(path.join(format!("texture-{i}.bin")), packed)
                    .expect("texture snapshot");
            }
            for (i, b) in buffers
                .lock()
                .expect("buffers")
                .as_ref()
                .expect("prepared")
                .iter()
                .enumerate()
            {
                let bytes = thrum_test_support::readback::buffer(&device, &queue, b, ..);
                if i == 0 {
                    let count =
                        f32::from_le_bytes(bytes[112..116].try_into().expect("sample count"));
                    assert!(count > 0.0, "each diagnostic ray crosses occupied cloud");
                }
                std::fs::write(path.join(format!("buffer-{i}.bin")), bytes).expect("snapshot");
            }
            // Weather coordinates are fixed by the uploaded camera and zero travel.
            let mut map = WeatherMap::new(
                &thrum_render::runtime::DeviceSite::untracked(device.clone()),
                39,
                c.model,
                c.types.clone(),
                c.weather.clone(),
            )
            .expect("mapping reference");
            map.upload(
                &queue,
                EffectTime {
                    epoch: 1,
                    timeline: 1,
                    tick: 0,
                    tick_duration: Duration::from_secs_f64(1.0 / 60.0),
                    alpha: 1.0,
                    display_phase: 1.0,
                    capture: false,
                },
                &state,
                [camera[0], camera[2]],
            )
            .expect("mapping");
            let origins: Vec<_> = map
                .origins()
                .iter()
                .zip(&c.weather.texels)
                .map(|(p, t)| format!("{} {} {t}", p[0], p[1]))
                .collect();
            std::fs::write(path.join("mapping.txt"), origins.join("\n")).expect("mapping snapshot");
        }
    }
}
