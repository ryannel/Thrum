# Thrum — contributor instructions

Thrum is a native-first, reusable Rust 3D engine and a showcase of AI-driven engineering. Start at
[README](README.md), then the purpose in [the brief](docs/plans/BRIEF.md) and your module in
[the module list](docs/plans/ARCHITECTURE.md). A provisional simulation and synthetic renderer run in the V1 test game,
but no general engine runtime exists yet; future SDK commands in design documents are proposals, not commands to run.

## How work runs

- Work follows the orchestrate skills, configured by [ORCHESTRATE.md](ORCHESTRATE.md): `plan-module` to plan,
  `build-milestone` to build one milestone, `review-milestone` to judge one, and `build` to run the build unattended.
  [thrum-engineering](.agents/skills/thrum-engineering/SKILL.md) holds the engine rules they use.
- For code changes, read [CONTRIBUTING](CONTRIBUTING.md) and use `./dev check`; documentation-only work uses
  `./dev check-docs`, and `./dev help` lists the implemented commands. Ordinary fixes need only their owning design,
  the relevant contracts and affected consumers, not the whole design library.
- Keep building. Agents review visual output themselves (images and sequences) and independently, following
  [visual review and human steering](docs/design/AI-DEVELOPMENT.md#visual-review-and-human-steering). Owner digests
  are asynchronous: never wait for a reply to a progress report. Stop to ask only for direction changes, spending,
  publishing or irreversible actions, and continue with unblocked work meanwhile.
- If work is delegated, name the model and effort (medium by default) and give the delegate the shared contracts, file
  ownership, affected consumers and completion criteria. Integrate and verify the combined result sequentially.
- Local commits are verified checkpoints. The owner authorises pushing verified checkpoints to `main`, as
  ORCHESTRATE.md's `Integrate` line says. Don't publish or deploy anything else without authorisation.

## Engine rules

- [C4 contracts](docs/design/CONTRACTS.md) own shared invariants. Resolve changes with producers and consumers before
  building dependent features. Keep one authority for each kind of state.
- [The maturity ledger](docs/design/maturity.json) owns capability status. Record implementation, correctness, visuals
  and device measurements separately; repository checks prove none of them.
- The goal is full use of each device's hardware. Follow
  [the hardware-native decision](docs/design/decisions/HARDWARE-NATIVE-ENGINE.md): registry-selected recipes and
  accelerated paths, a portable fallback, promotion only by measured win.
- For capability design or performance-sensitive code, use
  [the performance policy](docs/design/AI-PERFORMANCE.md#performance-investment-from-the-first-design): account for
  expensive and frequently repeated work before choosing representations. Specialised internals are welcome when
  verifiable and worthwhile; preserve public semantics, stability and quality. Routine edits need scoped checks.
- Register real crates and permitted workspace dependencies in `.config/architecture.toml`. Keep public boundaries
  narrow; avoid speculative packages and dependencies.

## Repository hygiene

- Work only in this repository and preserve unrelated changes.
- Keep experiments in `tmp/<task>/` and selected evidence in `docs/evidence/`. Follow
  [the storage rules](docs/ASSET-LIFECYCLE.md) for bulk outputs, profiling and temporary Python environments, and clean
  up owned recordings and environments at closeout, including tool-created scratch outside the repository.

## Broadsides

Broadsides is a resource, not a design authority or a build dependency. Its code, shaders and assets may be copied
into this repository and reused as is wherever they are useful and fit the approach Thrum's own design has chosen;
Thrum's designs and contracts decide, and Broadsides never dictates them. Its sea and clouds come across module by
module under [the port decision](docs/design/decisions/SEA-AND-CLOUD-PORT.md). Don't modify Broadsides itself or apply
its TS/foundation workflow here.
