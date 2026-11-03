# Thrum — contributor instructions

Thrum is a reusable Rust 3D engine. Start at [README](README.md) and the relevant item in
[the build plan](docs/BUILD-PLAN.md). A bounded headless host exists, but no general engine
runtime exists yet; examples of future SDK commands in design documents are proposals, not commands to execute.

- For code changes, read [CONTRIBUTING](CONTRIBUTING.md) and use `.agents/skills/engine-contributor`. Use `./dev check`; documentation-only
  work uses `./dev check-docs`. `./dev help` lists the implemented commands.
- For architecture changes, use `.agents/skills/engine-architecture`. For a new or substantially
  redesigned module, use `.agents/skills/engine-module-design`. Ordinary fixes need only their
  owning design, relevant contracts and affected consumers, not the whole design library.
- [C4 contracts](docs/design/CONTRACTS.md) own shared invariants. Resolve changes with producers
  and consumers before building dependent features. Keep one authority for each kind of state.
- [The maturity ledger](docs/design/maturity.json) owns capability status. Record implementation,
  correctness, visuals and device measurements separately. Repository checks prove none of those.
- For capability design or performance-sensitive code, use [the performance policy](docs/design/AI-PERFORMANCE.md#performance-investment-from-the-first-design):
  account for expensive and frequently repeated work before choosing representations. Specialized internals are welcome
  when verifiable and worthwhile; preserve public semantics, stability and quality. Routine edits need scoped checks.
- Register real crates and permitted workspace dependencies in `.config/architecture.toml`.
  Keep public boundaries narrow; avoid speculative packages and dependencies.
- Preserve unrelated changes. Keep experiments in `tmp/<task>/`, selected evidence in
  `docs/evidence/`, and follow [the storage rules](docs/ASSET-LIFECYCLE.md) for bulk outputs.
- If work is delegated, use bounded Sol High commissions with shared contracts, file ownership,
  affected consumers and completion criteria. Integrate and verify the combined result sequentially.
- Work only in this repository. Broadsides is historical context and is not a build dependency.
  Do not modify it, import its assets/code wholesale, or apply its TS/foundation workflow here.

Local commits are useful verified checkpoints. Do not push, publish or deploy without authorization.
