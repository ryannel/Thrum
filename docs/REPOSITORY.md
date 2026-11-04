# Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/` | Engine libraries with explicit ownership; currently provisional simulation, render, network, source and cook modules |
| `tools/xtask/` | Host-only contributor command runner, never an engine runtime dependency |
| `scripts/` | Small repository/document validation tools |
| `.config/architecture.toml` | Registered workspace packages and allowed dependency edges |
| `.agents/skills/` | Focused engine-design and repository workflows |
| `docs/design/` | Canonical engine requirements, contracts, specifications and maturity |
| `docs/evidence/` | Compact dated research/review and implementation evidence |
| `docs/provenance/` | Import origins and historical context, not working dependencies |
| `tmp/<task>/` | Ignored, owned disposable work with a bounded lifetime |
| `target/` | Ignored reproducible Cargo output |

Add `examples/<name>/` when an actual complete public-API game exists. Add selected authored assets with
that game's own package and exact pins when needed. Do not create empty directories/crates for every
planned capability. Engine code must not depend on contributor tools, scratch work or example internals.

Plans, source and small selected evidence belong in Git. Bulk masters require a deliberately configured
LFS or durable store before they are claimed recoverable. No shared global cache or remote asset store
is configured. See [the lifecycle rules](ASSET-LIFECYCLE.md).
