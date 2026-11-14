//! Shared independent oracle and target readback harness.
use std::{fmt::Write as _, sync::mpsc, time::Duration};
use thrum_render::runtime::DeviceSite;
use thrum_sea::{Ambient, MAX_QUERIES, MIP_PIXELS, PIXELS, Profile};
use thrum_sea_model::{Forcing, LongWaves, Spectrum};

pub(super) struct Harness {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) sea: Ambient,
    pub(super) record: String,
}
pub(super) struct Output {
    pub(super) long: Vec<f32>,
    pub(super) cascades: Vec<f32>,
    pub(super) moments: Vec<f32>,
}
pub(super) fn spectrum(wind: f64) -> Spectrum {
    Spectrum::build(&Forcing {
        wind_speed: wind,
        wind_direction: 0.41,
        fetch: 1e6,
        swells: vec![],
    })
    .expect("valid Metal qualification fixture")
}
pub(super) const ANCHOR: [f64; 2] = [50000.25, -50000.75];

impl Harness {
    pub(super) fn new(portable: bool) -> Self {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::METAL,
                ..wgpu::InstanceDescriptor::new_without_display_handle()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .expect("mac-metal adapter is required");
            assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
            let features = wgpu::Features::TIMESTAMP_QUERY;
            assert!(
                adapter.features().contains(features),
                "Metal pass timings required"
            );
            let limits = if portable {
                wgpu::Limits::default()
            } else {
                adapter.limits()
            };
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    required_features: features,
                    required_limits: limits,
                    ..Default::default()
                })
                .await
                .expect("valid Metal qualification fixture");
            let site = DeviceSite::untracked(device.clone());
            let sea =
                Ambient::new(&site, Profile::default()).expect("valid Metal qualification fixture");
            sea.graph()
                .validate()
                .expect("valid Metal qualification fixture");
            check_graph(&sea.graph());
            let mut record = format!(
                "adapter={:?}\nportable={portable}\nlimits={:?}\n",
                adapter.get_info(),
                device.limits()
            );
            for (command, args) in [
                ("git", vec!["rev-parse", "HEAD"]),
                ("uptime", vec![]),
                ("sysctl", vec!["-n", "hw.model", "hw.memsize", "hw.ncpu"]),
                ("pmset", vec!["-g", "therm"]),
            ] {
                let output = std::process::Command::new(command)
                    .args(args)
                    .output()
                    .expect("machine identity witness");
                writeln!(
                    record,
                    "{command}: {}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
                .expect("record");
            }
            for (name, bytes) in sea.resource_bytes() {
                writeln!(record, "resource {name}: {bytes} bytes")
                    .expect("valid Metal qualification fixture");
            }
            Self {
                device,
                queue,
                sea,
                record,
            }
        })
    }
    pub(super) fn frame(
        &mut self,
        spectrum: &Spectrum,
        time: f64,
        points: &[[f64; 2]],
        submit: bool,
    ) -> Option<Output> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.sea
            .upload(&self.queue, &mut encoder, spectrum, time, ANCHOR, points)
            .expect("valid Metal qualification fixture");
        let graph = self.sea.graph();
        let count = u32::try_from(graph.passes.len()).expect("valid Metal qualification fixture");
        let timestamps = self.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("sea pass timings"),
            ty: wgpu::QueryType::Timestamp,
            count: count * 2,
        });
        for (i, pass) in graph.passes.iter().enumerate() {
            let i32 = u32::try_from(i).expect("valid Metal qualification fixture");
            let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&pass.name),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &timestamps,
                    beginning_of_pass_write_index: Some(2 * i32),
                    end_of_pass_write_index: Some(2 * i32 + 1),
                }),
            });
            self.sea.encode_pass(i, &mut compute);
        }
        if !submit {
            drop(encoder);
            return None;
        }
        let resolve = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timings resolve"),
            size: u64::from(count) * 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        encoder.resolve_query_set(&timestamps, 0..count * 2, &resolve, 0);
        let timing = copy(&self.device, &mut encoder, &resolve);
        let long = copy(&self.device, &mut encoder, self.sea.long_buffer());
        let cascades = copy(&self.device, &mut encoder, self.sea.cascade_buffer());
        let moments: Vec<_> = (0..=8)
            .map(|mip| {
                copy(
                    &self.device,
                    &mut encoder,
                    self.sea.moments_buffer(mip).expect("moment mip"),
                )
            })
            .collect();
        self.queue.submit([encoder.finish()]);
        let timing = read(&self.device, &timing);
        for (pass, bytes) in graph.passes.iter().zip(timing.as_chunks::<16>().0.iter()) {
            let start = u64::from_le_bytes(
                bytes[..8]
                    .try_into()
                    .expect("valid Metal qualification fixture"),
            );
            let end = u64::from_le_bytes(
                bytes[8..]
                    .try_into()
                    .expect("valid Metal qualification fixture"),
            );
            let ms = (end - start) as f64 * f64::from(self.queue.get_timestamp_period()) / 1e6;
            writeln!(self.record, "time {time:.9} {}: {ms:.6} ms", pass.name)
                .expect("valid Metal qualification fixture");
        }
        let moment_mips: Vec<_> = moments
            .iter()
            .map(|b| floats(&read(&self.device, b)))
            .collect();
        let mut moments = Vec::new();
        for band in 0..3 {
            for mip in &moment_mips {
                let count = mip.len() / 3;
                moments.extend_from_slice(&mip[band * count..(band + 1) * count]);
            }
        }
        Some(Output {
            long: floats(&read(&self.device, &long)),
            cascades: floats(&read(&self.device, &cascades)),
            moments,
        })
    }
    pub(super) fn save(&self, name: &str) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/sea-m2");
        std::fs::create_dir_all(&path).expect("valid Metal qualification fixture");
        std::fs::write(path.join(format!("{name}.txt")), &self.record)
            .expect("valid Metal qualification fixture");
    }
}
fn check_graph(declaration: &thrum_render::contribution::EffectGraph) {
    use thrum_render::graph::{Access, FrameGraph, Pass, PassScope};
    let mut graph = FrameGraph::default();
    let resources: Vec<_> = declaration
        .resources
        .iter()
        .map(|(d, s)| graph.resource(d.clone(), *s).expect("real descriptors"))
        .collect();
    for pass in &declaration.passes {
        graph.pass(Pass {
            name: pass.name.clone(),
            scope: PassScope::View(1),
            pipeline: None,
            accesses: pass
                .accesses
                .iter()
                .map(|a| Access {
                    resource: resources[a.resource].clone(),
                    range: a.range.clone(),
                    kind: a.kind,
                    view: a.view,
                    store: true,
                })
                .collect(),
            external_side_effect: true,
        });
    }
    let compiled = graph
        .compile(1, &std::collections::BTreeSet::new())
        .expect("all transient reads follow complete producers");
    assert_eq!(
        compiled.pass_names().collect::<Vec<_>>(),
        declaration
            .passes
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
    );
}

