use crate::AmbientError;
use crate::{
    Ambient, MAX_QUERIES, PIXELS, UploadBytes, records,
    spec::{BufferKey, MAX_LONG},
};
use bytemuck::Zeroable as _;
use std::f64::consts::{PI, TAU};
use thrum_sea_model::{LOOP_SECONDS, Spectrum, loop_cycles};

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "loop time is reduced to an unsigned fixed-point fraction"
)]
pub(super) fn loop_fraction(time: f64) -> u32 {
    (time.rem_euclid(LOOP_SECONDS) / LOOP_SECONDS * 4_294_967_296.0) as u32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "validated modes have bounded lattice indices and round to binary32 on the GPU"
)]
pub(super) fn lattice_mode(
    component: thrum_sea_model::Component,
    length: f64,
    cycles: u32,
) -> (usize, records::Bin) {
    let lattice = component.wave_vector.map(|k| {
        ((k * length / TAU).round() as i32).rem_euclid(crate::TRANSFORM_SIZE.cast_signed()) as usize
    });
    let index = lattice[1] * crate::TRANSFORM_SIZE as usize + lattice[0];
    let record = records::Bin {
        physical: [
            component.wave_vector[0] as f32,
            component.wave_vector[1] as f32,
            component.amplitude as f32,
            component.phase as f32,
        ],
        clock: [cycles, 0, 0, 0],
    };
    (index, record)
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "physical values reduce to GPU-relative f32; bounded loop/lattice values fit integers"
)]
pub(super) fn frame(
    sea: &mut Ambient,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    spectrum: &Spectrum,
    time: f64,
    anchor: [f64; 2],
    queries: &[[f64; 2]],
) -> Result<(), AmbientError> {
    let changed = !sea.modes_initialized
        || spectrum
            .cascades
            .iter()
            .zip(&sea.cached_cascades)
            .zip(sea.cached_lengths)
            .any(|((band, cached), length)| {
                band.length().to_bits() != length.to_bits() || band.components() != cached
            });
    validate(sea, spectrum, time, anchor, queries, changed)?;
    if changed {
        sea.modes.fill(records::Bin::zeroed());
        for (band, cascade) in spectrum.cascades.iter().enumerate() {
            for c in cascade.components() {
                let (index, record) = lattice_mode(*c, cascade.length(), loop_cycles(*c));
                sea.modes[band * PIXELS as usize + index] = record;
            }
        }
    }
    let mut params = records::Params::zeroed();
    params.time = [
        loop_fraction(time),
        u32::try_from(spectrum.long.len()).expect("validated count"),
        u32::try_from(queries.len()).expect("validated count"),
        0,
    ];
    params.physical[0] = sea.profile.choppiness as f32;
    for (length, cascade) in params.lengths.iter_mut().zip(&spectrum.cascades) {
        *length = cascade.length() as f32;
    }
    for (i, level) in sea.levels.iter_mut().enumerate() {
        level.texel = f64::from(1_u32 << i);
        level.max_wavenumber = PI / level.texel;
        level.origin = anchor.map(|v| (v / level.texel).floor() * level.texel - 64.0 * level.texel);
        params.levels[i] = [
            level.origin[0] - anchor[0],
            level.origin[1] - anchor[1],
            level.texel,
            level.max_wavenumber,
        ]
        .map(|v| v as f32);
    }
    let mut long = [records::Long::zeroed(); MAX_LONG];
    for (record, c) in long.iter_mut().zip(&spectrum.long) {
        *record = records::Long {
            physical: [
                c.wave_vector[0],
                c.wave_vector[1],
                c.amplitude,
                c.anchored_phase(anchor, time),
            ]
            .map(|v| v as f32),
            rate: [c.frequency() as f32, 0.0, 0.0, 0.0],
        };
    }
    sea.upload.clear();
    sea.upload.extend_from_slice(bytemuck::bytes_of(&params));
    sea.upload
        .extend_from_slice(bytemuck::cast_slice(&long[..spectrum.long.len()]));
    for point in queries {
        let relative = records::Query {
            point: [(point[0] - anchor[0]) as f32, (point[1] - anchor[1]) as f32],
        };
        sea.upload.extend_from_slice(bytemuck::bytes_of(&relative));
    }
    // Modes have their own immutable-to-compute allocation. Queue writes survive a discarded
    // encoder, so caching one here cannot lose a needed upload when that frame is dropped.
    let mode_bytes = if changed {
        let bytes = bytemuck::cast_slice(&sea.modes);
        queue.write_buffer(&sea.buffers[&BufferKey::Modes], 0, bytes);
        for (cached, band) in sea.cached_cascades.iter_mut().zip(&spectrum.cascades) {
            cached.clear();
            cached.extend_from_slice(band.components());
        }
        sea.cached_lengths = spectrum
            .cascades
            .each_ref()
            .map(thrum_sea_model::Cascade::length);
        sea.modes_initialized = true;
        bytes.len() as u64
    } else {
        0
    };
    let stats = UploadBytes {
        total: mode_bytes + sea.upload.len() as u64,
        parameters: size_of::<records::Params>() as u64,
        modes: mode_bytes,
        long: (spectrum.long.len() * size_of::<records::Long>()) as u64,
        queries: (queries.len() * size_of::<records::Query>()) as u64,
    };
    queue.write_buffer(&sea.buffers[&BufferKey::UploadStaging], 0, &sea.upload);
    let mut offset = 0;
    for (key, bytes) in [
        (BufferKey::Params, stats.parameters),
        (BufferKey::Long, stats.long),
        (BufferKey::Queries, stats.queries),
    ] {
        if bytes > 0 {
            encoder.copy_buffer_to_buffer(
                &sea.buffers[&BufferKey::UploadStaging],
                offset,
                &sea.buffers[&key],
                0,
                bytes,
            );
        }
        offset += bytes;
    }
    sea.last_upload = stats;
    Ok(())
}

