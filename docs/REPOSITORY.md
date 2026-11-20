# Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/` | Engine libraries with explicit ownership; currently provisional simulation, content runtime, render and network modules |
| `examples/` | `v1-game`, the V1 test game, and its iOS shell `v1-game-ios` |
| `examples/v1-game/tests/fixtures/` | Retained inputs to the V1 game's automated checks |
| `tests/fixtures/<module>/` | Test data a repository-level check or script reads, such as the showcase motion inputs and the `RENDER-2D` cost receipts, outside the recorded build inputs; data a Rust test reads sits beside that crate's tests |
| `tools/xtask/` | Host-only contributor command runner, never an engine runtime dependency |
| `scripts/` | Small repository/document validation tools |
| `.config/architecture.toml` | Registered workspace packages and allowed dependency edges |
| `.agents/skills/` | Focused engine-design and repository workflows |
| `docs/plans/` | The orchestrate plans: the status page, the brief, the module list, and a folder per module with its design, plan, log and done-when file |
| `docs/design/` | Canonical engine requirements, contracts, specifications and maturity |
| `docs/evidence/` | Frozen: the merge refuses a new or changed file here. It holds the last records a script still reads until they move to fixtures, and copies `evidence_store.py restore` writes, which Git ignores. Run records and reviews go in the working folder, `.orchestrate/evidence/<MODULE>/` |
| `docs/provenance/` | Import origins and historical context, not working dependencies, and each module's record of what it copied or translated, from where and under what licence, as `<MODULE>.md` |
| `tmp/<task>/` | Ignored, owned disposable work with a bounded lifetime |
| `target/` | Ignored reproducible Cargo output |

Add another `examples/<name>/` only when an actual complete public-API game or demo exists. Add selected authored assets with
that game's own package and exact pins when needed. Do not create empty directories/crates for every
planned capability. Engine code must not depend on contributor tools, scratch work or example internals.

Plans, source and test data belong in Git; evidence does not. Bulk masters require a deliberately configured
LFS or durable store before they are claimed recoverable. No shared global cache or remote asset store
is configured. See [the lifecycle rules](ASSET-LIFECYCLE.md).
