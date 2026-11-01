# Coupled cloud performance disposition

1 November2026. **Not admitted:** this fresh standalone graph misses the proposed cloud GPU allocation.
The [600-sample receipt](performance/graph-1793565955083.json), [loaded source snapshot](loaded-code/)
and [governing budget](../../design/modules/CLOUDS.md#cost-and-profile-admission) are the evidence.

## Boundary and observations

Local Apple M3 Max/36GB host, adapter `apple/metal-3`; device name is absent from WebGPU adapter info.
This is browser WebGPU, not native Thrum or M2/mobile qualification. No capture, oracle or repository build
ran during the selected sweep; another development browser was open, so this is not an isolated-device
certificate.600 measured fresh graphs after50 warmup cover source ages40–220 repeatedly. Output1280x720,
primary media640x360, independent reflected view320x180; source, all caches, both views, resolve, separate
full-resolution fallback and display execute every graph. No temporal reuse or hidden quality reduction.

| Observed boundary | median ms | p95 ms | p99 ms | max ms |
| --- | ---: | ---: | ---: | ---: |
| Compute-to-compute GPU elapsed bracket around graph | 12.845 | 18.809 | 21.758 | 23.855 |
| Submission-to-queue-completion observer wait | 16.2 | 23.9 | 29.2 | 31.1 |
| Host graph encoding only | 0.10 | 0.20 | 0.40 | 0.40 |

All600 bracketing compute timestamps are nonzero and ordered around the fused compute stage.
Render **begin** timestamps precede their producer passes on this Metal path; per-render durations are
explicitly invalid. Do not sum them. Two compute sentinels bound elapsed GPU clock time, including graph
transitions/backend stalls and any local scheduling interference. Their instrumentation is not free.
Pre-bracket tiny buffer clears and readback after the bracket are outside it. This is stronger than a
source-kernel timing, but not a claim of exact shader execution cycles or a native renderer frame.
Queue waits include host/driver/observer overhead and cannot be relabelled GPU execution.
The encoding statistic does not establish whole-process CPU cost or the complete8ms browser CPU budget.

Every measured frame retains counters:0 iteration-cap failures; fallback peak142675/921600 output pixels.
Normal and accelerated capture runs retain their own source clock/callback/queue receipts; recording adds
load and24fps video encoding is distinct from callback/presentation cadence. Their p95 callbacks are about
33.3–33.4ms, so a steady60Hz experience is not qualified. The source clock follows1x/16x/64x to floating
precision without holding or stepping a separate view clock. No weather simulation ticks are claimed.

## Budgets and memory

The5ms browser cloud GPU investment target is missed. The16ms complete browser frame envelope also has
failed tails before other game work is present. The3ms native cloud allocation and10ms desktop complete
frame have no native run; M3 browser timings cannot certify them. Q7 remains open.

The explicit GPU ledger is57.512MiB total:41.581MiB source/current/cache persistent resources and15.930MiB
transient raw/media/guide/HDR/fallback targets. Those logical counts fit the proposed50+16MiB sub-reservations.
The encoded recovery source plus descriptor is6.260MiB CPU; with GPU allocations the declared subtotal is
63.772MiB, below the88MiB service gross reservation before implementation overhead. Initial upload chunks
are2MiB; browser/driver copies, actual residency/peak, canvas swapchain, JavaScript/process memory and the
complete512MiB scene cap are **unmeasured**. Probe exports deliberately allocate readback arrays outside the
steady runtime recipe and are not a memory admission run. Future histories, extra lights/phases/views and
clouded probes need a new combined reservation; unused budget is not pre-admission for them.

## Changes retained and next risk

Direct RGBA16F storage writes preserve sampled Q1 RN-half density/mips; the same decoder mathematics are
retained. Compute construction is fused, proven source support bounds remove zero work, and canonical
missed/opaque-clipped cloud intervals avoid unnecessary fallback. Strict full-footprint compatibility
removes reflected edge streaks. Matched source/query and native image evidence accompany these choices.
Historical sweeps in this directory include changing variants and invalid render timing origins; they are
diagnostic records, not an isolated causal microbenchmark or reproducible old-release qualification.

The next bounded task is **cost qualification before expansion**: profile source/cache/transition and
fallback costs on an isolated target with trustworthy backend timings, select a complete graph cost
reduction, then repeat these exact optical/image/motion witnesses. Evaluate qualified storage, resource
transitions, consumer support/caching and actual renderer composition before adding history complexity.
Do not assume Q4 history, a weaker source or omitted reflected fog will make the budget close. Existing
Thrum I1/I2/I3a dependencies remain authoritative; no engine merge or production promotion is warranted.