fn copy(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    source: &wgpu::Buffer,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sea oracle readback"),
        size: source.size(),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(source, 0, &buffer, 0, source.size());
    buffer
}
fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = mpsc::channel();
    buffer.map_async(wgpu::MapMode::Read, .., move |r| {
        tx.send(r).expect("valid Metal qualification fixture");
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(120)),
        })
        .expect("valid Metal qualification fixture");
    rx.recv()
        .expect("valid Metal qualification fixture")
        .expect("valid Metal qualification fixture");
    let view = buffer
        .get_mapped_range(..)
        .expect("valid Metal qualification fixture");
    let result = view.to_vec();
    drop(view);
    buffer.unmap();
    result
}
fn floats(data: &[u8]) -> Vec<f32> {
    data.as_chunks::<4>()
        .0
        .iter()
        .map(|v| {
            let value = f32::from_le_bytes(*v);
            assert!(value.is_finite(), "nonfinite GPU readback");
            value
        })
        .collect()
}
pub(super) fn points() -> Vec<[f64; 2]> {
    (0..4096_u32)
        .map(|i| {
            let x = f64::from(
                i.wrapping_mul(2_654_435_761)
                    .wrapping_add(17)
                    .wrapping_mul(1_664_525)
                    % 100_000,
            ) / 100_000.0;
            let z = f64::from(i.wrapping_mul(2_246_822_519).wrapping_add(3_266_489_917) % 100_000)
                / 100_000.0;
            [
                ANCHOR[0] + 8000.0 * (x - 0.5),
                ANCHOR[1] + 8000.0 * (z - 0.5),
            ]
        })
        .collect()
}
pub(super) fn check_long(
    h: &mut Harness,
    out: &Output,
    s: &Spectrum,
    time: f64,
    points: &[[f64; 2]],
    full: bool,
) {
    let mut height = 0.0_f64;
    let mut slope = 0.0_f64;
    let oracle = LongWaves::new(s.long.clone(), 0.1).expect("valid Metal qualification fixture");
    for (i, &p) in points.iter().enumerate() {
        compare_long(
            &out.long[i * 12..],
            oracle
                .evaluate(p, time)
                .expect("valid Metal qualification fixture"),
            &mut height,
            &mut slope,
        );
    }
    for (level, info) in h.sea.levels.iter().enumerate() {
        let oracle = LongWaves::new(
            s.long
                .iter()
                .copied()
                .filter(|c| c.wave_vector[0].hypot(c.wave_vector[1]) <= info.max_wavenumber)
                .collect(),
            0.1,
        )
        .expect("valid Metal qualification fixture");
        for i in (0_u32..128 * 128).step_by(if full { 1 } else { 257 }) {
            let p = [
                info.origin[0] + (f64::from(i % 128) + 0.5) * info.texel,
                info.origin[1] + (f64::from(i / 128) + 0.5) * info.texel,
            ];
            let index = MAX_QUERIES as usize + level * 128 * 128 + i as usize;
            compare_long(
                &out.long[index * 12..],
                oracle
                    .evaluate(p, time)
                    .expect("valid Metal qualification fixture"),
                &mut height,
                &mut slope,
            );
        }
    }
    writeln!(
        h.record,
        "long time {time}: max height {height:.9} m; max slope {slope:.9}"
    )
    .expect("valid Metal qualification fixture");
    assert!(height < 0.002, "long height {height}");
    assert!(slope < 0.002, "slope {slope}");
}
fn compare_long(gpu: &[f32], cpu: thrum_sea_model::Sample, h: &mut f64, s: &mut f64) {
    *h = h.max((f64::from(gpu[1]) - cpu.height).abs());
    for axis in 0..2 {
        *s = s.max((f64::from(gpu[4 + axis]) - cpu.slope[axis]).abs());
    }
}
// First run records an observed bound; the selected bound is kept unchanged at both ages.
// 2026-11-14, bd36ca70: worst measured 0.000191522 across both ages and
// the lifecycle script. Rounded upward at twice that maximum, below one percent.
pub(super) const CASCADE_BOUND: f64 = 0.0004;
pub(super) fn check_cascades(h: &mut Harness, out: &Output, s: &Spectrum, time: f64, samples: u32) {
    for (band, c) in s.cascades.iter().enumerate() {
        let rms = c.variance().sqrt();
        let mut largest = 0.0_f64;
        for i in 0..samples {
            let pixel = (i * 4051 + 137) % PIXELS;
            let p = [
                (f64::from(pixel % 256) + 0.5) * c.length() / 256.0,
                (f64::from(pixel / 256) + 0.5) * c.length() / 256.0,
            ];
            let cpu = c.direct(p, time);
            let offset = (band * PIXELS as usize + pixel as usize) * 12;
            for axis in 0..3 {
                largest = largest.max(
                    (f64::from(out.cascades[offset + axis]) - cpu.displacement[axis]).abs() / rms,
                );
                largest = largest.max(
                    (f64::from(out.cascades[offset + 8 + axis]) - cpu.velocity[axis]).abs() / rms,
                );
            }
        }
        writeln!(
            h.record,
            "cascade {band} time {time}: max displacement/rate error per RMS {largest:.9}"
        )
        .expect("valid Metal qualification fixture");
        assert!(
            largest < CASCADE_BOUND,
            "band {band} time {time} measured {largest}, bound {CASCADE_BOUND}"
        );
    }
}
pub(super) fn check_moments(out: &Output) {
    for band in 0..3_usize {
        let mut width = 256_usize;
        let mut offset = 0;
        for level in 0..=8 {
            let block = 1 << level;
            // Every mip texel, against direct base-level moments rather than the parent mip.
            for z in 0..width {
                for x in 0..width {
                    let mut sums = [0.0; 7];
                    for dz in 0..block {
                        for dx in 0..block {
                            let p =
                                (band * PIXELS as usize + (z * block + dz) * 256 + x * block + dx)
                                    * 12;
                            let sx = f64::from(out.cascades[p + 4]);
                            let sz = f64::from(out.cascades[p + 5]);
                            let jac = f64::from(out.cascades[p + 3]);
                            for (sum, v) in sums.iter_mut().zip([
                                sx,
                                sz,
                                sx * sx,
                                sx * sz,
                                sz * sz,
                                jac,
                                jac * jac,
                            ]) {
                                *sum += v;
                            }
                        }
                    }
                    let p = (band * MIP_PIXELS as usize + offset + z * width + x) * 8;
                    for (j, sum) in sums.into_iter().enumerate() {
                        let expected = sum / (block * block) as f64;
                        assert!(
                            (f64::from(out.moments[p + j]) - expected).abs()
                                < 2e-5 * (1.0 + expected.abs())
                        );
                    }
                }
            }
            offset += width * width;
            width /= 2;
        }
    }
}
pub(super) fn ambient(portable: bool, name: &str) {
    frame_script(portable);
    let mut h = Harness::new(portable);
    let s = spectrum(20.0);
    let p = points();
    for time in [19.0 + 0.375 / 60.0, 86400.0 + 0.375 / 60.0] {
        let out = h
            .frame(&s, time, &p, true)
            .expect("valid Metal qualification fixture");
        check_long(&mut h, &out, &s, time, &p, true);
        check_cascades(&mut h, &out, &s, time, 1024);
        check_moments(&out);
        if time > 86400.0 {
            naive_seconds_ablation(&mut h, &s, time);
        }
    }
    h.save(name);
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "negative control deliberately forms phase in binary32 seconds"
)]
fn naive_seconds_ablation(h: &mut Harness, s: &Spectrum, time: f64) {
    let c = &s.cascades[2];
    let mut largest = 0.0_f64;
    for pixel in (0..PIXELS).step_by(1024) {
        let point = [
            (f64::from(pixel % 256) + 0.5) * c.length() / 256.0,
            (f64::from(pixel / 256) + 0.5) * c.length() / 256.0,
        ];
        let mut naive = 0.0_f64;
        for mode in c.components() {
            let omega = thrum_sea_model::cascade_frequency(*mode) as f32;
            let phase = mode.wave_vector[0] as f32 * point[0] as f32
                + mode.wave_vector[1] as f32 * point[1] as f32
                + mode.phase as f32
                - omega * time as f32;
            naive += mode.amplitude * f64::from(phase.cos());
        }
        largest = largest
            .max((naive - c.direct(point, time).displacement[1]).abs() / c.variance().sqrt());
    }
    writeln!(
        h.record,
        "negative control binary32 seconds at {time}: height error/RMS {largest:.9}"
    )
    .expect("record");
    assert!(
        largest > CASCADE_BOUND,
        "24-hour seconds clock must fail recorded bound"
    );
}

