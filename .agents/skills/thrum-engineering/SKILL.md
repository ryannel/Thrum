---
name: thrum-engineering
description: Thrum's engine rules for planning, building and reviewing, read alongside the generic orchestrate skills (plan-module, build-milestone, review-milestone). Use whenever working on Thrum engine code, module designs, architecture or milestone reviews, for the engine-specific standards those skills defer to the project for. Not a process skill on its own.
---

# Thrum engineering

Thrum is a native-first Rust 3D engine and a showcase of AI-driven engineering: butter-smooth, high-end visuals and
performance beyond what a human team would build and maintain. The process and its settings are in
[ORCHESTRATE.md](../../../ORCHESTRATE.md); this skill holds the engine rules each level reads.

**Planning** (`/plan-module`):
- [Engine architecture](references/architecture.md): users and usage profiles, responsibilities and interactions,
  module briefs, proof order.
- [Research and design](references/blue-sky.md) and [what a design covers](references/design.md): from what the player
  must see or feel, through shipped engines and research, to a mechanism shown to make each cue.
- [Performance](references/performance.md): cost account, tripwires, honest measurement on a shared GPU, and the
  [hardware-native decision](../../../docs/design/decisions/HARDWARE-NATIVE-ENGINE.md).
- [Reconciliation](references/reconciliation.md) of existing or ported code against a chosen target.

**Building** (`/build-milestone`):
- [Building in Thrum](references/building.md): picking the work, rigour by state, code shape, hot paths, verification
  on named targets, build outputs.
- [Rust code](references/rust-code.md) for types, APIs, errors, decoders and test support.
- [Implementation in the game](references/implementation.md) and [system diagnosis](references/system-diagnosis.md)
  for making the model produce a cue.

**Reviewing** (`/review-milestone`):
- [Review](references/review.md), including the calibration failures this engine has already paid for, and the
  [graded examples](../../../docs/design/GRADED-EXAMPLES.md).
