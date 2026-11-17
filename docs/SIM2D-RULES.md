# Portable 2D rules

Authoritative rules live in a registered rule package, using `thrum_core::profile` and
`thrum_sim2d`. The reference is `examples/crowd-2d`; platform I/O and its runner are in
`examples/crowd-2d-run`. Rendering reads published state and never owns the rules.

## Numbers, state and random draws

Use `Number` and `Vector` for coordinates and arithmetic. `Number` offers `ZERO`, `ONE`,
`from_i32`, `to_i32` (saturating, NaN becomes zero), `from_axis`, addition, subtraction,
multiplication, division, negation, comparisons, `sqrt`, `abs`, `floor`, `ceil`, `trunc`,
`round` (ties away from zero), `min`, `max` and `clamp`. Each arithmetic operation rounds
once to binary32. There is no fused multiply-add or trigonometry. Minimum and maximum
use comparisons and choose the right operand on equality, including signed zero.
Comparisons involving NaN follow IEEE rules. Do not build fast-math variants.

Convert device axes only with `Number::from_axis`: it clamps and quantizes through the
wire's signed 16-bit grid. `from_i32` rounds once. There is no arbitrary raw-float
constructor or bit inspection. Below 16,384 units, positions retain millimetre resolution
at one unit per metre.

Implement `StateEncode` for every component and resource, encoding every field in fixed
order. Fixed-width integers, booleans, profile numbers/vectors and fixed arrays have
implementations; `usize`, pointers and raw floats do not. Register components with
`register_profile`. Implement `TickGame::profile_resources` to return `Some` of the complete
staged authority resource aggregate that `apply` will commit, including unchanged values.
The default `None` is for games outside the portable profile. Exclude derived scratch;
encode every nested resource field. The engine checks its `StateEncode::is_finite` after
all callbacks and parallel merges, alongside component validation. Propagate nested
finiteness: a non-finite candidate is refused before
commit, leaving state unchanged. Use stable identities and explicit order, never addresses,
type IDs or `std::hash`. The portable digest includes complete registered authority state;
derived indexes are rebuilt and not encoded. See `Body` and `Resources` in the reference and
[the digest coverage](plans/SIM-2D/DESIGN.md#4-the-world-digest).

`random(seed: u64, tick: u64, entity: u128, stream: u32) -> Number` uses Philox2x32-10
and returns `[0,1)`. Supply the tick and stable entity identity, with a distinct stream
per purpose. There is no mutable cursor, so replay and parallel chunks do not change draws.

## Register and check

Register the manifest in `.config/architecture.toml`, with `rule_package = true` and
`allowed_workspace_dependencies`. Dependencies may be `thrum-core`, `thrum-sim2d` and
other rule packages; the outside-crate allow-list is empty. Inherit workspace package
fields and lints. Keep platform runners in separate ordinary packages.

From the repository root:

```sh
python3 scripts/sim2d_profile.py sync
python3 scripts/sim2d_profile.py check
python3 scripts/sim2d_profile.py controls
```

`sync` copies `.config/sim2d-profile/clippy.toml`, the single source, into rule packages.
`check` compares its bytes and runs profile Clippy lints, including tests. Both checks
run inside `./dev check`; the repository guard checks configuration and dependencies too.

| Refused class | Reason |
| --- | --- |
| Arithmetic on raw `f32`/`f64` | Bypasses the closed profile API |
| Raw float methods, including sine, powers, fused operations, bit access and min/max/clamp | Platform precision or observable NaN/signed-zero choices may differ |
| `HashMap`, `HashSet`, `BinaryHeap` | Iteration order is unspecified |
| Clocks | Wall time differs between peers |
| Threads | Scheduling cannot choose authoritative order |
| Process environment, including `env!` and `option_env!` | Machine-specific inputs are not recorded causes |
| Outside dependencies, build scripts, proc macros and dependency feature selection | Their operations have not been admitted |
| Suppressed lints or missing/altered profile configuration | Bypasses the checks |
| `usize`, pointers or raw floats encoded as state | Width or representation is outside canonical encoding |

Parallel rules use engine chunks and stable resolution. This is a development guard,
not a sandbox for hostile Rust. Review complete encodings and complete sort keys;
conformance samples executed behavior and cannot prove every future input.

## Queries

Rebuild `CircleIndex` from X[n], feeding circles in ascending slot order, including after
a rewind. **Cell size is part of the rule set**, identical on every peer, never a quality
setting. Circle, axis-aligned box and capsule overlaps return oversize bodies by slot,
then cells by row and column, then slots within each cell. Touching counts. Sweep a
projectile between old and new positions with a capsule to avoid tunnelling. Nearest-k
returns ascending squared centre distance with ties by slot. A neighbour query is a
circle overlap with the caller skipping itself. Table capacity and history do not affect
order. See the public `thrum_sim2d` methods and
[query design](plans/SIM-2D/DESIGN.md#5-the-index-and-its-queries).

Coordinates, radii and nearest distances are bounded by 16,777,216 units in magnitude;
cell size is at least 1/16. Invalid inputs return named errors. Wide queries enumerate
bodies with bounded work and the same order.

## Record and reproduce

The rule package's `conformance` table names its registered runner, sizes, vector file,
retained traces and nonzero witnesses. Copy `packages.crowd-2d.conformance`'s shape.
The runner depends on the rule package and implements this adapter:

- Native CLI: `vectors`, or `COUNT TICKS` with `--straight`, `--wire`, `--control`,
  `--mixed` (second variant) and `--parallel` flags.
- Wasm JSON-string exports: `vectors()`,
  `trace(count,ticks,straight,wire,control,variant,parallel)` and
  `conformance(count,ticks,straight,wire,control)`.
- Trace JSON: `bodies`, `ticks`, hexadecimal `fold`, `[tick,digest]` `checkpoints`,
  `timeline`, `wire`, `rewinds`, and registered nonzero witness fields. Wire rows report
  nonzero `changed_axes`; parallel rows report nonzero `parallel_chunks`.

The boolean variant is an adapter input whose meaning the game owns. Replay replaces
superseded digests; fold the corrected timeline in tick order. Wire input must pass through
whole batches using `thrum-network`'s public codec. The optional `control` table names a
deliberately divergent run and its size; the crowd registers platform sine as the second
consumer of the same target adapter. Engine crates name no crowd types or constants.

These commands run the shipped reference. Set the installed build-loop path for your host:

```sh
export ORCHESTRATE_SCRIPT=/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/2.3.6/skills/orchestration/build/scripts/agent_loop.py
python3 scripts/sim2d_conformance.py run --package crowd-2d --target mac-metal
python3 scripts/sim2d_conformance.py run --package crowd-2d --target chrome-webgpu
python3 scripts/sim2d_conformance.py run --package crowd-2d --target ios-sim
python3 scripts/sim2d_conformance.py control --package crowd-2d --target chrome-webgpu
```

Install pinned Rust targets as [contributing](../CONTRIBUTING.md) describes. Chrome needs
installed Chrome and matching `wasm-bindgen`; the simulator needs Xcode and iPhone 18 Pro.
The command takes each device's lock around its invocation. Simulator results prove
function only, never speed.

Record a new candidate straight timeline to a fresh path:

```sh
python3 scripts/sim2d_conformance.py record --package crowd-2d --trace crowd --output tmp/sim2d/new-crowd-candidate.txt
```

For your package, substitute its registered package and trace names. Review and promote
the candidate explicitly to its registered retained path, then run all three targets.
Recording refuses to overwrite an existing file or registered reference. Never regenerate
the reference crowd's retained files in this milestone. Record the command, commit, build,
target and load. Equality is for one build of one rule set, not across engine versions;
changes require an explicit reference decision, never a tolerance or fewer digest fields.
