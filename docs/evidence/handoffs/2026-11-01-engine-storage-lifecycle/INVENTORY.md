# Read-only storage inventory — 1 November 2026

Scope: directory-size and Git worktree registration inventory on the current host. Commands: `df -h .`,
`du -sh` for named repository/app storage paths, `du -k -d 1` for selected roots, and `git worktree list --porcelain`.
No file contents in app sessions were inspected. No assets, builds, worktrees, conversations or Git objects were removed.
Measurements were sequential on a live system, not an atomic filesystem snapshot.

| Location | Approximate footprint |
| --- | ---: |
| Available space, Data volume | 25 GiB (98% capacity reported) |
| `tmp` | 325 GiB |
| `tmp/worktrees` (within tmp) | 315 GiB |
| `.git` | 74 GiB |
| `.git/lfs/objects` (within .git) | 71 GiB |
| `factory` | 36 GiB |
| `labs` | 12 GiB |
| `notes` | 3.2 GiB |
| `game` | 1.0 GiB |
| `dist` | 315 MiB |
| `node_modules` | 235 MiB |
| `~/.codex/sessions` | 33 GiB |
| `~/.codex/archived_sessions` | 4.2 GiB |
| `~/.codex/worktrees` | 1.3 GiB |
| `.claude/worktrees` | 4.85 GiB |

Largest sampled registered trees: hamlet-runtime ~88.1 GiB, clouds-proof ~63.4 GiB, engine-rebuild ~40.7 GiB.
Hamlet-runtime contained ~62.9 GiB nested tmp and ~20 GiB across three target directories. Clouds-proof contained
~37.5 GiB target and ~24.6 GiB nested tmp. Engine-rebuild contained ~18.6 GiB factory and ~16.8 GiB target.
Current tmp/generations was ~292 MiB; tmp/handoffs ~7.2 MiB. Thirteen worktrees were registered including main;
two Claude worktrees carried lock records. Lock age was not treated as process-liveness evidence.

No dirty/untracked/ignored source, ownership, process-liveness, remote-backup or restore audit was performed.
All named large trees may contain active work or unique inputs. Footprints are not recoverable-byte estimates;
APFS sharing, snapshots, concurrent writes and nested totals prevent adding these rows into a free-space claim.
The design is [STORAGE-LIFECYCLE](../../../design/STORAGE-LIFECYCLE.md).
