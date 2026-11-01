# Thrum work list

The first goal is a small game that follows one clear path: **a player acts, the game checks the action, the world changes, and players see the result**. Local and multiplayer play use the same game rules.

This is the central to-do list. A checked box means that particular piece is done; it does not mean the larger stage is finished. The [detailed construction plan](design/CONSTRUCTION-PLAN.md) gives contributors the full sequence and technical requirements.

## Done

- [x] Set up the repository, contributor commands, linting, and checks that guard the project structure.
- [x] Build and measure a small game without graphics. A player can collect an item and open a door. The same rules work for local play and for a main world with a second, in-memory player view. Tests cover invalid, late, and repeated actions. See the [design](design/I1-HEADLESS.md) and [test and measurement record](evidence/i1-headless/QUALIFICATION.md).

## Next: finish the foundation

The small game above proves an important path, but the full foundation is still in progress. These are the remaining boxes:

- [x] Give the initial headless game a reliable simulation clock. It uses 60 world updates per second by default, with a configurable rate when a game needs one. Pause/resume, bounded catch-up and explicit overload preserve committed updates without silently skipping or applying one twice.
  - [x] Build and test the bounded headless clock, including high-refresh presentation callbacks, pause/resume, blocked publication, and explicit queue overload.
  - [x] Connect the initial authority/replica stages to a native headless host callback; measure slow-callback handling and the complete input-to-replica path. See the [host qualification](evidence/i1-headless/HOST.md).
- [ ] Limit the total memory used by published copies of world state, even when a reader is slow or the world grows. Show that memory becomes available again when readers release their copies.
  - [x] Identify why the existing snapshot-count limit cannot enforce a byte limit; select a precommit admission design and remove a redundant allocation when new world pages are published. See the [publication-memory design](design/I1-HEADLESS.md#publication-memory-admission-gate) and [growth measurement](evidence/i1-headless/PUBLICATION-MEMORY.md).
  - [x] Build and test the shared reservation arithmetic and select a faster private page layout against the 4,096 and 16,384-object workloads. The [layout evidence](evidence/i1-headless/PAGE-LAYOUT.md) records the speed and memory tradeoff; these pieces are not yet wired into authority admission.
  - [x] Add atomic validation for prepared world edits, including checked handles and net-zero structural growth; keep the common single-edit path free of validation-collection allocation. See the [prepared edit proof](evidence/i1-headless/PREPARED-EDITS.md).
  - [x] Add a pool-charged immutable byte payload that copies only after admission, can spend an existing tick reservation, and releases its charge after its final shared owner. Prove its pending-to-completed input-history lifetime in the headless game. See the [payload proof](evidence/i1-headless/PUBLICATION-BYTES.md); general input, world and replica byte admission remains open.
  - [x] Attach unique lifetime charges to immutable world pages and roots, including shared old readers and two worlds using one pool; measure the charged path across scale and churn. See the [page-owner proof](evidence/i1-headless/CHARGED-PAGES.md). This is not yet connected to precommit authority admission.
  - [ ] Connect the shared byte pool to bounded prepared writes and every published allocation; prove rejection before mutation, unique accounting across worlds and readers, and release after the last owner drops.
- [x] Run two independently controlled players in one in-memory match. Check simultaneous actions, permissions, late and repeated messages, and what each player sees when joining or falling behind. See the [test and cost record](evidence/i1-headless/TWO-PEER.md).
- [ ] Add basic iPhone and Android app shells. They should start with bundled game content, accept input, pause and resume, and recover when the display surface is recreated. Test on the chosen phones; this work can proceed alongside the other foundation tasks.
- [ ] Review the combined game path and its performance evidence. Record which parts work, which were measured, and which devices still need testing in the [maturity record](design/MATURITY.md).

## After the foundation

These are the next major pieces, in dependency order. Some work can overlap once the shared rules are settled.

- [ ] **Editable game and save:** Change a source file, build it into game content, run the game, save, restart, and restore. Recover cleanly if a build fails.
- [ ] **Small rendered game:** Add one camera and basic visuals, then hand a working development build to someone starting from a clean copy.
- [ ] **Real multiplayer:** Run that same game over a secure network connection. Test four players joining, reconnecting, delayed messages, and faults. This work starts alongside the rendered game, before broader integration.
- [ ] **More content and creation tools:** Add a larger authored scene, two world regions, reimporting, and spatial editing.
- [ ] **Integrated game:** Bring character movement, visuals, sound, interface, and environment together in one scene.
- [ ] **Platform choices:** Qualify desktop, browser, and mobile builds on named devices. The first platform build begins with the small rendered game.
- [ ] **Advanced capabilities:** Add distinctive improvements only with proof that they preserve the shared behavior and improve the intended result.

The world update rate is **not a frame-rate ceiling**. Rendering and input presentation run separately, so a capable device can show more frames than world updates, including high-refresh displays. The first 30/60 fps platform targets are qualification baselines, not engine limits. Higher frame rates need their own whole-game measurements for smoothness, latency, power, and thermal behavior.

Performance is part of every step. We want excellent speed and stable behavior, so each new capability needs a cost estimate and appropriate measurements while it is designed and built. The [performance plan](design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) explains how.

## Water design preparation

This D2 work can proceed alongside the foundation. It does not make water a dependency of the first small game.
The [water commission](design/modules/WATER.md) distinguishes the selected lightweight baseline from the wider
interactive-water target; all runtime and visual/device evidence remains separate.

- [x] Align the existing water ambitions and research with Thrum's owners, contracts, baseline limits and cost policy; obtain an independent coverage review. See the [alignment record](evidence/water/ALIGNMENT-2026-11-01.md).
- [ ] Complete D2-water B/C: select and derive coherent dynamics/contact/transfer/whitewater mechanisms and close their rendering, content, save and headless/replica contracts before dependent implementation.

## How a box gets checked

Passing `./dev check` shows that repository checks pass. It does not prove that a game works well, looks right, or performs well on a device. The [maturity record](design/MATURITY.md) tracks implementation, correct behavior, visual review, and device measurements separately. We check off a piece here when its stated behavior and relevant evidence are in place.

## Contributor reference

The [construction plan](design/CONSTRUCTION-PLAN.md) uses short stage names: **I1** is the headless foundation and early phone shells; **I2a** is source files, content, and saves; **I3a** is the first rendered game; **I2n** is real multiplayer; **I2b** expands content and creation tools; **I3b** integrates play and presentation; **I4** covers platform choices; and **I5** covers advanced capabilities. **D1/D2** are detailed design work needed before dependent features. Other codes point to specific technical areas or requirements: **F1** covers the runtime clock and coordination; **L4** in the [content lifecycle review](design/research/CONTENT-LIFECYCLE-REVIEW.md) says canceled or obsolete work must not reappear later. You do not need these codes to follow the work list above.

The [shared contracts](design/CONTRACTS.md), [foundation specification](design/FOUNDATION-SPEC.md), and [network specification](design/NETWORK-SPEC.md) govern the implementation. Keep one owner for each kind of state, and keep the public engine API independent of its private storage implementation. Use one small core crate until a real ownership or dependency need warrants splitting it.
