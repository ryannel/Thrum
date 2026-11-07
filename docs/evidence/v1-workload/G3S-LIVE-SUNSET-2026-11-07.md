# G3s live sunset motion — 7 November 2026

Earlier sunset evidence sampled one frame per 30 s of solar time. This page checks the sunset at live cadence: 300
consecutive 60 Hz ticks, 17.70–18.10 h, with temporal history carried from frame to frame.

[Live sunset clip](g3s-first-light/live-sunset.mp4): native Metal offscreen frames, one per world tick, played at
60 fps. Readback paces the frames, so the clip makes no presentation or frame-time claim.

## Route

`THRUM_CAPTURE_LIVE_TICKS=<n>` adds `live-sequence/` to a capture. It sets 17.7 h once, resumes the world, then
advances exactly one fixed tick per readback. A day is 18,000 ticks, so each frame is 4.8 s of solar time.

## Defect found: crest speckle

![Ticks 195–196 at about 17.96 h. Top: before, with orange speckle on the shadowed floor. Bottom:
fixed](g3s-first-light/live-sunset-speckle.png)

- Between ticks 140 and 202 (17.89–17.97 h), orange speckle wedges flashed on the shadowed floor for one or two
  frames at a time, with up to 1,140 floor samples lit in a frame. The 30 s sampling had shown only a few
  isolated red specks.
- Visibility debug captures put the cause in the last Sun cascade. Rays toward a Sun about 0.6° up clip distant
  crests through thin wedges of terrain. Both faces of such a wedge are nearly edge-on to the light, so their
  rasterized slivers can miss texel centres. The depth map then has holes, which move as the Sun turns.
- **Fix.** While the Sun's elevation sine is below 0.1 (about 5.7°), every receiver also takes the minimum with
  the [terrain height-map horizon](G3S-TERRAIN-HORIZON-2026-11-07.md). The horizon only darkens, and it holds
  terrain occlusion at any distance without rasterizing slivers. Above that elevation nothing changes.

## Checks

- Sampling every second tick from 140 to 298, the before capture had lit floor samples in almost every frame up
  to tick 202. The fix has none in any of them. It also removes the residual band at solar frame 77 that the
  [sunset caster fix](G3S-SUNSET-CASTERS-2026-11-07.md) left.
- Ray-truth scores are unchanged or better at solar frames 22–35, where the guard is active: within 500 m,
  frame 34 goes from 0.049 to 0.046 mean error. Sequence frames 20–21 are unchanged.
- In the fixed clip, the largest frame-to-frame changes are ticks 1–9, where the ridge shadow sweeps across the
  floor (118 → 100 codes, then steady), and ticks 259–260, where direct sunlight ends.
- **Cost.** A new `THRUM_BENCH_HOURS` option sets the frame bench's time of day. On a contended GPU (load
  average 8.5), the paired run gave forward+sky p50 of 1.79 → 1.86 ms and pass-sum 4.59 → 4.74 ms (+3.4%) at
  17.9 h. At noon there was no change: 2.50–2.52 ms either way.
- `./dev check` passed. Chrome's `sky-reflections` case passed with Wasm SHA-256
  `dd039cbd0af555d098a57cbee9da6d2866dde4a351f0804c4e641cb01b3ac34f`.

## Independent review

A fresh-context reviewer decoded and differenced all 300 frames of the clip. It found no sparkle or speckle on
the floor or sky, a smooth monotonic shadow arrival with no relighting, progressive fading on the ridge faces, and
no colour or brightness pops. It flagged one-pixel runs on ridge-over-sky silhouettes that changed by 10–20
codes for a frame or two. The lossless frames do not contain them: across rows 300–460 and ticks 10–299 there is
no one-frame spike of 6 codes or more, and the pixel it cited holds 223 throughout. They come from the clip's
H.264 encoding.

## Open

- There is no GPU oracle for the low-Sun guard; it is verified by ray truth and captures only.
- The ridge shadow takes about nine ticks (43 s of solar time) to cross the floor, while a 0.53° disk would take
  about two minutes. This is the final-sliver lag already recorded on the penumbra page.
