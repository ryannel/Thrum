# RIVERS provenance

This is RIVERS's record of what it copied or translated, from where and under what licence, and the one to
keep current. It was first copied from `PROVENANCE.md` in RIVERS's working folder,
`.orchestrate/evidence/RIVERS/`. The `design-proof/` files it names are kept in that working folder.

## M1

Nothing in M1 is copied or translated from a third-party source, Broadsides included.

| File | Source | Licence |
|---|---|---|
| `crates/thrum-river-model/src/scheme.rs` | The HLL flux, Chen and Noelle's reconstruction and the linearised friction are translated from part 1 of `design-proof/river_proof.py`, the RIVERS planner's own reproduction in this repository, and extended to two dimensions as its part 6 does. The methods are from the published papers the [design](../plans/RIVERS/DESIGN.md#research-and-selection) cites; no code from them | Thrum's own |
| `crates/thrum-river-model/tests/energy.rs` | Part 2's section of a stream (`stream_bed`) is translated from `design-proof/river_proof.py` | Thrum's own |
| `crates/thrum-river-model/tests/closed_forms.rs` | The closed forms (Bernoulli's branches and the jump condition over SWASHES' bump, a bed integrated for a chosen depth, Manning's normal depth) follow part 1 of `design-proof/river_proof.py`; SWASHES' case is its parameters only | Thrum's own |

Everything else in `crates/thrum-river-model`, `crates/thrum-water-model/src/river.rs` and
`scripts/river_cost.py` was written for Thrum from the RIVERS design. The two dependencies the
solver adds, `blake3` (CC0-1.0 or Apache-2.0) and `rayon` (MIT or Apache-2.0), were already
registered in `.config/architecture.toml`. Nothing is GPL or share-alike.

## WP4

The river island mesh comes from Thrum's [island generator](../../examples/water/make_island.py)
with `--river`. The generator reuses the WATERS channel cut and adds the spring valley above the
lake. The river selection uses that mesh for rendering and for the bed lattice. The channel
and island variants retain their own meshes. The keyed river files are derived from the bed and
the declarations at preparation. The exposed rock placements reuse Thrum's dry-valley mesh, copied byte for byte from
`examples/starter/assets/meshes/rock.gltf` and `rock.bin`. The valley's baked shoulders
remain the ground used by the solve.
