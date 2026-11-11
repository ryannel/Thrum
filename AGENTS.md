# Thrum — contributor instructions

Thrum is a native-first, reusable Rust 3D engine and a showcase of AI-driven engineering. Start at
[README](README.md), then the purpose in [the brief](docs/plans/BRIEF.md) and your module in [the module list](docs/plans/ARCHITECTURE.md). A provisional simulation and synthetic renderer run in the
V1 test game, but no general engine runtime exists yet; examples of future SDK commands in design documents are proposals, not commands to execute.

- Work follows the orchestrate skills, configured by [ORCHESTRATE.md](ORCHESTRATE.md): `plan-module` to plan,
  `build-milestone` to build one milestone, `review-milestone` to judge one, and `build` to run the build unattended.
  [thrum-engineering](.agents/skills/thrum-engineering/SKILL.md) holds the engine rules they use. For code changes,
  read [CONTRIBUTING](CONTRIBUTING.md). Use `./dev check`; documentation-only work uses `./dev check-docs`. `./dev help`
  lists the implemented commands. Ordinary fixes need only their owning design, relevant contracts and affected
  consumers, not the whole design library.
- [C4 contracts](docs/design/CONTRACTS.md) own shared invariants. Resolve changes with producers
  and consumers before building dependent features. Keep one authority for each kind of state.
- [The maturity ledger](docs/design/maturity.json) owns capability status. Record implementation,
  correctness, visuals and device measurements separately. Repository checks prove none of those.
- For capability design or performance-sensitive code, use [the performance policy](docs/design/AI-PERFORMANCE.md#performance-investment-from-the-first-design):
  account for expensive and frequently repeated work before choosing representations. Specialized internals are welcome
  when verifiable and worthwhile; preserve public semantics, stability and quality. Routine edits need scoped checks.
- The goal is full use of each device's hardware. Follow [the hardware-native decision](docs/design/decisions/HARDWARE-NATIVE-ENGINE.md):
  registry-selected recipes and accelerated paths, a portable fallback, promotion only by measured win.
- Register real crates and permitted workspace dependencies in `.config/architecture.toml`.
  Keep public boundaries narrow; avoid speculative packages and dependencies.
- Preserve unrelated changes. Keep experiments in `tmp/<task>/`, selected evidence in
  `docs/evidence/`, and follow [the storage rules](docs/ASSET-LIFECYCLE.md) for bulk outputs.
- If work is delegated, name the model and effort (medium by default) and give the delegate shared contracts, file
  ownership, affected consumers and completion criteria. Integrate and verify the combined result sequentially.
- Work only in this repository. Broadsides is historical context and is not a build dependency. Its Rust sea and clouds
  port after G7 module by module under [the port decision](docs/design/decisions/SEA-AND-CLOUD-PORT.md).
  Do not modify it, import its assets/code wholesale, or apply its TS/foundation workflow here.

- Keep building. Agents review visual output themselves (images and sequences) and independently, following
  [visual review and human steering](docs/design/AI-DEVELOPMENT.md#visual-review-and-human-steering). Owner digests
  are asynchronous: never wait for a reply to a progress report. Stop to ask only for direction changes,
  spending, publishing or irreversible actions, and continue with unblocked work meanwhile.

Local commits are useful verified checkpoints. The owner authorizes pushing verified checkpoints to `main`, as
ORCHESTRATE.md's `Integrate` line says. Don't publish or deploy without authorization.
