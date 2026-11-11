# M2 MF1 requalification

11 November 2026, mac-metal on this Apple M3 Max. This record closes the exactness failure in
[review MF1](../../APPLE-TILE/M2-review.md#mf1-d23-exactness-broken-at-claimed-commit-d23-verified).
The fresh release build contains APPLE-PRESENTATION's jitter gate `ae0cc8fa` and ATMOSPHERE M5 `7f020ca1`.
Both matrices ran from clean
revision `9af8664f63b95c18e7d3b8e5432288c066204aac`, with binary SHA-256
`8b4178416ea8039ef5672ba6651ad902f79c23de2aed3edad03aac537a2a8468`.
The clean-diff hash, compiler and build command are retained in [provenance](mf1-recheck/provenance.json).

| Proof | Result |
| --- | --- |
| `scripts/apple_tile_m1.py` | 38/38 exact M0 comparisons, zero changed pixels. Both recipes, SDR and scene-linear route checkpoints, portable repeats, Sun and night. Sample/exposure identities match. [Manifest](mf1-recheck/m1-manifest.json.gz). |
| `scripts/apple_tile_m2.py` | 4/4 exact M0 letterbox comparisons, zero changed pixels, both recipes and Bgra8Unorm/sRGB. Both realtime route/cut and render-stall captures complete. [Manifest](mf1-recheck/m2-manifest.json.gz). |
| Direct paired recipes | 10/10 exact SDR/scene-linear route comparisons with matching sample/exposure identity. [Results](mf1-recheck/paired.json). |
| Realtime source accounting | Portable/Apple route images 960/712, history epochs 1→2. Stall images 446/368, with 32/30 skipped opportunities including the deliberate 30-opportunity interruption, and recovery. Largest injected submission gaps 263.31/262.97 ms. Every source verified, zero ring/writer omissions. [Records](mf1-recheck/motion-summary.json). |

Both reproduction commands used `--store tmp/g3t-m0/valley` and fresh output directories
`tmp/g3t-m2/mf1-rebased/m1` and `tmp/g3t-m2/mf1-rebased/m2`, under `agent_loop.py lock gpu`.
Their default binary was the freshly built `target/release/v1-game`. The M0 baselines retain their original hashes.

The [inspection list](mf1-recheck/inspection.txt) covers full Sun/night frames, native Sun/star details,
terrain/shadows, mirror/local lights, route cut and skipped-view recovery. The builder inspected fresh full frames,
native crops, both recipes' motion contact sheets and cut-adjacent full-resolution sources. The Sun disk/glow,
stars, terrain, shadows, mirror and light/cube witness are present. Cut samples switch views with history epoch 2
and intact scene content. Stall sequences retain the scene and resume camera movement. Selected sources and
contact sheets accompany the manifests. Original-time 1x movies and lossless spools remain in owned scratch
through lead review.

D2.1, D2.2, D2.4 and D2.5 retain their passed [earlier evidence](README.md) and the independent review's
observations. The attended 1x playback record is retained as requested. Owner setup and direction remain verbatim
in [ACCEPTED.json](../../../plans/APPLE-TILE/ACCEPTED.json), `milestones.M2.owner_answers`, with the latest answer
for each item authoritative.

The required `agent_loop.py check --module APPLE-TILE --milestone M2 --worktree .` passes, including
`./dev check`, under the GPU lock. [Check output](mf1-recheck/milestone-check.txt),
[documentation check](mf1-recheck/check-docs.log), [selected-file hashes](mf1-recheck/retained-files.json).
The rebased source passed the profile check during [integration](mf1-recheck/source-integration.json).
The release binary was then rebuilt and both matrices rerun on that integrated source. The final evidence commit changes documentation
only. Captured source-file hashes permit comparison with the claimed tree. M2 is reclaimed for lead review.
