//! Two preparations compared as values, so the same bed and canopy can be shown to give the
//! same tree.
use super::Tree;

/// Floats compared bit for bit: negative zero differs from zero, as their bytes do.
pub(super) fn same_floats(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

/// Two trees are equal when every prepared value is identical, floats bit for bit.
impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        let (g, h) = (&self.ground, &other.ground);
        same_floats(&g.origin, &h.origin)
            && g.spacing.to_bits() == h.spacing.to_bits()
            && g.size == h.size
            && same_floats(&g.heights, &h.heights)
            && self.labels == other.labels
            && self.pits == other.pits
            && self.nodes.len() == other.nodes.len()
            && self.nodes.iter().zip(&other.nodes).all(|(a, b)| {
                (a.parent, a.children, a.over) == (b.parent, b.children, b.over)
                    && same_floats(&[a.sill, a.lower], &[b.sill, b.lower])
            })
            && self.curves.len() == other.curves.len()
            && self.curves.iter().zip(&other.curves).all(|(a, b)| {
                (a.start, a.len, a.base_cells) == (b.start, b.len, b.base_cells)
                    && same_floats(&[a.base_volume, a.capacity], &[b.base_volume, b.capacity])
            })
            && same_floats(&self.band, &other.band)
            && same_floats(&self.band_sum, &other.band_sum)
            && self.small == other.small
            && self.catchment == other.catchment
    }
}

#[cfg(test)]
mod tests {
    use crate::{Canopy, Ground, Profile, Tree};

    #[test]
    fn negative_zero_differs_from_zero_as_its_bytes_do() {
        let heights = vec![0.2, 0.1, 0.3, 0.0, -0.1, 0.2, 0.3, 0.2, 0.4];
        let ground = Ground::new([0.0, 0.0], 0.5, [3, 3], heights).expect("valid grid");
        let tree = Tree::prepare(&ground, &Canopy::open(), &Profile::default()).expect("tree");
        let mut flipped = tree.clone();
        assert_eq!(flipped, tree);
        let zero = flipped
            .ground
            .heights
            .iter()
            .position(|h| h.to_bits() == 0.0_f64.to_bits())
            .expect("a zero height");
        flipped.ground.heights[zero] = -0.0;
        assert_ne!(flipped, tree, "negative zero in the ground differs");
    }
}
