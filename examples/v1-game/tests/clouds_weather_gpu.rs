//! The authored fair-weather fixture at the same numerical bounds as Chrome.
/// Shared native diagnostic device policy.
#[cfg(target_os = "macos")]
use crate::support;

#[cfg(target_os = "macos")]
#[test]
fn portable_metal_publishes_the_authored_weather_within_the_shared_tolerance() {
    use std::time::Duration;
    use thrum_clouds::WeatherMap;
    use thrum_render::{contribution::EffectTime, runtime::DeviceSite};
    use v1_game::clouds::{
        initial_state,
        qualification::{WeatherSnapshot, inspect_weather},
        shade_config,
    };
    let Some(test_gpu) = thrum_test_support::gpu(
        "portable_metal_publishes_the_authored_weather_within_the_shared_tolerance",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };

    pollster::block_on(async {
        let (_, device, queue, caps) = support::device(&test_gpu, "portable", false);
        println!("adapter: {}", caps.adapter().name);
        let config = shade_config();
        let mut map = WeatherMap::new(
            &DeviceSite::untracked(device.clone()),
            config.seed,
            config.model,
            config.types.clone(),
            config.weather.clone(),
        )
        .expect("V1 weather");
        map.rebuild(1, 1);
        let mut state = initial_state();
        for tick in [0, 1] {
            let tick_duration = Duration::from_secs_f64(1.0 / 60.0);
            if tick != 0 {
                state
                    .step(tick, tick_duration.as_secs_f64())
                    .expect("next tick");
            }
            let time = EffectTime {
                epoch: 1,
                timeline: 1,
                tick,
                tick_duration,
                alpha: 0.5,
                display_phase: 0.5,
                capture: false,
            };
            map.upload(&queue, time, &state, [0.0; 2]).expect("upload");
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            map.encode(&mut encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default()));
            queue.submit([encoder.finish()]);
            map.submitted();
            let result = inspect_weather(
                &device,
                &queue,
                &config,
                WeatherSnapshot {
                    textures: map.textures(),
                    origins: map.origins(),
                    rows: map.row_stamps(),
                },
            )
            .await
            .expect("weather at the same bounds as Chrome");
            assert!(result.counts.into_iter().all(|count| count > 0));
            println!("portable Metal tick {tick}: {result:?}");
        }
    });
}
