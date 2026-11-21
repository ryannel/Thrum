//! M1.1: the tree of hollows, against brute force on a bed the test makes. Pits, paths and
//! passes are worked out here from the prepared bed alone; the model gives only its labels,
//! its hollows' pits and its nodes, which are what is checked.
#![expect(
    clippy::cast_possible_truncation,
    reason = "test counts of cells, hollows and nodes are far below u32::MAX"
)]
pub mod support;
use std::collections::{BTreeSet, VecDeque};
use support::{Lcg, design_bed, ground, minimax_from, pits, prepare};
use thrum_rain_model::{Canopy, Ground, Profile, Tree};
use thrum_water_model::NO_HOLLOW;

/// The design's bed with a pothole dug three cells across, 15 mm deep.
fn bed() -> Ground {
    let g = design_bed();
    let n = g.size()[0];
    let mut heights = g.heights().to_vec();
    for z in 50..53 {
        for x in 8..11 {
            heights[z * n + x] -= 0.015;
        }
    }
    Ground::new(g.origin(), g.spacing(), g.size(), heights).expect("valid bed")
}

/// The lowest level at which water over a pit leaves it: the least bottleneck from the pit
/// to the rectangle's edge or to any other pit given.
fn spill(g: &Ground, pit: usize, others: &[usize]) -> f64 {
    let pass = minimax_from(g, pit);
    (0..pass.len())
        .filter(|&i| g.edge(i) || (others.contains(&i) && i != pit))
        .map(|i| pass[i])
        .fold(f64::INFINITY, f64::min)
}

fn leaves_under(tree: &Tree, node: u32) -> Vec<u32> {
    match tree.node(node).children {
        None => vec![node],
        Some([a, b]) => [leaves_under(tree, a), leaves_under(tree, b)].concat(),
    }
}

fn root_of(tree: &Tree, mut node: u32) -> u32 {
    while let Some(p) = tree.node(node).parent {
        node = p;
    }
    node
}

#[test]
fn every_pit_is_a_hollow_and_shallow_ones_are_filled() {
    let original = bed();
    let tree = prepare(&original);
    let g = tree.ground();
    let least = Profile::default().least_depth;
    let hollow_pits: BTreeSet<usize> = (0..tree.hollow_count())
        .map(|h| tree.pit(h as u32))
        .collect();
    // Pits of the prepared bed, found by brute force, are exactly the hollows' pits.
    let found: BTreeSet<usize> = pits(g).into_iter().collect();
    assert_eq!(found, hollow_pits);
    assert!(tree.hollow_count() >= 4, "{} hollows", tree.hollow_count());
    // No hollow left is shallower than the least depth.
    let all: Vec<usize> = hollow_pits.iter().copied().collect();
    for &p in &all {
        let depth = spill(g, p, &all) - g.heights()[p];
        assert!(depth >= least, "hollow at {p} is {depth} m deep");
    }
    // The original bed's shallow pits were filled: raised to at least their own sill, and
    // the bed was only ever raised.
    let original_pits = pits(&original);
    let mut filled = 0;
    for &p in &original_pits {
        let sill = spill(&original, p, &original_pits);
        if sill - original.heights()[p] < least && !hollow_pits.contains(&p) {
            filled += 1;
            assert!(
                g.heights()[p] >= sill,
                "filled pit {p} not raised to its sill"
            );
        }
    }
    assert!(filled > 10, "the relief's small pits are filled: {filled}");
    for (a, b) in g.heights().iter().zip(original.heights()) {
        assert!(a >= b);
    }
}

#[test]
fn pits_and_depths_on_seeded_beds() {
    let least = Profile::default().least_depth;
    for seed in 0..12 {
        let tree = prepare(&support::random_bed(seed, 40));
        let g = tree.ground();
        let hollow_pits: Vec<usize> = (0..tree.hollow_count())
            .map(|h| tree.pit(h as u32))
            .collect();
        let found: Vec<usize> = pits(g);
        assert_eq!(
            found.iter().collect::<BTreeSet<_>>(),
            hollow_pits.iter().collect::<BTreeSet<_>>()
        );
        for &p in &hollow_pits {
            assert!(
                spill(g, p, &hollow_pits) - g.heights()[p] >= least,
                "seed {seed}"
            );
        }
    }
}

#[test]
fn each_cell_is_reached_from_its_pit_without_descending() {
    reached(&bed());
    for seed in 0..12 {
        reached(&support::random_bed(seed, 40));
    }
}

