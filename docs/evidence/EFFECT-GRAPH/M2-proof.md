# EFFECT-GRAPH M2 — target proof

13 November 2026. Engine source and checked commit `ec9dd79cacdf20861a1f82b28c2c454435eb4ddd`.
All six automated item commands pass on this final branch source, after the temporal-input integration and
the unchanged effect-recording method's relocation into `synthetic/contributed.rs`.
[Check report](M2/milestone-check.json) · [qualification identity](M2/qualification-identity.json).
The separately GPU-locked [repository gate](M2/dev-check.log) passes. M2 remains `in_progress` for scripted integration;
the lead starts independent review after the script checks the merged work.

| Item | Target evidence and builder inspection |
| --- | --- |
| M2.1 | Five `effect_state` tests pass, including the unchanged day/night/capture graph snapshots. `examples/v1-game/src/effect.rs` declares resources, named passes and drawing through public contribution types; its cooked contribution is retained in the ordered list. The realtime images below show both drawing together. |
| M2.2 | [Native state records](M2/state-record.txt), portable and Apple tile. The literal table in `effect/qualification/oracle.rs` is independent of `EffectState` and the GPU kernel. The field is nonempty before equal tick and pause, and again before timeline reset. Every row compares every texel, including the consumer and all auxiliary kinds. The stall counts exactly 76 dropped ticks; discarded/capture anchors do not commit. |
| M2.3 | [Native cadence records](M2/cadence-record.txt): both recipes keep pipeline count at 17 across runtime cadence changes. Each consumer output is checked across run/skip boundaries and after discard; reset/first-epoch readbacks cover every texel. Named refusal tests cover transient/imported skipping, loaded attachments, time-sliced history, duplicate producers and an early result read. |
| M2.4 | Both native state runs destroy the old device, request a new adapter from the same instance, publish generation 2 and compare the fully reset resources again. No old-generation resource is reused. |
| M2.5 | [Chrome result](M2/chrome-effect-state.json), [built-artifact hashes](M2/chrome-run.json), [hardware GPU identity](M2/chrome-browser-gpu.json). The same script passes through real destroyed-device recovery, reading every cube face, volume texel and array mip at each row. Function only; no browser timing claim. |
| M2.8 | [Verifier record](M2/baseline-record.json): all 38 native matrix comparisons, four native surface comparisons, and 481 records in each of day/night SDR/scene-linear Chrome sets pass. Covered-content key `5214d4597203d19ed8c4b3192ae4a390dbdf9ba171d1a8159923584fc79f65fe`. Original references remain unchanged. Compact source manifests and comparison records are retained alongside this record. |
| M2.6 | [Realtime movie at 1×](M2/trail-realtime-1x.mp4), [whole-sequence navigation](M2/whole-sequence.png), [exact source/image identities](M2/selected.json). T2 `realtime`, Metal on Apple M3 Max, portable recipe, 1920×1080: 964 submitted images, no ring or writer omissions. The source stops at tick 270; the field persists and fades to zero. Cadence changes from 2 to 4 at tick 300. Camera X crosses 32 m at tick 346; selected consecutive native-size crops span X=31.693…33.067 m. The trail stays attached to the checker ground and shows no reset or jump across either transition. The builder inspected whole frames and 12 consecutive crops per event at 1×: [source stop](M2/stopped-consecutive-1x.png), [cadence](M2/cadence-consecutive-1x.png), [wrap](M2/wrap-consecutive-1x.png). The field is deliberately periodic and the ground is a diagnostic opaque quad; the cooked valley draws alongside it. |
| M2.7 | [Costs and load](M2/costs.json), [diagnostic engine records](M2/runtime.json.gz), [matched no-effect records](M2/without-effect-runtime.json.gz). No-effect records contain no effect pass; the default empty declaration adds no resource or copy. The retained pre-M1 graph snapshot is also checked by `effect_state`. These are executor `PassTiming` names; the host's frame-sample fields were not changed. All eight effect compute passes have measured executed samples. Skips are absent, never fabricated zero times. The opaque diagnostic draw shares the existing forward pass. |

Median executed GPU time in microseconds: trail 6.230, derived history 3.667, consumer 4.375, volume 2.667,
array mips 2.917/2.917/2.792, cube 2.959. Descriptor memory is **23,840 bytes**: two history pairs 16,384,
consumer 4,096, volume 1,024, array 672, cube 1,536, uniforms 128. Driver pipeline/binding overhead is excluded.
Both cost arms use capture ON; the load averages and process usage are in the record. This is recorded cost,
not a speedup or display-cadence claim.

The [capture identity record](M2/manifest.json.gz) pins the binary, shaders, cooked root, route, parameters and
verified original RGBA hashes. The preview retains host submission times; its maximum PTS error is 0.5 µs,
with one labelled terminal duplicate. Full lossless `frames.zst` archives remain in this worktree at
`tmp/effect-graph-m2/trail-2e9ccbda/` and `tmp/effect-graph-m2/without-effect-2e9ccbda/` through lead review.
Those scratch folder suffixes come from the earlier handoff; both manifests pin the actual final source `ec9dd79c`.
After accepted review and verification of the promoted excerpts, owned bulk outputs may expire. Local bulk is
not a durable backup. No repository baseline was changed.
