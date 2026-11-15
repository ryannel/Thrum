//! Deliberately wrong provider: the same map, limiter and breaker section, but
//! phase, amplitude and direction stay deep-water. Test fields have no short bands
//! and zero choppiness, so no other mechanism differs from the surf oracle.
use thrum_water_model::{
    BreakerSection, CutConfig, Field, Map, Provider, Sheet, SheetPoint, Surface, WaterError, cut,
    limit,
};

pub(crate) struct Untransformed<'a> {
    pub(crate) map: &'a Map,
    pub(crate) field: &'a Field,
    pub(crate) section: BreakerSection,
}

struct PlainSheet<'a> {
    provider: &'a Untransformed<'a>,
    time: f64,
}

impl Sheet for PlainSheet<'_> {
    fn forward(&self, p: [f64; 2]) -> SheetPoint {
        let row = self.provider.map.row(0, p);
        let bed = self.provider.map.sample(p).and_then(|s| s.bed);
        let depth = bed.map_or(f64::INFINITY, |h| row.level - h);
        let profile = super::support::profile();
        let limited = limit(row, depth, self.provider.field.variances(), profile);
        let mut re = 0.0;
        let mut im = 0.0;
        let mut gx = [0.0; 2];
        let mut gy = [0.0; 2];
        for c in self.provider.field.layers[0].components() {
            let amplitude = c.amplitude * row.gains[0];
            let (sin, cos) = c.anchored_phase(p, self.time).sin_cos();
            re += amplitude * cos;
            im += amplitude * sin;
            for a in 0..2 {
                gx[a] += amplitude * cos * c.wave_vector[a];
                gy[a] += amplitude * sin * c.wave_vector[a];
            }
        }
        let env = re.hypot(im);
        let b = 2.0 * env / (0.78 * depth.max(0.5));
        let section = self.provider.section.sample(im.atan2(re), b);
        let coefficient = 0.6 / 8.0_f64.sqrt();
        let draw = coefficient * limited.swash_depth / (1.0 - coefficient);
        let f = ((depth - draw) / (0.5 * limited.swash_depth - draw)).clamp(0.0, 1.0);
        let fade = f * f * (3.0 - 2.0 * f);
        let scale = if row.gains[0] > 0.0 {
            limited.gains[0] / row.gains[0]
        } else {
            0.0
        };
        let direction = std::array::from_fn::<_, 2, _>(|a| gx[a] * re + gy[a] * im);
        let norm = direction[0].hypot(direction[1]);
        SheetPoint {
            world: std::array::from_fn(|a| {
                p[a] + if norm > 1e-20 {
                    section.throw * env * scale * fade * direction[a] / norm
                } else {
                    0.0
                }
            }),
            height: scale * (re + (section.height * env - re) * fade),
        }
    }

    fn reach(&self, _: [f64; 2]) -> f64 {
        self.provider.field.layers[0]
            .components()
            .iter()
            .map(|c| c.amplitude)
            .sum::<f64>()
            * 6.0
    }
}

impl Provider for Untransformed<'_> {
    fn answer(&self, p: [f64; 2], tick: u64) -> Result<Surface, WaterError> {
        let time = tick as f64 / 60.0;
        let mut out = self.map.surface(self.field, p, time)?;
        let row = self.map.row(0, p);
        let sheet = PlainSheet {
            provider: self,
            time,
        };
        let column = cut(&sheet, p, CutConfig::default())?;
        let h = 0.0001;
        out.sample.height = row.level + column.height;
        for a in 0..2 {
            let mut lo = p;
            lo[a] -= h;
            let mut hi = p;
            hi[a] += h;
            out.sample.slope[a] = (cut(&sheet, hi, CutConfig::default())?.height
                - cut(&sheet, lo, CutConfig::default())?.height)
                / (2.0 * h);
        }
        let lo = PlainSheet {
            provider: self,
            time: time - h,
        };
        let hi = PlainSheet {
            provider: self,
            time: time + h,
        };
        let a = lo.forward(column.rest);
        let b = hi.forward(column.rest);
        out.sample.velocity = [
            (b.world[0] - a.world[0]) / (2.0 * h) + row.current[0],
            (b.height - a.height) / (2.0 * h),
            (b.world[1] - a.world[1]) / (2.0 * h) + row.current[1],
        ];
        out.sample.height_dt = (cut(&hi, p, CutConfig::default())?.height
            - cut(&lo, p, CutConfig::default())?.height)
            / (2.0 * h);
        out.wet = self
            .map
            .sample(p)
            .and_then(|s| s.bed)
            .is_none_or(|bed| out.sample.height > bed);
        Ok(out)
    }
}
