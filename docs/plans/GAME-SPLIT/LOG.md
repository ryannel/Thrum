# GAME-SPLIT log

- 2026-11-17 M1 builder: added the detached game gate and a shared external-consumer boundary adapter.
  Confirmed four scratch-repository integration tests on this Mac, including builds against two engine
  revisions, unchanged source/index/lockfile, dirty and missing repositories, and rejected boundary edges.
  Existing shape-rule tests passed. Evidence and the command convention are in
  [M1 evidence](../../../.orchestrate/evidence/GAME-SPLIT/M1.md). Exact-image proof remains the shared main-branch check;
  no runtime source or retained image changed.

- 2026-11-14 Owner: "it sounds like we're blurring the lines between the engine and the game a ton. Please let's
  separate out the game as much as we can, it shouldn't even sit in the same code base ideally." The lead's task
  set the repository decision: a local repository at `/Users/ryannel/Workspace/thrum-games`, no remote, path
  dependencies on a sibling Thrum checkout, the swarm game there from its start.
- 2026-11-14 Probed while planning: a game whose manifest names `../../thrum/crates/…` builds against a second
  engine tree through a folder of links, with no change to the game. The linked build wrote a lockfile into the
  game's folder, so the gate works from a detached copy.
- 2026-11-14 Decided: the move waits for `SHOWCASE` M8; the proving ground stays at `examples/v1-game` under its
  old name until no open check names it, because 54 approved checks in other modules' open milestones do.
- 2026-11-15 Revised after [the first design review](../../../.orchestrate/evidence/GAME-SPLIT/design-review.md) (three Must
  Fix). The mixed-mesh field is one fixture in `crates/thrum-fixtures`, `TEST-SUPPORT`'s package, read by the
  visibility tests as a dev-dependency and by the proving ground as a normal one; M2 proves the proving ground's
  Mac and wasm32 builds. The presets, routes, valley sources and pages the exact baselines are drawn from stay in
  Thrum as frozen qualification content, and M4 runs the verifier with the games repository unavailable. M4's
  exit check runs every command `SHOWCASE` M5.5 and M7.2 to M7.8 pinned, the live clauses included; frame and
  visual evidence stays `SHOWCASE`'s accepted history. The owner's decisions are unchanged.
- 2026-11-15 Revised after [the second design review](../../../.orchestrate/evidence/GAME-SPLIT/design-review.md) (one Must
  Fix). M1 now waits for `TEST-SUPPORT:M1` as well as `REPO-RULES:M1`, because M1.4's `boundary` runs that
  milestone's `test-crate-edges` rule; the rule keeps one owner and no M1 item changed. Chosen over moving the
  check to M2 because nothing uses the gate before M4, so the wait costs nothing, and the gate's whole boundary
  is judged in one milestone. Suggestion taken: M4.7 has the verifier hand `THRUM_GAMES` to the commands it
  starts, because it removes every `THRUM_` variable today and the item's check would otherwise pass whether or
  not a child could reach the sibling games repository.
- 2026-11-15 Planner, with the ruling [proofs run once, on main](../ARCHITECTURE.md#proofs-run-once-on-main):
  M4.7's check gained `--full`. Plain `verify` now asks the run on main and draws nothing, and M4.7 needs a real
  run with `THRUM_GAMES` pointing at nothing. Its statement did not change. The other verifier items lost their
  `lock` and one simulator item moved to the `simulator` lock.
- 2026-11-18 M2 moved the mixed-mesh field to `thrum-fixtures` and routed the temporal witness, cloud shader and
  crowd trace through their owning packages. Both boundary rules are at zero, nine Metal visibility tests passed,
  and the engine built 214 test and bench executables with both game directories absent. The release Wasm bundle
  emitted the original witness bytes. The shared main-branch check supplies exact images. [Evidence](../../../.orchestrate/evidence/GAME-SPLIT/M2.md)

- 2026-11-24 WP3 put the engine rigs behind the default Cargo `proving` feature. The normal
  web and iPhone packaging commands build without it. The cooked texture and staging-fault
  tests now live in `thrum-asset-gpu`. Verified the no-default-features Mac tests and Wasm
  build, both cooked tests on Metal, the Chrome hooks, the iPhone build variants and the
  `game-names` rule at zero. The rebase kept the new Sun shadow, cloud capture and
  atmosphere test bank rigs behind `proving`. The scheduled `exact-images` check
  continues to verify the Mac and Chrome baselines on main.
  [WP3 evidence](../../../.orchestrate/evidence/GAME-SPLIT/WP3/README.md)
- 2026-11-25 Reduced the cooked-row visual rebuild fixture to 160×90 after a Metal test exceeded
  the merge timeout. All 12 rebuild cases passed on Metal with the same row-write stages and
  exact frame comparisons. [Metal run](../../../.orchestrate/evidence/GAME-SPLIT/WP3/rebuild-fixture-metal.log)
- 2026-11-25 WP3 review fixes compiled the diagnostic clouds, route overlays and browser
  qualification pages only with `proving`. The ordinary camera route still runs without an
  overlay. The full game-only package passed on Mac and built for Wasm, and the five pinned
  default-feature targets passed on Metal. Chrome exercised both bundles, and both iPhone
  simulator binaries linked with the expected command ABI.
  [Review-fix evidence](../../../.orchestrate/evidence/GAME-SPLIT/WP3-review-fixes/README.md)

- 2026-11-25 WP3 third builder round moved endpoint readback, native qualification
  selections and the sizing rigs behind `proving`. HOST's ten Chrome lifecycle
  cases loaded proving and material reload loaded the development bundle. The
  Mac game-only tests, Wasm build, shipping Chrome boundary, field oracle and
  both iPhone simulator links passed.
  [Review-fix evidence](../../../.orchestrate/evidence/GAME-SPLIT/WP3/round-3/README.md)
- WP3 proposes a BASELINE-GUARD verifier refresh for artifact identity. Build and
  hash one proving bundle, give it to each registered Chrome capture, and compare
  its hash with the Wasm instantiation receipt. Keep the references, comparison
  and sampling. The exact-baseline rule requires a planner ruling with the owner
  before the verifier's protected pins can change.
  [Proposed patch](../../../.orchestrate/evidence/GAME-SPLIT/WP3/round-3/exact-receipt-proposal.patch)
