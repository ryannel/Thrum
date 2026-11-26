use thrum_water_model::{Bed, Chart};

/// A continuous phase coordinate for a steady current with parallel,
/// section-uniform flow. Its spatial derivative is reference speed divided
/// by local speed, so translating at the reference speed carries a phase
/// at the local speed without accumulating deformation as the clock runs.
pub(super) struct CurrentTrack {
    pub(super) axis: usize,
    pub(super) reference: f64,
    sections: Vec<[f64; 2]>,
}

impl CurrentTrack {
    pub(super) fn prepare(chart: &Chart, bed: &Bed) -> Option<Self> {
        let first = chart
            .currents
            .iter()
            .zip(&chart.footprint)
            .find(|(current, inside)| **inside && current.iter().any(|&v| v != 0.0))?
            .0;
        let axis = usize::from(first[0].abs() < first[1].abs());
        let direction = first[axis].signum();
        let (origin, spacing, size) = bed.lattice();
        let mut speeds = vec![None::<f64>; size[axis]];
        for (i, (&current, &inside)) in chart.currents.iter().zip(&chart.footprint).enumerate() {
            if !inside {
                continue;
            }
            let speed = current[axis] * direction;
            if speed < 0.05 || current[1 - axis].abs() > 1e-6 {
                return None;
            }
            let section = if axis == 0 { i % size[0] } else { i / size[0] };
            let reference = speeds[section].get_or_insert(speed);
            if (speed - *reference).abs() > 1e-6 {
                return None;
            }
        }
        let first = speeds.iter().position(Option::is_some)?;
        let last = speeds.iter().rposition(Option::is_some)?;
        let active: Vec<_> = speeds[first..=last]
            .iter()
            .copied()
            .collect::<Option<_>>()?;
        // A uniform chart without joins needs no coordinate texture.
        let constant = active.iter().all(|speed| (speed - active[0]).abs() < 1e-6);
        if constant && chart.ends.is_empty() {
            return None;
        }
        let reference = active.iter().sum::<f64>() / f64::from(u32::try_from(active.len()).ok()?);
        let inlet_distance = f64::from(u32::try_from(first).ok()?) * spacing;
        let mut coordinate = origin[axis] + inlet_distance * (1.0 - reference / active[0]);
        let mut previous_speed = active[0];
        let sections = (0..size[axis])
            .map(|index| {
                let speed = active[index.clamp(first, last) - first];
                let difference = speed - previous_speed;
                // Exact travel time across a cell with linearly varying speed.
                // The constant-speed limit also avoids dividing small differences.
                let time_per_metre = if difference.abs() < previous_speed * 1e-6 {
                    speed.midpoint(previous_speed).recip()
                } else {
                    (difference / previous_speed).ln_1p() / difference
                };
                if index > 0 {
                    coordinate += reference * spacing * time_per_metre;
                }
                previous_speed = speed;
                [coordinate, speed]
            })
            .collect();
        Some(Self {
            axis,
            reference: direction * reference,
            sections,
        })
    }

    pub(super) fn at(&self, section: usize) -> [f64; 2] {
        self.sections[section]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn travel_coordinate_follows_section_speed_and_rejects_transverse_variation() {
        let bed = Bed::new([10.0, 20.0], 1.0, [4, 2], vec![0.0; 8]).unwrap();
        let speeds = [0.5, 1.0, 1.5, 1.5];
        let mut chart = Chart {
            levels: vec![1.0; 8],
            currents: speeds.iter().cycle().take(8).map(|&s| [s, 0.0]).collect(),
            footprint: vec![true; 8],
            ends: Vec::new(),
        };
        let track = CurrentTrack::prepare(&chart, &bed).unwrap();
        assert_eq!(track.axis, 0);
        assert!((track.at(0)[0] - 10.0).abs() < 1e-12);
        let displacement = track.at(1)[0] - track.at(0)[0];
        assert!((displacement / track.reference - 2.0 * 2.0_f64.ln()).abs() < 1e-12);
        let displacement = track.at(3)[0] - track.at(2)[0];
        assert!((displacement / track.reference - 1.0 / 1.5).abs() < 1e-12);
        chart.currents[5][0] = 0.9;
        assert!(CurrentTrack::prepare(&chart, &bed).is_none());
    }
}