fn validate(
    sea: &Ambient,
    spectrum: &Spectrum,
    time: f64,
    anchor: [f64; 2],
    queries: &[[f64; 2]],
    changed: bool,
) -> Result<(), AmbientError> {
    if !time.is_finite()
        || anchor.iter().any(|v| !v.is_finite() || v.abs() > 1e12)
        || queries.len() > MAX_QUERIES as usize
        || queries.iter().flatten().any(|v| !v.is_finite())
        || spectrum.long.len() > MAX_LONG
    {
        return Err(AmbientError::InvalidUpload);
    }
    let steepness = thrum_sea_model::contraction_bound(&spectrum.long, sea.profile.choppiness)
        .map_err(AmbientError::LongWaves)?;
    if !steepness.is_finite() || steepness >= crate::LONG_WAVE_CONTRACTION_LIMIT {
        return Err(AmbientError::NonContractive);
    }
    for band in spectrum.cascades.iter().filter(|_| changed) {
        if !band.length().is_finite()
            || band.length() <= 0.0
            || band.components().iter().any(|c| {
                !c.amplitude.is_finite()
                    || !c.phase.is_finite()
                    || c.wave_vector.iter().any(|v| !v.is_finite())
            })
        {
            return Err(AmbientError::InvalidCascade);
        }
        for c in band.components() {
            let lattice = c.wave_vector.map(|k| (k * band.length() / TAU).round());
            if lattice
                .iter()
                .any(|v| v.abs() >= f64::from(crate::TRANSFORM_SIZE / 2))
                || lattice == [0.0, 0.0]
            {
                return Err(AmbientError::OutsideLattice);
            }
        }
    }
    if spectrum.long.iter().any(|c| {
        !c.amplitude.is_finite()
            || c.amplitude < 0.0
            || !c.frequency().is_finite()
            || c.frequency() == 0.0
    }) {
        return Err(AmbientError::InvalidLongWave);
    }
    if queries
        .iter()
        .any(|p| (p[0] - anchor[0]).abs() > 8192.0 || (p[1] - anchor[1]).abs() > 8192.0)
    {
        return Err(AmbientError::QueryOutsideSupport);
    }
    Ok(())
}
