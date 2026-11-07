fn timed_route(host: &mut FrameHost, device: &wgpu::Device, output: &Path, cooked: impl Fn() -> Result<(Option<u64>, u64), String>) -> Result<(), String> {
    use std::io::Write as _;
    let mut output = std::fs::File::create(output).map_err(|error| error.to_string())?;
    host.resume(Duration::ZERO)?;
    let mut last_submission = cooked()?.1;
    for frame in 0_u16..80 {
        let group = frame / 20;
        let step = f32::from(frame % 20) / 19.0;
        let hours = [6.35, 12.0, 17.55, 17.691_667][usize::from(group)];
        let yaw = [90.0_f32, 0.0, -90.0, -90.0][usize::from(group)] + (step - 0.5) * 6.0;
        host.set_camera_yaw(yaw.to_radians())?;
        host.set_time_of_day(hours / 24.0)?;
        let stamp = Duration::from_micros(u64::from(frame + 1) * 16_667);
        let report = host.drive(stamp, None)?;
        if !report.submitted { return Err(format!("frame {frame} did not submit")); }
        let deadline = Instant::now() + Duration::from_secs(15);
        let row = loop {
            device.poll(wgpu::PollType::wait_indefinitely()).map_err(|error| error.to_string())?;
            if let Some(row) = host.poll_completed()? { break row; }
            if Instant::now() >= deadline { return Err(format!("frame {frame} did not complete")); }
        };
        let (generation, submission) = cooked()?;
        if generation.is_none() || submission != last_submission + 1 {
            return Err(format!("frame {frame} did not submit the loaded contribution"));
        }
        last_submission = submission;
        let row = serde_json::json!({
            "frame":frame,"hours":hours,"yaw":yaw,"tick":row.tick,
            "render_cpu_ns":row.render_cpu.as_nanos(),"encode_cpu_ns":row.encode_cpu.as_nanos(),
            "gpu_ns":row.gpu_pass_ns,"timing_fault":row.timing_fault,
            "cooked_generation":generation,"cooked_submitted":submission,
        });
        writeln!(output, "{row}").map_err(|error| error.to_string())?;
    }
    Ok(())
}
