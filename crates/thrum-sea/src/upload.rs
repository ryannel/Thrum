use crate::{Ambient, MAX_QUERIES, PIXELS};
use std::f64::consts::{PI, TAU};
use thrum_sea_model::{LOOP_SECONDS, Spectrum, loop_cycles};

#[expect(
    clippy::cast_possible_truncation,
    reason = "f64 physical values are explicitly reduced to bounded GPU-relative f32; loop fraction is in u32 range"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "loop time is reduced to nonnegative fraction and lattice indices use rem_euclid"
)]
pub(super) fn frame(
    sea: &mut Ambient,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    spectrum: &Spectrum,
    time: f64,
    anchor: [f64; 2],
    queries: &[[f64; 2]],
) -> Result<(), String> {
    if !time.is_finite()
        || anchor.iter().any(|v| !v.is_finite() || v.abs() > 1e12)
        || queries.len() > MAX_QUERIES as usize
        || queries.iter().flatten().any(|v| !v.is_finite())
        || spectrum.long.len() > 192
    {
        return Err(
            "sea upload: finite time/anchor, <=192 components and <=4096 queries required".into(),
        );
    }
    let steepness: f64 = spectrum
        .long
        .iter()
        .map(|c| sea.profile.choppiness * c.amplitude * c.wave_vector[0].hypot(c.wave_vector[1]))
        .sum();
    if !steepness.is_finite() || steepness >= 0.5 {
        return Err("sea long waves: contraction bound must be below 0.5".into());
    }
    for band in &spectrum.cascades {
        if !band.length().is_finite()
            || band.length() <= 0.0
            || band.components().iter().any(|c| {
                !c.amplitude.is_finite()
                    || !c.phase.is_finite()
                    || c.wave_vector.iter().any(|v| !v.is_finite())
            })
        {
            return Err("sea cascade: invalid physical input".into());
        }
    }
    sea.upload.fill(0);
    let fraction = (time.rem_euclid(LOOP_SECONDS) / LOOP_SECONDS * 4_294_967_296.0) as u32;
    let query_count = MAX_QUERIES;
    let long_count = u32::try_from(spectrum.long.len()).map_err(|_| "sea component count")?;
    let data = &mut sea.upload;
    for (i, v) in [fraction, long_count, query_count, 0]
        .into_iter()
        .enumerate()
    {
        put(data, i * 4, v.to_le_bytes());
    }
    put(data, 16, (sea.profile.choppiness as f32).to_le_bytes());
    for (band, cascade) in spectrum.cascades.iter().enumerate() {
        put(data, 32 + band * 4, (cascade.length() as f32).to_le_bytes());
    }
    for (i, level) in sea.levels.iter_mut().enumerate() {
        level.texel = f64::from(1_u32 << i);
        level.max_wavenumber = PI / level.texel;
        level.origin = anchor.map(|v| (v / level.texel).floor() * level.texel - 64.0 * level.texel);
        for (j, v) in [
            level.origin[0] - anchor[0],
            level.origin[1] - anchor[1],
            level.texel,
            level.max_wavenumber,
        ]
        .into_iter()
        .enumerate()
        {
            put(data, 48 + i * 16 + j * 4, (v as f32).to_le_bytes());
        }
    }
    let modes_offset = 176;
    for (band, cascade) in spectrum.cascades.iter().enumerate() {
        for c in cascade.components() {
            let lattice = c
                .wave_vector
                .map(|k| (k * cascade.length() / TAU).round() as i32);
            if lattice.iter().any(|v| v.abs() >= 128) || lattice == [0, 0] {
                return Err("sea cascade: transform_size lattice limit".into());
            }
            let index = band * PIXELS as usize
                + lattice[1].rem_euclid(256) as usize * 256
                + lattice[0].rem_euclid(256) as usize;
            let offset = modes_offset + index * 32;
            for (j, v) in [c.wave_vector[0], c.wave_vector[1], c.amplitude, c.phase]
                .into_iter()
                .enumerate()
            {
                put(data, offset + j * 4, (v as f32).to_le_bytes());
            }
            put(data, offset + 16, loop_cycles(*c).to_le_bytes());
        }
    }
    let long_offset = modes_offset + 3 * PIXELS as usize * 32;
    for (i, c) in spectrum.long.iter().enumerate() {
        if !c.amplitude.is_finite()
            || c.amplitude < 0.0
            || !c.frequency().is_finite()
            || c.frequency() == 0.0
        {
            return Err("sea long waves: invalid component".into());
        }
        for (j, v) in [
            c.wave_vector[0],
            c.wave_vector[1],
            c.amplitude,
            c.anchored_phase(anchor, time),
            c.frequency(),
        ]
        .into_iter()
        .enumerate()
        {
            put(data, long_offset + i * 32 + j * 4, (v as f32).to_le_bytes());
        }
    }
    let query_offset = long_offset + 192 * 32;
    for (i, point) in queries.iter().enumerate() {
        for axis in 0..2 {
            let relative = point[axis] - anchor[axis];
            if relative.abs() > 8192.0 {
                return Err("sea queries: local support exceeds 8192 metres".into());
            }
            put(
                data,
                query_offset + i * 8 + axis * 4,
                (relative as f32).to_le_bytes(),
            );
        }
    }
    queue.write_buffer(&sea.staging, 0, data);
    let mut offset = 0;
    for buffer in &sea.buffers[..4] {
        encoder.copy_buffer_to_buffer(&sea.staging, offset, buffer, 0, buffer.size());
        offset += buffer.size();
    }
    sea.query_count = query_count;
    Ok(())
}
fn put(data: &mut [u8], offset: usize, value: [u8; 4]) {
    data[offset..offset + 4].copy_from_slice(&value);
}
