# GAME-SPLIT log

- 2026-11-14 Owner: "it sounds like we're blurring the lines between the engine and the game a ton. Please let's
  separate out the game as much as we can, it shouldn't even sit in the same code base ideally." The lead's task
  set the repository decision: a local repository at `/Users/ryannel/Workspace/thrum-games`, no remote, path
  dependencies on a sibling Thrum checkout, the swarm game there from its start.
- 2026-11-14 Probed while planning: a game whose manifest names `../../thrum/crates/…` builds against a second
  engine tree through a folder of links, with no change to the game. The linked build wrote a lockfile into the
  game's folder, so the gate works from a detached copy.
- 2026-11-14 Decided: the move waits for `SHOWCASE` M8; the proving ground stays at `examples/v1-game` under its
  old name until no open check names it, because 54 approved checks in other modules' open milestones do.
- 2026-11-15 Revised after [the first design review](../../evidence/GAME-SPLIT/design-review.md) (three Must
  Fix). The mixed-mesh field is one fixture in `crates/thrum-fixtures`, `TEST-SUPPORT`'s package, read by the
  visibility tests as a dev-dependency and by the proving ground as a normal one; M2 proves the proving ground's
  Mac and wasm32 builds. The presets, routes, valley sources and pages the exact baselines are drawn from stay in
  Thrum as frozen qualification content, and M4 runs the verifier with the games repository unavailable. M4's
  exit check runs every command `SHOWCASE` M5.5 and M7.2 to M7.8 pinned, the live clauses included; frame and
  visual evidence stays `SHOWCASE`'s accepted history. The owner's decisions are unchanged.
