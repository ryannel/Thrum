# G3t traffic probe (D0.4), 8 Nov 2026

This was a single Xcode replay-and-profile pass over two saved one-frame Metal captures. It answers D0.4 in the
[Apple tile plan](../../plans/APPLE-TILE/PLAN.md): a per-encoder byte counter **is available** on this device.
That counter is Xcode's "Bytes Read From / Written To Device Memory", read per encoder from Profile › Counters › Memory.
The traffic rows in the G3t plans now use this method, and any row that wasn't captured keeps the *unmeasured* label.

One probe serves two consumers:
- **G3t (Apple tile), D0.4.** This is the measurement method for the plan's traffic rows.
- **G3s (star catalogue), M3's "physical bandwidth replay".** This gives the measured bytes of the `atmosphere stars`
  encoder and the effect of the star variant on the other encoders, on the same frame.

## Setup

- **Traces:** `tmp/g3s-star-catalog/current-main/m2-closeout-59e76f3/trace-midnight-20261108T020215Z/`, taken from
  `stars-off.gputrace` and `stars-on.gputrace`.
  - One frame each, 1920×1080 BGRA8, 60 Hz.
  - `sky.solar_time 0`, `camera.pitch 7`.
  - Input hashes are in `inputs.sha256`.
- **Device and tools:** Apple M3 Max, macOS 26.6.2, Xcode 27.2 (27B52). The Metal Toolchain 27.2 (27B47) download
  appeared mid-slot and was accepted by the owner. Profiler Mode was Full, Execution Mode Overlapping, and the
  Performance State reported Medium.
- **Slot:** 15:23:41–15:28:45 CET, run as an owner-present screen slot through background app control.
  - Load at the start was 29.5 / 26.5 / 21.0. By the end it was 64.2 / 49.1 / 32.6, because other sessions' builds
    were running.
  - The timeline shows *External Process* GPU work interleaved with both frames.
- **Screenshots:** 16 JPEGs in `tmp/g3t-traffic/` in the main checkout. They are not committed, under the storage
  rules.
  - Per-encoder memory tables: `11-off-encoders-memory-{a,b}.jpg` and `21-on-encoders-memory-{a,b}.jpg`.
  - Memory timelines: `10-off-timeline-memory.jpg` and `20-on-timeline-memory.jpg`.
- **Transcriptions:** `tmp/g3t-traffic/{off,on}-encoders.tsv`. CSV export wasn't reachable, because its share menu
  can't be opened under background control, so these were transcribed from the screenshots.

## Result

| Scope | stars-off | stars-on | Δ (stars) | Source |
| --- | --- | --- | --- | --- |
| Logical stored attachment bytes of `synthetic forward opaque` (derived from formats and extent, not measured) | 23.73 MiB (HDR RGBA16F 15.82 + motion RG16F 7.91; depth discarded) | 31.64 MiB (+ depth D32 7.91, stored for the star pass) | +7.91 MiB | D1.2 per-variant rules in `tmp/g3t-metal-boundary/NOTES.md` |
| Whole-frame device-memory bytes (sum of all encoders) | 28.49 MiB read, 17.71 MiB written, 46.20 MiB total | 27.78 MiB read, 20.85 MiB written, 48.63 MiB total | +2.43 MiB | Profile › Counters › Memory, summed |
| Peak whole-device bandwidth on the timeline | 204.9 GiB/s | 316.1 GiB/s | — | Timeline › Memory › Bandwidth (GiB/s, track maximum) |
| Per-encoder bytes: `atmosphere stars` (3 commands) | — | 601.88 KiB read, 1.22 MiB written | +1.81 MiB | Profile › Counters › Memory |
| Per-encoder bytes: `synthetic forward opaque` (3 commands) | 4.92 MiB read, 6.82 MiB written | 4.94 MiB read, 9.00 MiB written | +0.02 R, +2.18 W MiB | Profile › Counters › Memory |

**How the units read.**
- The byte columns are absolute bytes per encoder for the one replayed frame, in Xcode's binary units: KiB and MiB,
  shown as "bytes" below 1 KiB.
