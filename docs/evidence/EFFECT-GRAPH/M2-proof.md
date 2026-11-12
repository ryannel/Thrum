# EFFECT-GRAPH M2 — target proof

12 November 2026. Engine source `41357d2f9831227252e98190907db2b395ae0293`.
M2 is in progress: M2.6 and M2.7 have target evidence; automated checks and integration are still pending.

| Item | Target evidence and builder inspection |
| --- | --- |
| M2.6 | [Realtime movie at 1×](M2/trail-realtime-1x.mp4), [whole-sequence navigation](M2/whole-sequence.png), [exact source/image identities](M2/selected.json). T2 `realtime`, Metal on Apple M3 Max, portable recipe, 1920×1080: 963 submitted images, no ring or writer omissions. The source stops at tick 270; the field persists and fades to zero. Cadence changes from 2 to 4 at tick 300. Camera X crosses 32 m at tick 346; selected consecutive native-size crops span X=31.696…33.067 m. The trail stays attached to the checker ground and shows no reset or jump across either transition. The builder inspected whole frames and 12 consecutive crops per event at 1×: [source stop](M2/stopped-consecutive-1x.png), [cadence](M2/cadence-consecutive-1x.png), [wrap](M2/wrap-consecutive-1x.png). The field is deliberately periodic and the ground is a diagnostic opaque quad; the cooked valley draws alongside it. |
| M2.7 | [Costs and load](M2/costs.json), [diagnostic engine records](M2/runtime.json.gz), [matched no-effect records](M2/without-effect-runtime.json.gz). No-effect records contain no effect pass; the default empty declaration adds no resource or copy. The retained pre-M1 graph snapshot is also checked by `effect_state`. These are executor `PassTiming` names; the host's frame-sample fields were not changed. All eight effect compute passes have measured executed samples. Skips are absent, never fabricated zero times. The opaque diagnostic draw shares the existing forward pass. |

Median executed GPU time in microseconds: trail 6.291, derived history 3.666, consumer 4.375, volume 2.667,
array mips 2.958/3.000/2.792, cube 2.958. Descriptor memory is **23,840 bytes**: two history pairs 16,384,
consumer 4,096, volume 1,024, array 672, cube 1,536, uniforms 128. Driver pipeline/binding overhead is excluded.
Both cost arms use capture ON; the load averages and process usage are in the record. This is recorded cost,
not a speedup or display-cadence claim.

The [capture identity record](M2/manifest.json.gz) pins the binary, shaders, cooked root, route, parameters and
verified original RGBA hashes. The preview retains host submission times; its maximum PTS error is 0.5 µs,
with one labelled terminal duplicate. Full lossless `frames.zst` archives remain in this worktree at
`tmp/effect-graph-m2/trail-proof/` and `tmp/effect-graph-m2/without-effect-proof/` through lead review.
After accepted review and verification of the promoted excerpts, owned bulk outputs may expire. Local bulk is
not a durable backup. No repository baseline was changed.