fn frame_script(portable: bool) {
    // Also keeps this target's full-grid/moment conformance under the same device limits.
    let mut h = Harness::new(portable);
    let s = spectrum(20.0);
    let p: Vec<_> = points().into_iter().take(16).collect();
    // (tick, alpha, capture, discard, timeline): the source identity cannot affect stateless fields.
    let frames = [
        (600, 0.1, false, false, 0),
        (600, 0.9, false, false, 0),
        (600, 0.9, false, false, 0),
        (601, 0.0, false, false, 0),
        (900, 0.7, false, false, 0),
        (901, 0.4, false, true, 0),
        (902, 0.2, false, false, 0),
        (53, 0.3, true, false, 0),
        (903, 0.8, false, false, 0),
        (8, 0.2, false, false, 0),
        (8, 0.8, false, false, 1),
        (0, 1.0, false, false, 2),
        (0, 0.5, true, false, 2),
        (1, 0.0, false, false, 2),
    ];
    let mut previous = None;
    let mut first = None;
    for (i, (tick, alpha, capture, discard, timeline)) in frames.into_iter().enumerate() {
        let clock = thrum_render::contribution::EffectTime {
            epoch: 7,
            timeline,
            tick,
            tick_duration: Duration::from_secs_f64(1.0 / 60.0),
            alpha,
            capture,
        };
        let time = thrum_sea::display_seconds(clock);
        let expected = (tick as f64 - 1.0 + f64::from(alpha)) * clock.tick_duration.as_secs_f64();
        assert_eq!(time, expected);
        let out = h.frame(&s, time, &p, !discard);
        if let Some(out) = out {
            check_long(&mut h, &out, &s, time, &p, false);
            check_cascades(&mut h, &out, &s, time, 32);
            if i == 0 {
                first = Some(out.long[..p.len() * 12].to_vec());
            }
            if i == 1 {
                assert_ne!(
                    first.as_ref().expect("valid frame script"),
                    &out.long[..p.len() * 12]
                );
            }
            if i == 2 {
                assert_eq!(previous.as_ref().expect("valid frame script"), &out.long);
            }
            previous = Some(out.long);
        }
        writeln!(h.record,"script tick={tick} alpha={alpha} capture={capture} discard={discard} timeline={timeline}").expect("valid frame script");
    }
    h.save("frames");
    // A genuinely new Metal device and all generation-owned pipelines/resources.
    h.device.destroy();
    drop(h);
    let mut h = Harness::new(portable);
    let out = h
        .frame(&s, 86400.003, &p, true)
        .expect("valid frame script");
    check_long(&mut h, &out, &s, 86400.003, &p, false);
    check_cascades(&mut h, &out, &s, 86400.003, 32);
    let transition = thrum_sea_model::Transition::new(s, spectrum(7.0), 86400.0, 10.0)
        .expect("valid frame script");
    let mut height: Option<f64> = None;
    for time in [
        86400.0 - 1e-6,
        86400.0,
        86400.0 + 1e-6,
        86400.125,
        86401.5,
        86410.0,
        86460.0,
    ] {
        let spectrum = transition.at(time).expect("valid frame script");
        let out = h
            .frame(&spectrum, time, &p, true)
            .expect("valid frame script");
        check_long(&mut h, &out, &spectrum, time, &p, false);
        check_cascades(&mut h, &out, &spectrum, time, 32);
        if time <= 86400.0 + 1e-6 {
            if let Some(before) = height {
                assert!((f64::from(out.long[1]) - before).abs() < 1e-4);
            }
            height = Some(f64::from(out.long[1]));
        }
        let cpu = LongWaves::new(spectrum.long.clone(), 0.1)
            .expect("valid frame script")
            .evaluate(ANCHOR, time)
            .expect("valid frame script");
        assert!(cpu.height.is_finite());
    }
    h.save("replaced-device-transition");
}
