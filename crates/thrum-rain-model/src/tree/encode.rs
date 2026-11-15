//! The prepared body as bytes, so two preparations can be compared exactly.
use super::Tree;

impl Tree {
    /// Every prepared value, little-endian, in a fixed order: the same bed and canopy
    /// always give the same bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes() + 64);
        let mut f = |v: f64| out.extend_from_slice(&v.to_le_bytes());
        f(self.ground.origin[0]);
        f(self.ground.origin[1]);
        f(self.ground.spacing);
        for &h in &self.ground.heights {
            f(h);
        }
        for &v in &self.band {
            f(v);
        }
        for &v in &self.band_sum {
            f(v);
        }
        for c in &self.curves {
            f(c.base_volume);
            f(c.capacity);
        }
        for n in &self.nodes {
            f(n.sill);
            f(n.lower);
        }
        for &size in &self.ground.size {
            out.extend_from_slice(&(size as u64).to_le_bytes());
        }
        let mut u = |v: u32| out.extend_from_slice(&v.to_le_bytes());
        for &l in &self.labels {
            u(l);
        }
        for &p in &self.pits {
            u(p);
        }
        for c in &self.curves {
            u(c.start);
            u(c.len);
            u(c.base_cells);
        }
        for n in &self.nodes {
            let [a, b] = n.children.unwrap_or([u32::MAX; 2]);
            for v in [
                n.parent.unwrap_or(u32::MAX),
                a,
                b,
                n.over.unwrap_or(u32::MAX),
            ] {
                u(v);
            }
        }
        for s in &self.small {
            u(s.hollow);
            u(u32::try_from(s.across).unwrap_or(u32::MAX));
        }
        out.extend_from_slice(&self.catchment.to_bytes());
        out
    }
}
