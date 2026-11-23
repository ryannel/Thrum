# REVIEW-ENGINE plan

The 14 November review's findings outside the renderer and the shells are fixed: the simulation, the sea, cooked
assets, the cooker and the iPhone app's device request. [Design](DESIGN.md) · [done-when file](DONE.json), which
owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. Every crate here is a covered path.

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: a game can no longer fill its own published page. The engine fills it, as the G2S decision says, and no planned game needs otherwise.
- Assumption: the fix for Metal drawing every meshlet one by one (M71) is not built here. `MESH-SHADING` builds it for cooked placements beside V1, and its record decides whether entities follow. It is the largest cost finding this plan leaves alone.
- Assumption: the cooker's mip filter keeps its two small quality defects (odd sizes, colour bleeding at alpha edges), because fixing them changes cooked bytes and would move every exact reference image.

## M1 The iPhone app gets compressed textures and enough storage buffers for the sea, and a rewound game no longer carries a patch that never ran

The iPhone request is in `examples/v1-game-ios/src/ios/render.rs` (`request_device`), used for preparation and for
recovery; the Mac window's is `crates/thrum-app/src/app/device.rs`. Make the request reachable from a test on the
Mac. The publication code is `crates/thrum-core/src/sim/runtime.rs` and `sim/history.rs`, the page's patch is in
`sim/publication.rs`, and the game's dead closure is in `examples/v1-game/src/simulation/history.rs`. `SIM-2D` M1
may be working in `thrum-core` at the same time.

## M2 The sea names its buffers and passes, shares its records with its shaders under a layout test, and uploads its spectrum only when the spectrum changes

`thrum-sea` is `WATERS`' to extend from its M2, which starts when `EFFECT-GRAPH` M3 and `ATMOSPHERE` M9 are Done. If
it has started, integrate on what it has landed. The named-field shape to copy is `thrum-asset-gpu`'s, and the
layout test to copy is at the end of `crates/thrum-render/src/synthetic/view.rs`. Record a frame's CPU cost before
you change the upload.

## M3 A cooked mesh reaches the GPU straight from its mapped file, and the cooker's edit-to-visible test measures the engine's own loader

`COOKED-DRAW`'s equivalence rests on meshlet indices being contiguous in meshlet order; the cooker already writes
them so. `scripts/check_g5_tripwires.sh` is pinned by `CONTENT-LOOP` M3's check: keep its name and what it reports.
Removing `examples/asset-loop` changes `.config/architecture.toml`.

## M4 A failure in the simulation, the sea, cooked assets, the cooker or the network codecs can be matched and printed: every public error there is a typed enum

The model is `crates/thrum-asset/src/error.rs`. Callers in the renderer, the shells and the game follow each
changed signature; convert at their edge and leave their own errors to `REVIEW-RENDER` and `HOST`.

## M5 Replay stops copying undo it never uses, the cooker decodes each texture once, and the sea transforms its cascades in half the passes its design allows for

Take each paired record at the commit you claim and say what else the machine was doing; other builders share it.

## M6 Corrected replay stays fast and the 100,000-entity run stays covered

This is the V1 game’s G2S replay test, which exercises the simulation and sparse undo paths maintained
here. The fast `parallel_movers_match_the_straight_corrected_run_in_a_small_world` case in `simulation` carries
the rollback and corrected-replay comparisons described in the design. Move the existing 100,000-entity
test whole to the long tier with an ignore reason beginning with `long:`. List its existing binary in
`.config/long-tests.txt` (release with overflow checks) with the fault that would reach main before the
long run catches it. Raise the `main-tier` rule version as the settings require. Run the original case
in that build on the branch, then take the covering `long-tests` record on main. Verify that the normal
runner skips it and the long runner executes it once.

Record the disposition of the original test and the coverage that replaces it in the fast tier. Use the
supplied workspace run for the starting time. Record both new per-test times from nextest on the Mac,
with the tested revision and the activity on the machine. Use M6.1's command for the isolated
measurement. For the first-attempt result, run a focused selection including the compact case with
the same build and profile alongside ordinary builder work.

Starting time from the passing `747909d1c` workspace check:

- 7.965 s — `v1-game::simulation parallel_movers_match_the_straight_corrected_run_after_churn_and_refusal`