fn reached(bed: &Ground) {
    let tree = prepare(bed);
    let g = tree.ground();
    let z = g.heights();
    let labels = tree.labels();
    for h in 0..tree.hollow_count() {
        let pit = tree.pit(h as u32);
        let mut seen = vec![false; z.len()];
        seen[pit] = true;
        let mut q = VecDeque::from([pit]);
        while let Some(i) = q.pop_front() {
            for j in g.neighbours(i) {
                if !seen[j] && labels[j] as usize == h && z[j] >= z[i] {
                    seen[j] = true;
                    q.push_back(j);
                }
            }
        }
        let missed = (0..z.len())
            .filter(|&i| labels[i] as usize == h && !seen[i])
            .count();
        assert_eq!(
            missed, 0,
            "hollow {h}: {missed} cells not reached without descending"
        );
    }
    // Every pit's own cell carries its hollow.
    for h in 0..tree.hollow_count() {
        assert_eq!(labels[tree.pit(h as u32)] as usize, h);
    }
    assert!(
        labels.contains(&NO_HOLLOW),
        "cells by the edge drain over it"
    );
}

#[test]
fn each_sill_is_the_lowest_pass() {
    assert!(sills(&bed()) >= 2, "the bed's hollows join");
    // And on seeded beds with more hollows.
    let joined: usize = (0..12)
        .map(|seed| sills(&support::random_bed(seed, 40)))
        .sum();
    assert!(joined >= 30, "{joined} joined nodes over the seeded beds");
}

/// Check every node's sill against brute force; returns how many joined nodes there were.
fn sills(bed: &Ground) -> usize {
    let tree = prepare(bed);
    let g = tree.ground();
    let mut joined = 0;
    for k in 0..tree.node_count() as u32 {
        let node = tree.node(k);
        if let Some([a, b]) = node.children {
            // The lowest pass between any pit of one side and any of the other.
            let (la, lb) = (leaves_under(&tree, a), leaves_under(&tree, b));
            let pass = minimax_from(g, tree.pit(la[0]));
            let lowest = lb
                .iter()
                .map(|&l| pass[tree.pit(l)])
                .fold(f64::INFINITY, f64::min);
            assert_eq!(tree.node(a).sill, lowest, "node {k}");
            assert_eq!(tree.node(b).sill, lowest, "node {k}");
            joined += 1;
        }
        if node.parent.is_none() {
            // A root leaves where its lowest pass to the edge or to another tree's pit is.
            let root = k;
            let own = leaves_under(&tree, root);
            let others: Vec<usize> = (0..tree.hollow_count() as u32)
                .filter(|&h| root_of(&tree, h) != root)
                .map(|h| tree.pit(h))
                .collect();
            let lowest = own
                .iter()
                .map(|&l| {
                    let pass = minimax_from(g, tree.pit(l));
                    (0..pass.len())
                        .filter(|&i| g.edge(i) || others.contains(&i))
                        .map(|i| pass[i])
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min);
            assert_eq!(node.sill, lowest, "root {k}");
        }
    }
    joined
}

#[test]
fn small_hollows_are_reported() {
    let tree = prepare(&bed());
    let g = tree.ground();
    let w = g.size()[0];
    let mut expected = BTreeSet::new();
    for h in 0..tree.hollow_count() {
        let sill = tree.node(h as u32).sill;
        let cells: Vec<usize> = (0..g.heights().len())
            .filter(|&i| tree.labels()[i] as usize == h && g.heights()[i] < sill)
            .collect();
        let span = |f: &dyn Fn(usize) -> usize| {
            cells.iter().map(|&i| f(i)).max().expect("valid test case")
                - cells.iter().map(|&i| f(i)).min().expect("valid test case")
                + 1
        };
        let across = span(&|i| i % w).min(span(&|i| i / w));
        if across < 4 {
            expected.insert((h as u32, across));
        }
    }
    let reported: BTreeSet<(u32, usize)> = tree
        .small_hollows()
        .iter()
        .map(|s| (s.hollow, s.across))
        .collect();
    assert_eq!(reported, expected);
    let pothole = tree.labels()[51 * w + 9];
    assert!(
        reported.iter().any(|&(h, a)| h == pothole && a == 3),
        "the pothole is reported"
    );
}

#[test]
fn the_same_bed_gives_the_same_tree() {
    let a = prepare(&bed());
    let b = prepare(&bed());
    assert_eq!(a, b, "two preparations of one bed are equal");
    // And a changed bed gives another tree.
    let mut r = Lcg(3);
    let g = bed();
    let heights = g.heights().iter().map(|h| h + r.uniform() * 1e-6).collect();
    let other = Ground::new(g.origin(), g.spacing(), g.size(), heights).expect("valid test case");
    assert_ne!(a, prepare(&other), "a changed bed gives another tree");
}

#[test]
fn a_body_over_the_profile_is_refused() {
    let g = ground([40, 40], 0.05, |x, y| {
        ((x * 9.0).sin() * (y * 9.0).sin()) * 0.05
    });
    let profile = Profile {
        max_hollows: 3,
        ..Profile::default()
    };
    let err = Tree::prepare(&g, &Canopy::open(), &profile).unwrap_err();
    assert!(err.to_string().contains("hollows"), "{err}");
    let profile = Profile {
        max_cells: 100,
        ..Profile::default()
    };
    let err = Tree::prepare(&g, &Canopy::open(), &profile).unwrap_err();
    assert!(err.to_string().contains("cells"), "{err}");
}
