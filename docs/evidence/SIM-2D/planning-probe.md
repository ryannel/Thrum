# SIM-2D planning probe, 13 November 2026

A throwaway measurement the planner ran before writing [the design](../../plans/SIM-2D/DESIGN.md). It answers three
questions the design had to settle and that analysis alone could not. It is **not** engine code and proves nothing
about the engine: milestone M1 repeats every row through `thrum_core::sim`. The probe's source stayed in the
planner's scratch folder and is not kept, so nothing here is a baseline.

## What ran

One self-contained Rust file, no dependencies, toolchain 1.99.1, on this M3 Max while other agents were building
(timings are contended and single-threaded).

- **Crowd.** Four scripted players; 2,000, 5,000 or 10,000 enemies that chase the nearest player and push apart
  from neighbours; projectiles fired in fans that hit the lowest-order overlapping enemy; enemies die at zero
  integer health; new enemies spawn on a ring to hold the count. Slots are reused through a free list.
- **Arithmetic.** `f32` with add, subtract, multiply, divide, square root and comparisons only. Random numbers from
  a stateless integer mix of seed, tick, stream and entity.
- **Index.** A counting-sort grid over hashed cells, rebuilt every tick; circle-overlap queries.
- **Digest.** A 64-bit integer mix over tick, free list, generations and the bits of every live value, taken every
  tick and folded into one value for the run.
- **3,600 ticks** per run: about 27,000 to 35,000 spawns, 25,000 despawns and 19,000 hits.

Three deliberately broken variants ran beside it: the platform's `sin`/`cos` in steering, `powf` in steering, and
`f32::max`/`min` as a clamp.

## 1. Does restricted `f32` give the same bytes on the three targets?

Yes, in this probe. The folded digest of all 3,600 ticks was equal on every row, at all three sizes and both query
orders:

| Row | Build | Clean rules | `sin` variant | `powf` variant | `max`/`min` variant |
|---|---|---|---|---|---|
| Mac, arm64 | release | reference | reference | reference | reference |
| Mac, arm64 | debug (20 of 24 runs; the last four were stopped by a time limit) | equal | equal | equal | equal |
| iPhone 18 Pro simulator, iOS 27.2, `simctl spawn` | release | equal | **equal** | **equal** | equal |
| Wasm in Node 24.16 (V8) | release | equal | differs | differs | equal |
| Wasm in headless Chrome 157 | release | equal | differs | differs | equal |

Two things follow for the design:

- The simulator shares the Mac's processor and maths library, so it cannot tell a rule that calls the platform's
  `sin` from one that does not. The Mac-to-Wasm comparison is the row that discriminates.
- The `max`/`min` variant never diverged, though Rust documents that either zero may come back when the inputs are
  `+0.0` and `-0.0`. A run cannot be relied on to catch that class; it has to be refused before it runs.

## 2. What does the crowd cost?

Median time per tick, single thread, rules and index and structure together. The crowd here packs about three
deep (each enemy overlaps about 14 others at 10,000), which is denser than a shipped game lets it get.

| Bodies | Mac release | Wasm, Node | Wasm, Chrome 157 | Index build (Mac) | Digest (Mac) |
|---|---|---|---|---|---|
| 2,000 | 0.57 ms | 0.64 ms | 0.65 ms | 0.011 ms | 0.043 ms |
| 5,000 | 2.1 ms | 2.2 ms | 2.3 ms | 0.024 ms | 0.093 ms |
| 10,000 | 5.0 ms | 5.6 to 5.9 ms | 5.6 ms | 0.045 ms | 0.18 ms |

Almost all of it is the neighbour pass: about 144,000 accepted neighbours a tick at 10,000 bodies. Copying
positions into cell order for the queries changed no digest and saved no time, so the cost is arithmetic on
candidate pairs, not memory traffic. Replaying a tick costs a tick, so an eight-tick replay of this crowd at 10,000
would be about 40 ms.

## 3. What does it cost to hand neighbours back in slot order?

Sorting each query's results into ascending slot order, against handing them back in the grid's own order (cell
row, cell column, then slot):

| Bodies | Grid order | Slot order | Extra |
|---|---|---|---|
| 2,000 | 0.57 ms | 0.71 ms | 25% |
| 5,000 | 2.1 ms | 2.8 ms | 29% |
| 10,000 | 5.0 ms | 6.8 ms | 37% |

Both orders gave equal digests across the targets. The design takes the grid's order.

## Also checked

- `xcrun simctl spawn <device> <binary>` runs a plain arm64 executable built for `aarch64-apple-ios-sim` in the
  booted simulator, with no app bundle.
- Clippy's `disallowed-methods` refuses `f32::sin`, `f32::powf`, `f32::max` and `f32::mul_add` by name, including
  through a function pointer; `disallowed-types` refuses `HashMap`; `float_arithmetic` refuses `x * 2.0` on a raw
  float.
