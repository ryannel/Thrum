# RENDER-2D plan

A game draws thousands of crisp sprites and pays for nothing 3D. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Interfaces](DESIGN.md#interfaces),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-render-2d) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing.
[The planning probe](../../evidence/RENDER-2D/planning-probe.md) says what was calculated before the design was
written; its code was thrown away on purpose, so build from the design.

## Open for the human

- Assumption: RENDER-2D waits for the V1 exit (`SHOWCASE` M8), because the owner's page puts the game after V1 ("Once V1 lands, the next goal is an original game") and this module changes the frame runtime every V1 module builds on.
- Assumption: the three changes RENDER-2D needs inside other modules (choosing a 2D scene in the frame runtime, a sprite-sheet product in the content loop, a sprite scene in the V1 game and its shells) are built by RENDER-2D's own builder after V1, under rulings in the module list. No milestone is added to `APPLE-TILE`, `CONTENT-LOOP` or `APPLE-PRESENTATION`, so no V1 plan changes while V1 is being built. The other choice adds a milestone to each of those modules.
- Assumption: "2D support in the engine" means a sprite path for top-down and side-on games: sprites in layers, a view that pans and zooms, frame animation, tint, hit flash, transparency and additive glow. 2D lights, tile-map tools, skeletal animation, a particle system, a rotating view, sprites inside a 3D scene and a game that switches between 2D and 3D are not built.
- Assumption: "thousands" is proved at 2,000, 5,000 and 10,000 moving sprites, and 10,000 is the largest crowd claimed, as in `SIM-2D`.
- Assumption: both pixel art and smooth painted art are supported, because the game's look is not chosen yet.
- Assumption: the sprite path is proved on the Mac and in Chrome, and on the iPhone simulator for function only, inside the V1 game's iOS shell. The reference crowd is drawn on the Mac and in Chrome; no iOS app exists yet for a second game, which is `CREATOR`'s work for `SWARM`.
- Assumption: "at full refresh" is judged from the engine's frame records and real-time engine capture at the built-in display's pixel size, with no frame time as a pass mark (brief decision 3). A measurement on the panel itself needs someone at the Mac and is left to `SWARM`'s close.
- Assumption: sprites blend in linear light, as the rest of the engine does. Half-transparent art looks lighter over dark than in Aseprite or Photoshop (white at half alpha over black is 188, not 128); hard-edged pixel art is unaffected. The other choice blends in display space, matches paint tools, and gives up the shared output path.
- Assumption: bright effects clip at white on an SDR display. The frame keeps values above white for an EDR display, and how they look on the XDR panel is not judged in this module.
- Assumption: a scene shows at most eight sprite sheets at once.
- Assumption: where sprites overlap in a crowd, an order that holds still matters more than one sorted exactly by height. A sprite can therefore be drawn behind a neighbour that is very slightly higher on the screen (by a few times the jitter the engine is hiding), so that packed sprites do not flicker. The other choice sorts exactly and lets neighbours that push each other swap every few ticks.
- Assumption: test art is made by the builder (generated test cards and a few simple original sprites), and the owner's judgement of a look is not an item here. The game's art is `SWARM`'s.

## M1 A game selects a 2D scene, and sprites reach the screen through the game's own frame with no sky, shadow or light work, on the Mac and in Chrome

Criteria: M1.1–M1.11 in the done-when file.

Start by reading what V1 left: the design was written before `APPLE-TILE` M10 to M12 and `APPLE-PRESENTATION` M3
to M11 were built. `FrameHost` in `crates/thrum-render/src/runtime.rs` held one prepared 3D scene when this was
planned, and the tone stage took one bound scene-linear colour with its exposure (`synthetic/display.rs`,
`synthetic_display.wgsl`). Scene-kind selection, the display-referred transfer and a scene with no reconstruction
are built in those files under ruling 1; the 3D kind stays the default and must compile to the same passes. C07's
no-reconstruction form, added for this module, is the contract for the last of them, and the refusal of a
reconstructor asked for a 2D scene belongs with `APPLE-PRESENTATION`'s registry code.
Sheets are embedded in this milestone and rows are frame rows; world rows are M2's. The sprite scene is a new `THRUM_SCENE` selection beside the ones in
`examples/v1-game/src/content.rs`; `thrum_app`'s window, `offscreen` capture and `web` shell are already generic
over the host. `scripts/render2d.py` is new and this module's.

## M2 Ten thousand sprites follow the simulation, gliding between ticks in an overlap order that holds still

Criteria: M2.1–M2.13 in the done-when file.

The 3D instance field is the precedent for extraction, changed-row upload and interpolation: `extract.rs` and
`synthetic/upload.rs`. The world rows of the sprite scene come from the V1 game's own simulation, which stays
as it is. The order is new: nothing in the 3D path keeps a list from tick to tick. Its rule and the cases a test
must cover are in [mechanism 4](DESIGN.md#4-order), and the planner's throwaway version is described in
[probe 4](../../evidence/RENDER-2D/planning-probe.md#4-does-a-kept-order-with-a-band-stop-reversals-and-what-does-it-cost).

## M3 Sprites stay crisp at every window size and pixel density, and nothing shimmers when the view scrolls

Criteria: M3.1–M3.11 in the done-when file.

The order is part of the task: M3.1 measures and retains the two coverage figures first, and M3.2 and M3.7 then
test against the retained weight figure. Mip levels and gutters for embedded sheets are made by the engine here; M4's cooker
must make the same.

## M4 An artist saves a sprite sheet and the running game shows the new art in one frame, on the Mac and in Chrome

Criteria: M4.1–M4.11 in the done-when file.

Start by reading the lease and the publication as `CONTENT-LOOP` built them in `thrum-asset-gpu`; the design
names their rules, not their API. The texture cooker is `crates/thrum-cook/src/texture.rs` and the runtime views
are in `thrum-asset`. The sheet product, pages as a lease and residency in a 2D scene are built there and in the
contribution list under ruling 2. The game connects residency to the sprite scene; `thrum-render` gains no
dependency on `thrum-asset-gpu`.

## M5 The reference crowd of 10,000 is drawn on the Mac and in Chrome, the sprite scene works on the iPhone simulator, and a game maker can start a 2D scene from the guide

Criteria: M5.1–M5.13 in the done-when file.

`SIM-2D` owns `examples/crowd-2d` and its runners, traces and retained digests; the crowd view is a new package
beside them that depends on the rule package and changes none of it. Register it in `.config/architecture.toml`.
The iOS shell is `examples/v1-game-ios`, which takes the V1 game's scene selection; `scripts/culling_sim.py`
shows how a selected scene is run on an owned simulator. The creator guide is
`docs/design/ENGINE-EXPERIENCE.md` and the ledger is `docs/design/maturity.json`.

Stop rules, for every milestone:
- A 3D scene must compile to the same passes and draw the same bytes. If it does not, stop and fix that first.
- If your change fails an exact test, a baseline or a witness another module owns, do not edit it. Write the proposal in the log and end `BLOCKED`.
- If the code V1 left cannot carry a mechanism of the design, write the difference in the log and end `BLOCKED`. Do not build a second frame host, a second output path or a loader of your own.
- An exact comparison that fails is diagnosed. Never widen it, and never rewrite a retained coverage figure to pass.
- No time stops the build (brief decision 3). Record a row that trips a tripwire as measured, note what you found in the log, and keep building. Do not thin a crowd or shrink a window to improve a row.
- Record only runs of the commit being claimed, with the load on the machine.
- Chrome and the simulator start outside the Codex sandbox. If a launch is refused, leave the item open and say so; do not substitute a Mac result.
- Lift no art or code from another game or from Broadsides.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a proof.