- The bandwidth columns are each encoder's average GiB/s over its own execution.
- The timeline Bandwidth track is a sampled whole-device rate, and the table reports its maximum. The sampling
  interval isn't shown in the UI, so the peaks are indicative only.

**Labels matched.** All 25 stars-off encoders and all 26 stars-on encoders carry the expected labels from the runbook:
- wgpu internal blits and `(wgpu internal) Indirect draw validation`;
- the synthetic passes, from `transform history` to `tone encode`;
- the atmosphere passes, from `sky view` to `reflection mip 6`;
- `contribution compute`;
- `V1 surface present`;
- `atmosphere stars`, which appears in stars-on only.
In the GPU-command list, the `(wgpu internal) Pre Pass` staging groups appear as `(wgpu internal) Staging` entries.

## Reading

- **D0.4 has a counter.**
  - Per-encoder bytes to and from device memory, written to and read from separately, are available from a frame
    capture replay on this M3 Max. They are not available per attachment.
  - Attribution to one attachment comes from the encoder that owns it, or from a variant delta like the one below.
  - The plan's traffic rows can use this method where an encoder maps cleanly to its attachments.
- **The star variant's forward store is visible.**
  - With stars on, `synthetic forward opaque` writes +2.18 MiB more. D1.2 says forward depth turns from discarded to
    stored in the star variant, because the star pass samples it: that is +7.91 MiB logical.
  - The measured physical store is about 28% of that logical figure, which is consistent with lossless framebuffer
    compression on a mostly dark night frame. This is a reading, not a proof.
  - The star encoder itself adds 0.59 MiB read and 1.22 MiB written.
  - Physical bytes for the whole frame rise 2.43 MiB, which is within the noise below.
- **Physical bytes are far below logical bytes.** Forward opaque writes 6.82 MiB physically against 23.73 MiB of
  logical stored attachments with stars off. So logical-bytes cost models, D1.2's included, overstate Apple physical
  store traffic on dark content. The plan's rows should keep logical and physical columns separate.

## Limits

- **One replay per trace, with no repeat.**
  - Encoders that the star variant doesn't touch still moved between the two runs: `(wgpu internal) Indirect draw
    validation` −1.00 MiB read, `MTLBlitCommandEncoder 1` −0.75 MiB read, `synthetic Sun horizon heights` +1.04 MiB
    read.
  - Treat about ±1 MiB per encoder as the noise floor until repeated replays measure it.
  - The whole-frame Δ of +2.43 MiB is inside that band. The `atmosphere stars` row and the +2.18 MiB forward write are
    the only star-attributable figures.
- **The machine was shared.**
  - Load was high, and other processes' GPU work interleaved with the frames.
  - The byte counters are per encoder, so they should be robust to this. The timeline peaks and timings are not.
- **The stars-on cost column is unreliable.** In stars-on, Xcode's *Execution Cost* column gives
  `atmosphere diffuse irradiance` 69.27% and every synthetic encoder 0.000%, while stars-off spreads cost normally.
  Draw-counter profiling was still running (about 6%) when the table was read. Use no cost or timing figure from the
  stars-on table. The memory columns had already populated in both traces.
- **Replay of a saved capture is not the live game.**
  - The figures cover one midnight frame of the offscreen valley capture, not the route.
  - Day frames, the route (D0.1–D0.2) and the iPhone tile (M12) remain *unmeasured*.

## Consumers

- **G3t, the [Apple tile plan](../../plans/APPLE-TILE/PLAN.md).** D0.4 is done with this method. D1.3's
  storage-mode log and M12's traffic rows should report physical per-encoder bytes beside the logical cost model.
- **G3s, M3's "physical bandwidth replay".**
  - The `atmosphere stars` encoder costs 601.88 KiB read and 1.22 MiB written per frame at 1920×1080 on M3 Max.
  - Enabling stars also makes forward store depth: +2.18 MiB written, physically.
  - The optional receiver variant (about 23.73 MiB logical per slot) would need its own replay. It is *unmeasured*
    here.
