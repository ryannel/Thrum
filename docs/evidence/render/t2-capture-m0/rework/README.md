# T2-CAPTURE M0 rework — 9 November 2026

Builder reclaim after [review F1–F4](../../../T2-CAPTURE/M0-review.md), Apple M3 Max/Metal.
[Verification](verification.json), [all capture commands/exits](integrated/attempts.json),
[measurements](summary.json), [runbook](../../../../CAPTURE.md). Lead re-review is owed; no reviewer launched.
All device runs held `gpu`, serially, without concurrent builds/cooks. Every attempt is retained: preliminary
probes/pan are at this directory's root; `final/` retains the pre-accounting-correction series and five unchanged
writer probes; `integrated/` qualifies the corrected binary `0e77c57d…dccbe97`, with one source/recipe digest.

| Proof | Complete repeated workload | Measured headroom/storage |
| --- | --- | --- |
| Writer | 5/5 consecutive probes before scenario integration, each 480 × 1920×1080 RGBA at 120 records/s; every decoded PNG hash verified | 146.4–297.3 records/s against 120; CPU peak 4–31/96; uncached writes, full flush, drain and deferred packaging per attempt |
| Cooked paused pan | 5/5 consecutive corrected-build runs; 480, 480, 481, 481, 481 opportunities/submissions/images; tick 0; no omissions, skipped indices or view refusals | 1.64–2.98 GB/s writer service; observed staging/CPU peaks at most 3/20 against 6/96 caps; every archive decoded/hash-checked and preview PTS validated |

Six staging slots and 96 CPU leases charge **846,028,800 pixel bytes** below the 1 GiB policy; CPU reserve is
0.8 s, not a four-second session. Small query/marker/pipeline metadata is separate. Per-session output/free-space
allowance stays 16 GiB. Retention files verify selected roots and every decoded archival RGBA hash before owned
raw/PNG duplicates are deleted. Local archives under `tmp/t2-capture-m0-rework/integrated/` remain through lead
review and are not durable backups. PNGs, hashes, times, profiles and selected 1× previews are committed.

Generation and lead originate in the engine: the host stamps device generation, presenter incarnation and
consumed lead at submission; the adapter reads its submission ID. Presenter incarnation means engine
prepare/resize/device publish, not an OS lease. Metal tests exercise device 7→8 and resize 1→3 with old captures
retaining their old identities. The engine clock targets the next opportunity and consumes it using the native
actor's shared `display_target_lead`; untargeted prime/warmup lead is explicitly zero. Setup marker pipelines
use the generation's creation authority and transition scope, checked with `creation-counters` enabled.

GPU copy cost uses **real dependency-ordered compute markers**. Encoder-level timestamps are unsupported here;
empty marker passes returned zero counters. Final markers read staging before/after its blit write without
altering pixels, forcing WAR/RAW hazards. Before-marker end→after-marker end is a measured upper bracket,
including the after-marker and encoder transitions. Every corrected-build image has a positive bracket. Resolution
is later coordinator work after completion. Unsupported devices report null. This excludes map/unpack and
query-resolution costs and is not a whole-GPU span. Disabled capture has no copy/marker resources. Failed maps
or timing collection retire the GPU slot and return the CPU lease, leaving the interval explicitly incomplete.

Matched unprofiled order was **OFF/ON/ON/OFF**, with per-second host CPU/disk samples, load averages and background
process snapshots. Snapshot CPU is a lifetime average; iostat's first row averages since boot.

| Arm | Step CPU median ms | Scene GPU median ms | Copy bracket median ms | Whole-child CPU s | Peak RSS MB |
| --- | ---: | ---: | ---: | ---: | ---: |
| OFF 1 | 1.008 | 2.672 | absent | 1.643 | 82.31 |
| ON 1 | 0.834 | 2.681 | 1.044 | 2.515 | 153.62 |
| ON 2 | 0.761 | 2.685 | 1.154 | 3.043 | 203.95 |
| OFF 2 | 1.364 | 2.674 | absent | 1.731 | 83.67 |

ON arms retained 481/481 and 480/480 images, no omissions: 3,989,606,400 / 3,981,312,000 raw bytes. Full flush
was 15.717/10.324 ms; deferred PNG/archive/preview validation 46.899/45.924 s. Load-1 ranged 2.40–3.11;
raw `system-iostat.txt` gives CPU idle and disk0 MB/s. Step CPU excludes image draining/writing, so its lower ON
value is not a speedup. Whole-child CPU includes preparation and three-second warmup; ON rises 53%/76% in these
pairs. This is material opt-in observer cost, not an uncaptured baseline or a performance promotion.

Separate [OFF](integrated/profile-off/cpu-sample.txt)/[ON](integrated/profile-on/cpu-sample.txt) CPU samples
(requested 1 ms) start at the engine's pre-prime ready marker and end at child exit. Added paths are observed,
not just inferred: `try_collect_into → _platform_memmove`, stdout newline scanning (201 stack observations),
`write` (516) and writer channel waiting (1,922). Counts include blocked stacks and are **not CPU percentages**.
The profile locates bulk row copies, unnecessary line scanning and storage work; it does not causally apportion
every millisecond. Driver work appears in both arms. Profiled arms are separate from the unprofiled baseline.

Builder inspection used native full frames, adjacent native crops and host-time sequence views, with PTS-checked
1× previews: [pan](integrated/pan-1/preview-1x.mp4) has coherent rock/ridges and sphere exit, no cut/reversal,
initial/final pixel holds at least 1.0027/1.0006 s. Still/fixed/pan endpoints match. [Running](integrated/running/preview-1x.mp4)
retains 481/481, ticks 0–119, 155 strict intermediate alphas and history epoch 0, with consumed lead median 5.895 ms.
Its 82 alpha-one samples among 235 interior-tick records remain visible; interpolation clamping is the existing
engine limit identified in review, not corrected by capture. [Render stall](integrated/render-stall/preview-1x.mp4)
logs 255.275 ms and a 261.871 ms image gap; [input stall](integrated/input-stall/preview-1x.mp4) logs 250.036 ms
with 30 identical fresh images, then recovery. Input edits coalesce to the newest value. Slow writer correctly fails:
480 continuing submissions, ticks 0–119, 167 images, 313 omissions, finite 6/96 peaks and 14.013 s engine drain.
Native-sequence PNGs have adjacent source IDs/times in sibling JSON files. No history-reset trail or copy corruption
was seen. No window/compositor/panel/EDR qualification follows; Mac M2 owners remain separate.
