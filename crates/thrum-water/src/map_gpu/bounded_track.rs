use thrum_water_model::Chart;

/// Reference velocities for the bounded departure map of a general chart.
/// Each component averages its own axis within a contiguous run of same-sign
/// current. This keeps the residual bounded along the run without making
/// different lanes share one reference velocity.
pub(super) struct BoundedTrack {
    cells: Vec<[f32; 4]>,
    pub(super) period_limit: f32,
}

impl BoundedTrack {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "prepared chart reference velocities use GPU metres"
    )]
    pub(super) fn prepare(charts: &[&Chart], size: [usize; 2], spacing: f64) -> Self {
        let mut currents = vec![[0.0; 2]; size[0] * size[1]];
        let mut gradient = [0.0_f64; 2];
        for chart in charts {
            for (i, (&current, &inside)) in chart.currents.iter().zip(&chart.footprint).enumerate()
            {
                if inside {
                    currents[i] = current;
                }
                for (axis, stride) in [1, size[0]].into_iter().enumerate() {
                    let cell = [i % size[0], i / size[0]];
                    if cell[axis] + 1 < size[axis] {
                        let next = chart.currents[i + stride];
                        let difference = (next[0] - current[0]).hypot(next[1] - current[1]);
                        gradient[axis] = gradient[axis].max(difference / spacing);
                    }
                }
            }
        }
        let mut cells = vec![[0.0; 4]; currents.len()];
        for (axis, stride) in [1, size[0]].into_iter().enumerate() {
            for lane in 0..size[1 - axis] {
                let start = if axis == 0 { lane * size[0] } else { lane };
                let index = |section: usize| start + section * stride;
                let speed = |section| currents[index(section)][axis];
                let mut first = 0;
                while first < size[axis] {
                    let initial = speed(first);
                    if initial == 0.0 {
                        first += 1;
                        continue;
                    }
                    let mut sum = initial;
                    let mut end = first + 1;
                    while end < size[axis]
                        && speed(end) != 0.0
                        && speed(end).is_sign_positive() == initial.is_sign_positive()
                    {
                        sum += speed(end);
                        end += 1;
                    }
                    let length = u32::try_from(end - first).expect("GPU lattice");
                    let mean = (sum / f64::from(length)) as f32;
                    for section in first..end {
                        cells[index(section)][axis] = mean;
                    }
                    first = end;
                }
            }
        }
        for (reference, current) in cells.iter_mut().zip(currents) {
            reference[2..].copy_from_slice(&current.map(|speed| speed as f32));
        }
        // Bound the residual offset before its spatial gradient can fold the
        // departure map. The limit is shared by all owners of the track.
        let period_limit = gradient[0]
            .hypot(gradient[1])
            .recip()
            .min(f64::from(f32::MAX)) as f32;
        Self {
            cells,
            period_limit,
        }
    }

    pub(super) fn cells(&self) -> &[[f32; 4]] {
        &self.cells
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped chart lattice in GPU metres"
    )]
    pub(super) fn sample(&self, world: [f64; 2], bed: &thrum_water_model::Bed) -> [f32; 4] {
        let (origin, spacing, size) = bed.lattice();
        let grid = [0, 1].map(|axis| {
            let last = u32::try_from(size[axis] - 1).expect("GPU lattice");
            ((world[axis] - origin[axis]) / spacing).clamp(0.0, f64::from(last))
        });
        let [x, z] = [0, 1].map(|axis| (grid[axis].floor() as usize).min(size[axis] - 2));
        let fraction =
            |axis, cell| (grid[axis] - f64::from(u32::try_from(cell).expect("GPU lattice"))) as f32;
        let u = fraction(0, x);
        let v = fraction(1, z);
        let i = z * size[0] + x;
        let lerp = |a: f32, b: f32, w: f32| (b - a).mul_add(w, a);
        std::array::from_fn(|axis| {
            let near = lerp(self.cells[i][axis], self.cells[i + 1][axis], u);
            let far = lerp(
                self.cells[i + size[0]][axis],
                self.cells[i + size[0] + 1][axis],
                u,
            );
            lerp(near, far, v)
        })
    }
}

pub(super) fn uniform_current(chart: &Chart) -> bool {
    let mut currents = chart
        .currents
        .iter()
        .zip(&chart.footprint)
        .filter_map(|(current, &inside)| inside.then_some(current.map(f64::to_bits)));
    currents
        .next()
        .is_none_or(|first| currents.all(|current| current == first))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_follows_separate_runs_in_both_directions() {
        let size = [5, 2];
        let bed = thrum_water_model::Bed::new([0.0, 0.0], 2.0, size, vec![0.0; 10]).unwrap();
        let currents = [
            [0.5, 1.0],
            [1.5, 0.0],
            [0.0, 0.0],
            [-1.5, 0.0],
            [-2.5, 0.0],
            [3.0, 2.0],
            [3.0, 0.0],
            [0.0, 0.0],
            [-3.0, 0.0],
            [-3.0, 0.0],
        ];
        let chart = Chart {
            levels: vec![1.0; 10],
            currents: currents.to_vec(),
            footprint: vec![true; 10],
            ends: Vec::new(),
        };
        let track = BoundedTrack::prepare(&[&chart], size, 2.0);
        assert_eq!(track.sample([0.0, 0.0], &bed)[..2], [1.0, 1.5]);
        assert_eq!(track.sample([2.0, 1.0], &bed)[..2], [2.0, 0.0]);
        assert_eq!(track.sample([4.0, 1.0], &bed)[..2], [0.0, 0.0]);
        assert_eq!(track.sample([8.0, 1.0], &bed)[..2], [-2.5, 0.0]);
        assert!(track.period_limit > 0.0 && track.period_limit < 2.0);
    }
}
