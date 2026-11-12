//! Literal state-table expectations, independent of the seam's clock and GPU kernels.
use crate::effect::EffectControls;

pub(super) struct Expected {
    pub(super) trail: Vec<u32>,
    pub(super) derived: Vec<u32>,
    pub(super) kinds: Vec<Vec<Vec<u32>>>,
    tick: u64,
}

pub(super) fn seeded_kinds() -> Vec<Vec<Vec<u32>>> {
    [(4, 1), (2, 3), (6, 1)]
        .into_iter()
        .map(|(layers, mips)| {
            (0..mips)
                .map(|mip| {
                    let size = 8 >> mip;
                    (0..layers)
                        .flat_map(|z| {
                            (0..size)
                                .flat_map(move |y| (0..size).map(move |x| x + 16 * y + 256 * z))
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

impl Expected {
    pub(super) fn new() -> Self {
        Self {
            trail: vec![0; 1024],
            derived: vec![0; 1024],
            kinds: seeded_kinds(),
            tick: 10,
        }
    }
    pub(super) fn apply(
        &mut self,
        kind: &str,
        tick: u64,
        controls: EffectControls,
    ) -> Result<(), String> {
        match kind {
            "seed" | "backward" | "timeline" => {
                self.trail.fill(0);
                self.derived.fill(0);
                self.kinds = seeded_kinds();
                self.tick = tick;
            }
            "equal" | "pause" | "discard" | "after-discard-skip" | "capture" | "resize" => {}
            _ => {
                self.trail = super::advance(&self.trail, self.tick, tick, controls.source);
                self.tick = tick;
                let cache = tick.is_multiple_of(controls.cadence.max(1));
                if cache {
                    self.derived = self.trail.iter().map(|value| value * 3).collect();
                }
                let tick = u32::try_from(tick).map_err(|error| error.to_string())?;
                let seeds = seeded_kinds();
                for (index, (value, seed)) in
                    self.kinds[0][0].iter_mut().zip(&seeds[0][0]).enumerate()
                {
                    if index / 64 == (tick % 4) as usize {
                        *value = seed + tick;
                    }
                }
                if cache {
                    for (mip, seeds) in self.kinds[1].iter_mut().zip(&seeds[1]) {
                        for (value, seed) in mip.iter_mut().zip(seeds) {
                            *value = seed + tick;
                        }
                    }
                }
                for (value, seed) in self.kinds[2][0].iter_mut().zip(&seeds[2][0]) {
                    *value = seed + tick;
                }
            }
        }
        Ok(())
    }
}
