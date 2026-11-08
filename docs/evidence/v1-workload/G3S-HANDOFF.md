# Thrum V1 handoff

Updated 8 November 2026. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

## Current checkpoint

- A connected west spur removes the large lookout depth break at the
  held Sun camera and preserves the far ridge within one pixel across
  a ±20° pan. The six-stop Metal/Chrome route and both controls pass at
  the existing tolerance on scratch root `32ed89be`. The remaining
  17.69 h stripe isolates a direct-Sun leak. At x640 y434 the terrain
  horizon reports 49/255 visibility, the cascade zero and the exact
  triangle reference zero. The 96-tap mirror reproduces the leak.
  Roughly 830 m between taps skips the 12.54 km crest, and the cascade
  fade exposes the wrong horizon. The candidate remains a diagnostic.
  CPU max-hierarchy prototypes restore zero disk at that pixel on
  exact triangles and on the 32 m raster. Across 2,496 sampled rays,
  the log march's largest visibility error is 44.2 percentage points.
  Flat raster cells reduce it to 6.4 with 0.2 points of worst light
  leak. Bilinear leaves reduce absolute error to 2.6 but can extend a
  discontinuous caster across cells. A Metal diagnostic replayed 2,499
  rays against Depth32Float depths reconstructed from the spur mesh.
  The three witness pixels went from 13.8%, 19.4% and 23.4% disk
  visibility to zero. The flat-cell CPU and GPU agreed within 0.00055
  percentage points. Against the original triangles, the GPU hierarchy
  had 0.183 points of maximum leak and 6.462 points of maximum absolute
  error. In 14 paired timestamp rows under shared load (1-minute load
  15.88 to 22.67), p50/p95 was 87/533 µs for taps, 296/688 µs for the
  hierarchy and 438/1,065 µs for reduction plus traversal. This compute
  witness includes neither the caster draw nor forward shading. Stepping
  directly across integer tile boundaries cut p95 node tests from 195 to
  96 and produced identical visibility on all 2,499 terrain rays and 30
  sparse boundary cases. Across six ordered pairs under shared load
  (1-minute load 18.54 to 22.04), the new traversal's median was 82 µs
  versus 217 µs for the previous hierarchy. Reduction plus traversal was
  121 µs versus 42 µs for legacy taps. The actual V1 Metal graph at
  `596746f` cut the 97-frame spur sweep's worst stripe contrast from
  122.42 to 61.16 red codes. The original witness fell from 228 to 111,
  matching the no-direct-light control. Dawn, noon and midnight held
  images were byte-identical. On the cooked route at 17.6917 h, paired
  GPU pass-sum medians were 4.99–5.00 ms for taps and 5.69–5.79 ms for
  cells (1-minute load 34.53–34.93). Each of the 320 frames submitted
  the cooked contribution and returned a valid timestamp batch. This
  is offscreen, with no presentation claim. The receiver-aware leaf at
  `938fbfb` now passes the Metal rasterized 5% ramp and 12.54 km crest
  test and retains the 61.16-code sweep result. The repeat's GPU medians
  at 17.6917 h are 4.92–5.10 ms for taps and 6.03–6.05 ms for cells
  (1-minute load 21.52–22.26).
  The composed prototype over `ce848011` reduces the 300-frame connected
  spur's peak stripe contrast from 100.36 to 75.38 red codes. It reproduces
  an intermittent 16-code fetched/mapped difference in the near receiver
  plane. Smoothing that plane passes all 13 Metal shadow tests and four
  fetched/mapped comparisons, but fails strict discarded-frame equality
  at eight pixels (one code). Repeated captures at the same endpoint match.
  Keep the strict test. The prototype remains off main.

  The far cascade's 34-code sunset step occurs with a steady 54-texel
  radius and six blocker-search hits. Doubling wide last-cascade taps
  reduces the worst one-frame step to 19.75 codes and raises median GPU
  pass sum from 4.46–4.48 to 5.33–5.35 ms (interleaved 1080p60 Metal,
  shared load 19.69–20.89). Keep the 16-tap path. A 96 m blocker cutoff
  gives a smaller step but has no compact-caster coverage guarantee.
  Routing terrain to the horizon and sampling compact shadows over the
  open disk cap reduces the worst drop from 34.0 to 14.6 codes on Metal.
  The remaining drop comes from the terrain disk crossing. In a cooked
  upper-cap witness 160 m from the receiver, a 24 cm caster misses both
  depth maps. The same-centre 2.4 m control writes two last-map texels
  and drops visible disk from 34.51% to 3.92%. The 1.18 m last-map texel
  loses the narrow caster before filtering. A tighter 82 m scratch fit
  gives 0.16 m texels and restores the caster to the last map. The sparse
  search still misses it one tick later despite six matching depths.
  A bounded texel search over three caster positions cuts mean absolute
  disk error from 3.62 to 1.90 percentage points against the cooked
  quad/terrain reference. It removes the full-light fallback but leaves
  one-tick rebounds. A finer local map removes the rebound at all three
  caster positions. Max-depth blocks match all 110 exhaustive-scan
  frames and cut forward p95 from 12.70–13.63 to 5.07–5.42 ms in
  interleaved Metal runs (shared load 18.06–18.13). The extra reduction
  adds about 0.17 ms to the far bucket median. The compact fit is still local.
  The fine terrain raster restores a 31 m crest but misses narrower
  peaks. At 68 m a flat-topped ridge leaves 24.85% of the disk visible
  at the rendered phase, versus 34.51% with the 64 m fine scan and
  27.84% with a 96 m scan. A hierarchy with original triangle leaves
  recovers it at 24.71%. Its maximum error across six 90-frame fixture
  runs at one receiver is 0.34 percentage point for the centre azimuth.
  A cooked off-axis crest exposes the limit: 35.29% visible by centre
  azimuth versus 22.49% by 3D disk rays. A 32-column Metal approximation
  gives 21.57%. Sharing the traversal cuts 1080p diagnostic GPU p95 from
  55.13–56.31 to 20.37–21.03 ms (load 11.89–12.54), within one code
  over all 76 frames. Depth-guided 2×2 visibility reuse still produces
  139-code outliers after refinement. At sampled 2×2 tile positions,
  a conservative union retains every individual receiver's candidates
  and yields 13/43/175 leaves at p50/p95/max. Test workgroup-local
  sharing with exact per-receiver visibility. A fixed full-frame list
  can cost 33 MB for just 16 IDs per tile at 1080p. Close clearance,
  compact receiver reach, aerial ownership, discarded-frame equality
  and Chrome remain open. Reference rows use rendered solar time.
  [Maximum Mipmaps](https://pure.mpg.de/pubman/item/item_1325622_5/component/file_3590464/i3d08.pdf).
  Exact builds, raw rows and clips are in `tmp/g3s-sunset-lip/`. The
  diagnostic source and rerun command are on `codex/g3s-horizon-metal-wip`.

- The 17.4 h sunset stripe follows the lookout's authored depth edge. In
  the held Metal valley at x=320, adjacent rays hit terrain at 1,826 m
  and 65 m. The near facet faces the viewer while its interpolated
  normal points behind it. A scratch west taper halves the local red
  step (52→26 codes) and moves the edge 15 px. A shader-only face-normal
  experiment gives 29 codes and adds a second contour. Across 97 held
  solar stops the largest step remains 144/136/137 codes for baseline,
  taper and shader. Independent terrain rays put the far receiver in
  full shadow by 17.60 h while the near receiver sees the whole disk at
  17.64 h. This explains the stronger stripe later in the sweep. Keep
  both experiments as diagnostics. The next look pass should change
  the authored lookout and check the full sweep. The clip and ray
  witness are in `tmp/g3s-sunset-lip/`. Ground E can land independently.
- The M3 Max's Xcode 27.2 Instruments probe completed 200 star-bearing
  frames but exported only the ray-tracing activity counter. The public
  Metal query exposed GPU timestamps. Actual attachment bandwidth needs
  an explicitly configured capture. [Apple's bandwidth workflow](https://developer.apple.com/documentation/xcode/measuring-the-gpus-use-of-memory-bandwidth).

- The production optics and observer-wrap referees pass on `67c69313`.
  Metal preserves flux to 0.0571% across 36 footprints, at centre and edge
  from 720p to 2160p, and covered centres emit zero across both mask axes.
  The central magnitude 3/6 SDR control yields 207/58, 211/74 and 212/86
  green codes at matched exposure. The game basis keeps known equatorial
  directions and matches the slider wrap. Chrome's cooked valley build
  `b870c1ee` rendered 00:00 and 24:00 byte-identically. [Review brief](../../design/modules/ATMOSPHERE-PLAN.md#production-star-optics-review-brief).
  Raw tests and rerun commands are in `tmp/g3s-star-catalog/current-main/`
  and `tmp/g3s-star-catalog/NOTES.md`.

- Star graph reconciliation `d2a5342` preserves main's Occlusion
  prepass, pyramid, draw and count copy. A Metal test captures night and
  day under Frustum and Occlusion with valid timestamps and actual list
  counts. Chrome's 100,513-instance oracle matches Direct byte for byte
  at all 16 views. On the canonical cooked root `59f0484f`, all six
  Metal/Chrome views and both controls pass the 1-code tolerance. The
  largest cross-target cell difference is 0.16044. Only the old midnight
  baseline drifts (19.2 median RGB codes). Same-build stars-off controls
  attribute the broad change to exposure. Stars add local sky contrast
  while preserving fully covered terrain. The held review clears a
  midnight baseline refresh at unchanged tolerance. The six-stop recheck
  with only midnight grids refreshed in scratch passed with at most
  0.006 codes of baseline drift. G7 made 512² ground E canonical at
  `8f1dcf76` while this check ran. Retain its new baseline and recheck
  the star build against ground E before admission.

- The offscreen Metal recorder uses the normal frame submission and SDR
  blit with a texture in place of the drawable. It records one real
  world tick per encoded frame and its GPU pass timestamps before the
  pixel copy. The cooked valley emergence clip has 1,440 consecutive
  ticks at 1920×1080, encoded losslessly at 60 Hz for 24 seconds
  (18.4→20.32 committed solar hours, with rendered time one tick behind).
  GPU pass-sum p50/p95/p99 was 5.08/23.40/31.39 ms
  during shared solver load (1-minute load 29.37–35.82). Those rows cover
  the changing scene and blit. Independent decoded-frame review
  tracks eight bright stars without loss over the last six seconds and
  finds a gradual three-code sky lift. Continuous playback, slow-pan
  review, paired midnight cost and attachment traffic remain open.
  To repeat, set `THRUM_CAPTURE_SCENE` to a fresh output directory,
  `THRUM_CAPTURE_MOTION_FRAMES=1440`, and the normal scene/store inputs.
  Use `THRUM_PARAMS` with running-world solar time 18.4 h, yaw 0°, pitch 7°.
  Pipe the game binary's stdout to `ffmpeg -f rawvideo -pixel_format bgra
  -video_size 1920x1080 -framerate 60 -i pipe:0 -c:v libx264rgb -crf 0 motion.mkv`.
  The JSONL log stays beside the capture. Exact inputs, logs and command
  are in `tmp/g3s-star-catalog/current-main/offscreen-emergence/` and
  its parent `run-offscreen-motion.sh`.

- Ground E `70d2baf8` passed the six-stop Metal and Chrome comparison and
  both controls with the same one-code tolerance. Only the two midnight
  grids changed. In the matched 10-second native pans, yaw moves −8° to
  +8° through the game's control, with stars on and off in the same
  build. Each clip has 600 consecutive ticks and GPU samples. The yaw
  edit resets retained instance-transform history as in the window.
  The independent decode found no RGB changes under the terrain mask.
  Two faint tracks persisted through all 600 frames, with peak changes
  of at most two codes per frame. Sampled frames and decoded motion
  were reviewed. Continuous playback is still open. Repeat with
  `THRUM_CAPTURE_PAN_TO_DEGREES=8` and start yaw −8°, pitch +7° and
  midnight. Capture and route records are in
  `tmp/g3s-star-catalog/current-main/`.

  | Midnight 1080p run, 200 submitted frames each | GPU pass-sum p50 / p95 (ms) |
  | --- | ---: |
  | Stars off 1 | 6.94 / 24.05 |
  | Stars on 1 | 8.53 / 28.54 |
  | Stars on 2 | 9.79 / 26.28 |
  | Stars off 2 | 16.97 / 42.66 |

  Solver load confounds the cost. The two control rows vary more than
  the apparent effect. A separate HDR load and store accounts for
  33.2 MB of logical RGBA16Float attachment area at 1080p. Physical
  traffic and display cadence remain unmeasured.

- Star integration `00c2645` is on `codex/g3s-stars-integration`. Both
  graphs preserve Frustum count copies and nonblocking slot retirement.
  Each native Metal mode matches the previous 132 held images byte for
  byte. Chrome records a continuous eight-second midnight pan and 30-second
  twilight sweep with zero world overloads. Independent review tracked
  141 isolated stars through every pan sample. Aperture brightness varied
  9–12% at the median, and a high-sky patch retains a 1.55-code late rise.
  Chrome's paused midnight replacement rejects the old capture and
  reproduces the full frame.
  `./dev check` passed 518 tests. The Mac helper found no foreground
  window. Keep production admission open. This branch's twilight shoulder
  changes default output even with stars off. Qualify it with the normal
  valley launch before refreshing G7's canonical baseline. The exact
  builds, clips and commands are in `tmp/g3s-star-catalog/NOTES.md`.

- Daylight selector WIP `3407173` fades the audited star field out at
  68.1–71.5 klux for the valley's default camera. Fixed exposure, an
  unsupported projection or the diagnostic overlay keeps the full field.
  The bright graph drops the star draw and discards opaque depth. All 132
  Metal captures and 42 Chrome daylight poses match the always-draw build
  byte for byte. Both Chrome midnight views also match. `./dev check`
  passed 515 tests. Independent review verified the fixed Earth and
  display bounds. The 120 Hz offscreen row could not retire half its
  baseline callbacks under shared load. The separate same-binary inline
  run omits the pass in all 400 selected frames. Full GPU span p95 was
  4.33–6.31 ms with the pass and 4.39–4.41 ms without it, at shared load
  173–176. The pass sum grows because the measured intervals overlap.
  There is no demonstrated whole-frame speed gain. Reconcile the culling
  count graph on main and qualify live motion before admission. Exact
  hashes, measurements and restartable commands are in
  `tmp/g3s-star-catalog/NOTES.md`.

- Twilight shoulder WIP `6a2dbf6` preserves physical sky radiance and HDR
  storage scale while softening incident exposure recovery in deep twilight.
  Across the Metal referee's 18.85–19.65 h pair, exposed linear luminance
  changes from a baseline 0.03294 → 0.04980 to 0.03163 → 0.02408. The
  matched Chrome patch darkens from median RGB 33/47/76 to 42/42/42
  over that interval. Independent review found a small residual late rise
  on the right and legible stars and terrain at midnight. The rock's lit
  face remains faint. The checked daylight and early twilight captures
  match the baseline. `./dev check` passed 514 tests. The change stays in
  the existing GPU meter dispatch. The 1080p inline GPU pass-sum p95 at
  19.65 h overlaps the baseline (5.18–5.23 versus 5.13–5.17 ms, shared
  load 31–32.5). Live motion remains open. Raw pairs and rerun commands are in
  `tmp/g3s-star-catalog/NOTES.md`.

- The opt-in star pass is verified on Metal and Chrome at local WIP
  `1fa3228`. It shares the atmosphere's checked altitude and the frame's
  exposure. Metal on/off changes 332 sky pixels at midnight, with the held
  dawn, noon, sunset and twilight frames byte-identical. The centre-depth
  mutant leaks into 58 sky pixels. `./dev check` passed 511 tests.
  Independent review favours the tighter provisional full-field still.
  Exact captures, preliminary paired costs, input hashes and rerun commands
  are in `tmp/g3s-star-catalog/NOTES.md`. Two historical catalogue
  pages yield 72 strict OCR rows out of 100, with 12 recognized spot checks
  exact. At the first right-ascension hour rollover, four manual spots match.
  Another page has a format-valid 48.3 → 18.3 OCR error, so raw rows stay local.
- WIP `a61bb80` verifies both graph variants on the same Metal target.
  Without a celestial basis, the graph removes one pass and discards opaque
  depth. With a basis it derives the depth store from the star pass and
  retains opaque → stars → tone order. All 132 prior opt-in Metal valley
  frames match byte for byte. The workspace check is green. Production
  daylight eligibility, the shipping catalogue and motion remain open.
- Tight footprint WIP `500ef37` keeps HDR flux within 0.0571% across 36
  Metal scratch cases. An above-horizon control twice Sirius's illuminance
  changes daylight by at most one RGB code and brightens its central footprint
  at night at 720p, 1080p and 2160p. All non-midnight held Metal and Chrome
  captures match the wider build. The independent review confirmed tracking
  in 49 sampled Chrome poses and reported faint-star phase variation for live
  follow-up. Workspace check passed 513 tests. Benchmark ranges, exact builds
  and reproduction commands are in `tmp/g3s-star-catalog/NOTES.md`.
  The local `harvard-1908/layout-pages/scan-table.sh` retains each completed
  scan and transcription for restart. The production cutoff and catalogue
  admission remain open.
- Catalogue WIP `98e0be1` brings 6,632 original Table I rows into the opt-in
  valley. Independent review verified all 183 page headers and 23 rollovers,
  recovered 16 source rows and reproduced the 106,112-byte pack independently
  of the comparison catalogue. The source transcript, date and hashes sit
  beside the asset. `./dev check` passed 513 tests. On Metal the matched
  noon frames are byte-identical and all 4,820 fixed-midnight changed pixels
  lie above the ridge. Chrome shows gradual emergence in 33 settled poses
  and coherent motion in 49 sampled pan poses. The same upper-sky rebound
  after 18:51 occurs with stars disabled. The exposure follow-up is above.
  Exact hashes, raw pairs and rerun commands are in `tmp/g3s-star-catalog/NOTES.md`.
- M1 closeout at `b8f230e7` used the same cooked scene hash on Metal and
  Chrome. The Chrome compositor sweep ran from 17.659 to 24.053 h with 75
  samples. All eight overloads were inside screenshot brackets. The same
  Wasm without screenshots ran through midnight with zero overloads and
  82 shed views. The completed Metal foreground run captured 465 samples
  over 92.53 s, past midnight to approximately 01:06 by the window's tick.
  Independent review passed the shadow crossing, exposure progression and
  midnight landmarks on both targets. All nine Chrome held scene images
  matched the selected ground look byte for byte. The [G7 fixed route](G7-CHROME-VALLEY-2026-11-07.md#showcase-launch-and-fixed-review-route)
  independently covers held daytime on both targets.
  The whole Mac run presented 12,200 frames with zero world overloads
  and 827 dropped views at shared load 35–39. Sample 431 shows a
  67.287 ms encode in the window title. The sampled visual result closes
  M1. Physical intervals and short hitches remain open for G3b. The full
  clip, samples and counters are under `metal-foreground-continuation-163219/`
  in the closeout directory. Repeat with `sh run-metal.sh <fresh-name>`
  while the unlocked Mac keeps the game frontmost.
- Paired Metal offscreen runs at 1080p compare `9f13c5f6` and `b8f230e7`
  against the same cooked store under shared load. Median GPU pass-sum
  p95s were 22.66 → 25.15 ms at noon, 10.36 → 9.77 ms at the crossing,
  8.33 → 8.33 ms at 18.97 h, and 8.00 → 8.54 ms at midnight with the
  additional paired repeat. Midnight callback p95 was 4.03 → 4.99 ms
  and p99 6.60 → 10.27 ms. The tail increase survives both orders and
  also appears in the measured render stages. Encode p95 is unchanged in
  the medians, while upload p95 rises from 1.26 to 1.46 ms. Adjacent
  `80e519ec` → `b8f230e7` pairs with the overlay off give a callback
  p99 rise in the first order and overlapping values in reverse under
  shared load 30–35. That does not isolate the overlay as the cause.
  Raw data, executable hashes and
  reproducible commands are in `tmp/g3s-showcase/night-design/m1-closeout-b8f230e7/`.
  These observations do not establish display cadence.
- The browser still fetches Portable. The GPU loader resolves only among products present
  in the fetched generation, in device preference order. A Metal test with BC enabled
  exercises a manifest that advertises compressed variants without fetching them.
  [Chrome valley evidence](G7-CHROME-VALLEY-2026-11-07.md)
- Inspection of the [aerial comparison](G3S-SHADOWED-AERIAL-2026-11-07.md) supports keeping
  the terrain-shadowed aerial path. The reduced orange glow follows the ridge occlusion.
  The air map picks the strongest caster for the Sun's centre. The strongest caster can
  change across the disk, so replacing the surface march needs a separate geometry check.
  The Metal compute ordering diagnosis remains unconfirmed. Fresh Chrome stills retain
  the blue twilight gradient and warm horizon. The stepped ridge and featureless ground
  still read as diagnostic terrain.
- The [sunset comparison](G3S-SHADOWED-AERIAL-2026-11-07.md#mac-cost-shared-load) measured a 29%
  higher scene GPU median under very heavy shared load. Reusing one horizon march in the fade
  band preserved all 191 native frames and 34 Chrome images. Its separate ABBA shows no clear
  whole-frame speedup. Rerun and hashes are in `tmp/handback-review/horizon-fade/`.
  The resumable comparison is under `tmp/handback-review/aerial-cost/`. From the repository
  root, run `sh tmp/handback-review/aerial-cost/build-pair.sh` and then
  `python3 tmp/handback-review/aerial-cost/run-pair.py` with access to Metal. The script uses
  exact commits `9b4bea6` and `2db5039`, the cooked valley store and a 17.8 h solar start.
  It retains partial attempts and skips completed slots on restart. When reusing a Cargo target
  for Git archives, touch the archived Rust files before building and verify the executable
  hashes differ. Cargo accepted the archived timestamps on the first attempt and reused the
  baseline binary for both commits.
- Metal reproduced and verified the below-horizontal horizon fix. The analytic plateau
  and near/last blocker-window oracles now pass, with identical valley images on Metal and
  Chrome. [Terrain horizon evidence](G3S-TERRAIN-HORIZON-2026-11-07.md)
- Chrome's cooked edit and replacement probe found no missed next animation callback at
  the measured swaps. Retain the current loader for the valley and carry the tripwire
  forward for heavier authored edits. [Chrome measurements](G7-CHROME-VALLEY-2026-11-07.md)
- G6's re-split controls and macOS runner commits are approved. The native starter and
  cooked material shader reload are separate reviews. ExposureZones can use the
  existing pre-tone exposed luminance seam. See `tmp/g6-boundary/NOTES.md`.
- The G6 stone circle now reports Metal timestamps with zero synthetic instances.
  Skip the empty transform-history pass and match timestamps against each submitted
  slot's layout. In two interleaved sets of four 120-frame live runs, all baseline
  windows report GPU timing unavailable and all candidate windows report it. The
  1600×900 Metal captures are byte-identical. In the repeat after the workspace
  check, shared load was 25–27.2 and render-worker p50/p95 was 1.40–1.52/3.84–4.94 ms
  before and 1.44–1.49/4.99–7.02 ms after. The tails are unresolved under this load.
  All-clear diagnostic render passes can still lack Metal counters. Rerun
  `python3 tmp/handback-review/empty-timing/run-pair.py <fresh-prefix>` on Metal.
  Raw logs, input hashes and the G6 fixture are in the same directory.
- CPU rays through the G6 stone scene classify 130 of 140 strong streak samples
  fully blocked, three partially blocked and seven clear. The Metal map leaks more
  than 0.25 visibility at 115 of those pixels. The global bias stays. Smoother
  authored geometry reduces the marks in a held view and changes the stone bounds.
  The exact capture, 64-ray comparison and asset variants are in
  `tmp/handback-review/empty-timing/ray-truth/`. G6 owns its starter art.
- The same morning capture has coherent arcs in the stored fp16 sky, before SDR
  quantization. Manual bilinear and double LUT height move the arcs. The green
  radiance p99 change is 0.027% and 0.098% respectively over the sampled sky.
  Neither probe is selected. Scripts and curvature images are in
  `tmp/handback-review/empty-timing/sky-bands/`. Resume after the ultra-review fixes.
- The ultra-review atmosphere fixes keep the five fixed Metal valley phases
  byte-identical. The two-bank Sun query is shared by full and single aerial
  scattering. In the repeated 1080p ABBA at shared load 25–29, scene GPU p50
  was 6.20–6.36 → 6.27–6.68 ms and aerial p50 was 0.096–0.098 → 0.098–0.124 ms.
  The second candidate sample had elevated tone, forward and aerial tails.
  No throughput claim. Raw logs and hashes are in
  `tmp/ultrareview-2026-11-07/atmosphere/`.
- The ultra-review frame and clock fixes are in the
  [bounded-frame evidence](../render/V1-BOUNDED-FRAMES-2026-11-04.md).
  The all-clear Metal timing issue remains separate.
- The cooked valley probe retained the existing sky filter. Vertical cubic
  reconstruction changed morning curvature p99 from 0.157% to 0.079%, but
  changed the sampled SDR sky by at most one code. Baseline midnight was
  entirely black. Capture and scoring scripts, matched frames and source hash
  are in `tmp/g3s-showcase/sky-contours/`.
- The first [M1 night checkpoint](../../design/modules/ATMOSPHERE-PLAN.md)
  lights the cooked valley at midnight in Metal and Chrome. On Metal, the prior
  black frame becomes a sky of RGB code 63–68 in the top 130 rows. The sampled
  foreground rectangle reads 20–29. Dawn, noon, sunset, twilight and the held Sun
  frames change by at most one RGB code per channel. The independent capture
  review passed the held ground camera on both paths. A sharp foreground shadow
  transition around 17.71 h also occurs in the earlier terrain-horizon capture.
  The sunset sequences stop before full night. An upslope midnight camera, a
  matched nightglow-off control, continuous late twilight, and live motion remain
  open. The integrated workspace check passed 465 tests and the rebuilt Chrome
  valley reproduced the reviewed midnight PNG byte for byte. The new meter's
  lower EV limit is exercised in the GPU arithmetic test.
- The production Metal sky sampler agrees with the dense f64 reference to
  0.413% across five rays and both horizon limits at 0, 1.8 and 15 km.
  Horizontal night irradiance differs by at most 0.087%. Toggling the
  source leaves the Sun's RGB sky LUT byte-identical and changes diffuse SH.
  The shipping aerial texture and composite agree to 0.014% at 1, 10 and
  30 km, using interpolated depths. The real scene graph rebuilds sky, diffuse,
  aerial and every reflection mip on source changes, then resumes caching.
  These are Metal tests of the named approximation. Reflection pixel
  agreement remains open. Raw rows and checks are in
  `tmp/g3s-showcase/night-design/m1-transport/`.
- Chrome's paused cooked valley was sampled every 0.11 solar hours from
  18.3 to 22.3 h, with a separate +12° midnight view. The foreground sample
  falls from mean RGB code 98.56 to 24.91. The upper sky drops to 61.14 at
  18.86 h, rises to 65.78 at 19.41 h, then settles at 65.21. Compare raw
  radiance and incident EV before judging that rise. The first run stopped
  presenting when its tab became hidden. The retry brings the qualification
  tab forward through CDP and passes. Use
  `node tmp/g3s-showcase/night-design/m1-transport/chrome-late-twilight-front.mjs --case valley --output <new-directory>`
  with the built default page. The sampled clip, contact sheet and pixel
  rows are in `m1-transport/chrome-late-front/`. These are held edits.
  Independent review found a warm S contour on the metal ball. The
  diagnostic cube without convolution loses the contour and soft horizon.
  At 1,024 taps per mip the contour largely disappears and the horizon
  stays soft. Redistributing taps to the rougher mips lowers Chrome's
  sphere-region error against dense from 0.87–1.10 to 0.33–0.39 RGB codes
  over frames 05–09. The independent review saw the same improvement on
  Metal, with faint residual streaks. Two interleaved Metal 1080p offscreen
  pairs per order reject this schedule on cost. Median run p95s:

  | Solar hour | Scene GPU ms, before → after | Atmosphere GPU ms, before → after |
  | --- | --- | --- |
  | 12 | 4.41 → 8.29 | 1.03 → 2.32 |
  | 18.97 | 6.26 → 10.10 | 1.11 → 2.40 |

  Shared load was about 34. Both pairs agree on the direction. Same-count
  angular probes move the streaks; the split-horizon footprint experiment
  leaves the error essentially unchanged. The four-lane reduction keeps
  the denser per-mip samples and reduces the contour with a smaller cost
  increase. In the final Chrome build, a 55-pixel-radius interior sphere
  region over frames 05–09 differs from the rejected scalar schedule by
  at most one RGB code. Its mean error against the dense reference falls
  from 1.42–1.65 to 0.54–0.68 codes. Faint warm streaks remain. Metal
  held stills retain the soft reflected horizon and both midnight materials.
  The diagnostic four-lane ABBA/BAAB at 1080p offscreen under shared
  load 35–40 measures noon scene GPU p95 4.51 → 5.04 ms and late twilight
  6.45 → 6.91 ms. Atmosphere p95 is 1.05 → 1.26 ms and 1.13 → 1.34 ms
  respectively. Late callback p95 increases in the medians (3.52 → 4.73 ms)
  and varies widely between runs. Exact-source ABBA/BAAB against
  `4fecd095` at load 44–45 puts late GPU p95 at 6.83 → 7.23 ms
  and callback p95 at 3.71 → 4.05 ms. Noon is heavily contended
  (scene GPU p95 13.17 → 13.93 ms, with individual values from 4.85 to
  24.43 ms). No live cadence inference from these rows. The independent
  review also reproduced a sharp foreground
  shadow change at 17.67 → 17.73 h in both Chrome builds while yaw changed.
  This cue and live cadence remain open. The exact capture builds are Metal
  `c6e7b102` and Chrome Wasm `4afa8b38` (SHA-256 prefixes). Under
  `m1-reflection-isolation/`, see `cooperative-final-metal/`,
  `cooperative-final-chrome/`, `cooperative-final-sweep/`,
  `cooperative-final-roi.log`, `cooperative-final-perf/summary.log` and
  `cooperative-perf/paired*/`. The rejected
  `packed-schedule.patch` and the diagnostic `cooperative-source/` remain
  there for reproduction.
  The +12° midnight camera cropped the rock. The matched +7° capture keeps
  both materials in frame and is now in the native and Chrome capture routes.
  On the foreground M3 Max window, 180 compositor samples over 36 s show
  the natural progression into night. The run reports 4,200 presents,
  4,429 render callbacks, 117 dropped surfaces and zero world overloads
  under shared load 45–48. The early shadow handoff loses the cast-shadow
  tails in one sample and then the warm rock key, while the sky and
  reflected sky continue smoothly. The camera is fixed. Sample spacing
  cannot resolve the shadow edge between those pictures. Chrome's 52
  natural compositor samples cover 18.26–20.26 h. Of 41 world overloads,
  39 fall inside screenshot brackets. A same-Wasm repeat polling only
  the game report has zero overloads and eight shed views over the
  observed interval under load 52–62. Independent review passes the
  late-twilight colour progression and readable night on both targets.
  A Metal replay at the live camera samples all 82 consecutive ticks
  from 17.651 to 17.759 h. Four foreground patches darken across the
  crossing, with a largest single-tick change of 3.19 RGB codes.
  The independent finite-disk terrain ray trace falls from full to
  0.80 at 17.680 h, 0.42 at 17.695 h, 0.08 at 17.710 h, and zero by
  17.725 h. Chrome's 37 held views follow the same terrain transition.
  The independent reviewer sampled the object shadow tails separately.
  One Metal patch at (1100,559) jumps from 97.0 to 108.7 RGB codes at
  17.6913 → 17.6927 h. The Chrome tail jumps from 96.9 to 108.5 at
  17.692 → 17.695 h. The reviewer added both object meshes to the CPU
  trace. Each affected tail stays fully blocked across 2,048 disk rays
  at both Metal ticks. Debug colour channels show the near-map blocker
  search jumping from local to distant range while the terrain horizon
  fades continuously. The closest-to-light depth is masking the local
  blocker as the ridge enters the map. A scratch 64 m light-depth
  restriction keeps the sphere tail at 97.0 → 97.1 and the rock tail
  at 101.2 → 101.1 across the failing tick. Dawn, noon, sunset, late
  twilight and midnight stills are byte-identical. The engine fix
  partitions the two existing cascades in light depth using an
  eight-near-texel penumbra reach. The last map fits all fallback
  receivers. It accepts depth-partitioned blockers against an absolute
  clip plane and evaluates spatial support in receiver-relative units.
  Metal verifies sloped taps on both sides of the plane; the first
  implementation leaked 0.5096 visibility on a fully blocked witness.
  The narrow-camera fixture proves a 60 m excluded caster and its 5 m
  receiver both reach the last map. The normal caster clamp still applies
  outside intentional upstream clipping. A standalone instance upload
  can leave the prepared height bound stale. The height estimate sizes
  the search while absolute clip ownership remains in force. The added
  GPU witness otherwise returned fully lit for an excluded blocker.
  The 82-tick Metal replay and
  37 held Chrome views preserve both object tails through the old pop.
  Independent review passed those sequences and matched stills. Noon
  differs at a few pixels on the rock; all other checked stills match.
  Capture builds are Metal `510e62ce` and Chrome `c6500ef7`
  (SHA-256 prefixes). See `partition-valley-release-{metal,chrome}/`.
  The same Chrome Wasm passed independent review of 62 unpaused
  compositor samples from 17.66–20.27 h. The screenshot run recorded
  19 overloads. The revised foreground Metal capture reported no
  foreground window. Metal's final evidence is the offscreen replay;
  keep the foreground review open. Full workspace check passed 499 tests.

  Exact Metal 1080p offscreen ABBA/BAAB against `9f13c5f6` at load 24–27,
  medians of four run p95s. The same textures and passes are retained.
  The sampled uniform grows by 16 bytes and the three caster uniforms
  by 48 bytes in total.

  | Starting solar hour | Scene GPU p95 ms, before → after | Render callback p95 ms, before → after |
  | --- | --- | --- |
  | 12.00 | 5.09 → 5.42 | 2.83 → 2.90 |
  | 17.69 | 9.10 → 9.20 | 3.11 → 3.14 |
  | 18.97 | 7.04 → 6.75 | 3.13 → 3.06 |

  Noon has extra work in the forward bucket. Crossing callback p99
  is 3.71 → 5.46 ms, with wide overlapping run ranges under shared
  load. Live cadence and paired Chrome cost remain open. Raw per-run
  and per-stage values and executable hashes are in
  `partition-perf/paired*/`.
  Physical presentation cadence remains open. Raw inputs are under
  `live-metal-foreground-retry/`, `final-live-chrome/` and
  `idle-live-chrome/`; see also `shadow-step-{metal,chrome}-pixels.log`
  `shadow-step-blockers-pixels.log`, and `shadow-step-local-depth-pixels.log`
  in the same scratch directory.
  Rerun
  `run-visible-window.sh <fresh-name>`, or run
  `chrome-natural-twilight.mjs` and `chrome-natural-unsampled.mjs` with
  `--case valley --output <fresh-directory>`.
- In matched nightglow-off and on builds, the two midnight cameras go
  from black to mean RGB codes 63.5–66.4 (sky), 25.0–25.4 (ground),
  47.2–47.9 (metal highlight), and 25.4–25.6 (diffuse rock) across Metal
  and Chrome. The noon frame changes by at most one code per channel.
  The scratch switch removes only `.with_nightglow()` from game lighting;
  the on-control's Chrome frames match production byte for byte. See
  `nightglow-off.patch`, `nightglow-control.log`, and the on/off capture
  folders in `m1-reflection-isolation/`. The Metal source-switch referee
  above checks the solar LUT separately from the displayed scene.
- The production forward and tone passes store a neutral dim card at
  0.00013661385 in the HDR attachment and display code 21 on Metal and Chrome.
  The card emits the analytically known radiance of a 1% Lambert surface
  under 0.000671 lux. This isolates storage from BRDF and SH approximation.
  Both exposure sides transition between fixed black and metered output.
  Metal repeats after dropped encodes and captures. Native diagnostic
  midnight pixels are byte-identical after GPU replacement. Chrome's cooked
  valley canvas is byte-identical at midnight after two actual WebGPU device
  replacements. Repeat via the attachment and V1 recovery Rust tests and
  `m1-attachment/chrome-night-recovery.mjs` under the night scratch directory.
  The Chrome attachment oracle uses a scratch Wasm with the production
  shaders, `browser-control.patch`, and `chrome-dim-control.mjs`.
  Results and actual Wasm hashes are in `m1-attachment/`.
- Raw files in `tmp/g3s-showcase/night-design/`: native `night-first/`, Chrome
  `m1-chrome-valley/` and `m1-integrated-chrome/`, the workspace log
  `m1-integrated-dev-check.log`, scalar reference `mixed-2.csv`, and checked profile
  `regenerated.rg16f`. To repeat on the current tree, rebuild the game and run
  the cooked-valley capture command below. Build the default Web artifact and
  run `node scripts/check_v1_browser.mjs --case valley --output <new-directory>`.
  Regenerate the embedded profile using the commands in
  `scripts/generate_earth_night.c` and compare the resulting bytes. The profile
  is generated from Thrum's model, with no imported sky data.
- Metal 1080p threaded offscreen valley, two interleaved pairs in each
  order with one-minute load about 16–21. Medians of four runs per variant
  against `143c9c82`, in ms:

  | Solar hour | Callback p50/p95/p99, before → after | Scene GPU pass-sum p50/p95/p99, before → after |
  | --- | --- | --- |
  | 12 | 1.25/2.61/4.39 → 1.28/2.77/4.67 | 3.85/4.89/6.40 → 4.23/5.38/6.31 |
  | 19.6 | 1.30/2.16/3.69 → 1.29/2.23/2.93 | 5.52/6.31/7.26 → 5.98/7.59/8.35 |

  The late-twilight scene GPU p95 rises 20%, with overlapping run ranges.
  Its aerial p50 stays within 0.081–0.109 ms across the pair. Isolated noon
  tails exceed the tripwire under shared load. Check live pacing before
  closing M1. Build, raw rows, binaries and hashes live under
  `tmp/g3s-showcase/night-design/perf/`. Run `sh build-pair.sh` from the
  repository root using that script's path, then `python3 run-pair.py` with
  Metal access, adding `--repeat` for the reverse order. Each writes its
  completed rows atomically and resumes by input hash. The first attempt
  refused identical executable hashes from a reused Cargo target. The
  build script now invalidates the render unit before building the candidate.
- G6's starter case-alias fix landed. The material reload is held separately
  on default/dev Web build isolation. See `tmp/g6-boundary/NOTES.md`.

## Exposure audit decision

The audit is `tmp/research-2026-11-07/EXPOSURE-AUDIT.md`. G3s owns a global SDR tone-scale
comparison now. Keep the incident meter and low-light key. Compare a scene-referred
toe and shoulder against the current valley and the dawn, noon and dusk references.
Select mid-grey placement with the curve and keep the output path extensible to EDR.
Use [Polyphony's GT7 reference](https://s3.amazonaws.com/gran-turismo.com/pdi_publications/s2025_PBS_Physically_Based_Tone_Mapping_GT7.pdf)
as a candidate. The audit's performance figures are estimates.

**SDR baseline decision.** Keep Neutral Highlights with the current incident
meter and low-light key. The scalar GT7 probe washed out sunset. The shadow-only
probe preserved metered grey and highlight colour but moved the twilight floor
median by only three codes. On a matched Metal valley at `1f4e6932`, the GT7
ICtCp SDR adaptation used the reference chroma blend, a matched grey point and
Rec.709 output compression. Pixels with all channels at 254 or above rose from
0.007% to 7.918% in the clear sunset-Sun view. Midnight ground median fell
25.35 → 14.35 codes. Independent review of the fixed views and sunset sequence
found the white plateau erased the disk and warm gradient and made the midnight
rock harder to see. Keep the existing display pass as G6's overlay baseline.
The flat floor still differs from the uncalibrated CC0 Kiara photos.

**Ground material.** A generated 256×256 local mineral-albedo map adds near
contrast through the existing cooked texture path. Geometry, camera and display
transform stay fixed. Metal noon foreground p90−p10 rose from 1.29 to 10.14
codes, and midnight median stayed 25.35. The sky crop matched across all 97
sunset samples. Independent review confirmed the existing lookout lip reads
more strongly, with no repeat or mip jump in sampled Metal and Chrome motion.
A repeating tile caused moiré and a full-valley warped atlas caused streaks.
Chrome initial upload grew 1,523,712 → 1,882,112 B and the live edit still
uploaded only the retinted product. In a 1080p Metal ABBA at shared load 37–39,
scene GPU p95 ranged 6.06–6.11 → 5.47–5.67 ms and submitted render CPU p95
3.67–5.83 → 4.27–4.45 ms. No speedup claim. Captures, benchmark and comparison
scripts are under `tmp/g3s-terrain-contrast/`. Live cadence remains open.

Raw inputs, hashes, comparison, paired stills and `sunset-pair.mp4` are under
`tmp/handback-review/tone-scale/colour-aware/`, with the earlier scalar and
shadow probes one level up. Run its `build.sh`, `capture.sh` and `compare.py`
from the repository root to reproduce. These are offscreen visual comparisons.

- **Night and EV floor remain open with G3s.** The reviewed
  [night milestones](../../design/modules/ATMOSPHERE-PLAN.md) pair the key and
  storage correction with moonless light and then stars. The current renderer
  remains Sun-only. Lunar illumination and mesopic colour remain open.
- **G3t/G4 own HDR.** G3t owns storage and the native boundary. G4 owns the shared display
  mapping and SDR/EDR comparison, including history scale and dynamic headroom.
- **G4 camera work owns occlusion-aware and local exposure.** Keep the open-sky incident
  mode for the valley. Compare a view- or position-dependent meter when there is a shaded
  consumer. Reassess local exposure on the backlit route after the global curve. Include
  the later cloud provider at the camera-light seam.

## Claude root session (6–7 November)

**Handback:** see [the Codex handback](CODEX-HANDBACK-2026-11-07.md) for everything landed, what is verified,
open items in priority order, the G6 boundary state and owner questions. Codex resumes as root.

The Claude session covered the Codex quota pause. Its oversight log is
`tmp/code-review-2026-11-05/overnight/WATCH-LOG.md`.

The finite-Sun penumbra from `codex/wip-g3s-solar-penumbra` landed on `main` with two
follow-ups. See [the penumbra evidence](G3S-SOLAR-PENUMBRA-2026-11-07.md).

- **Stipple cause.** A CPU ray probe over the source terrain reproduced Codex's
  0.893…0 visibility. It showed that the remaining stippled patches were false: a
  concave join 0.6 m upstream of a planar receiver reads as a blocker at grazing Sun.
  Depth within 0.9·tan(Sun elevation above the receiver) × the tap offset, capped at
  slope 0.25, now counts as the receiver's own surface. A new shipping GPU oracle
  fixture fails without it.
- **Taps.** Reduced to 8 search and 16 disk taps after truth scoring.
- **Near map unpadded.** The draft's finite-Sun cone guard grew the near map about
  threefold at low Sun and faded sphere contact. Only the last cascade is padded now.
  Blockers whose support leaves the near map are evaluated there, combined by minimum.
- **Cost.** Shared-GPU ABBA pass-sum p50 is 1.71 → 2.02 ms.
- **Terrain horizon.** Receivers beyond the cascades march a top-down caster height
  map toward the Sun (third shadow layer, 32 m texels over 32 km). Ray-truth error past
  500 m is at most 0.06 mean.
- **Floor-edge seam.** A Sun-facing slope at the floor's far edge rendered grey-blue
  because the back-facing normal repair reflected it across V's plane. It now lifts
  the normal just onto V's visible side.
- **Review.** Independent image-only review ranks the candidate above main and the
  draft. A second reviewer caught crease lines left by an interim fixed-slope
  tolerance. After the elevation bound, ray-truth false samples are zero on the
  scored solar frames.

The probe, scores and bench logs are in `tmp/g3s-showcase/solar-penumbra-claude/`. The final
capture is `tmp/g3s-showcase/capture-29-terrain-horizon/`.

## Goal and status

One reusable engine powers the same single-player V1 showcase on the M3 Max's built-in 120 Hz
display and Chrome. V1 needs physical sky, Sun and aerial perspective, cooked assets, live
camera and shadows, GPU culling, Apple reconstruction and EDR, display-link interpolation,
hot reload, profiler and controls. G2S separately proves corrected-run multiplayer equality.
Keep G3s first. G3b and G2S can progress on disjoint files. G5 and G6 belong to separate Claude
sessions. Their seam notes are `tmp/g5-boundary/NOTES.md` and `tmp/g6-boundary/NOTES.md`.
Transport, sea and clouds are later. The owner chooses when the port starts.

Correctness, lifecycle, visible quality and independent review close a gate. Record numeric
baselines and investigate visible hitches or roughly 25% regressions. No physical iPhone is
available. Simulator results prove function only, and any Mac stand-ins are labelled proxy.
Commit and fast-forward verified checkpoints to `main`. Keep the owner's pending `AGENTS.md`
and `docs/ASSET-LIFECYCLE.md` edits out of the commits. Owner digests are asynchronous.

## Verified and open

- The eight-node analytic atmosphere transports solar light through Sky, M, GroundE and aerial
  in-scatter. It projects at the RGB boundaries. The disk and receiver Sun use the projected
  T plane. The CPU meter uses the same coefficients. Aerial transmission of arbitrary RGB
  materials still uses a diagonal three-band approximation.
- `env RUSTC_WRAPPER= ./dev check` passed on the real Metal host. The production upper-twilight
  sample matches the compact-profile oracle within 2.5% per channel. The test checks all eight
  M lanes alongside the separate matched-band CPU/GPU reference. Chrome's `sky-reflections`
  case passed with Wasm SHA-256 `add91ac18cae0ad8a6ba885cfe4e778cea24726e0a854d219028395bc73275f8`.
- Native review saw blue upper twilight, warm horizon, and matching haze and reflections.
  Final 32 captures are byte-identical to the independently reviewed set. The [evidence page](G3S-FIRST-LIGHT-2026-11-06.md)
  carries a contact sheet, clip and timings. The sequence is offline and sparse in solar time.
  Chrome now also draws the cooked valley (G7). Live cadence and calibrated Earth radiance remain open.
- The foreground read flat and bright against Kiara dusk until the low-light key below. Some low-Sun contact marks and
  terrain joins remain. The original 17.9 h Sun-facing captures point into a ridge. Added clear views show the disk, and midnight
  is black under the current sole-Sun model. Isotropic multiple scattering and exposure are
  still approximations. Do not tune a colour gain to compensate for incorrect sky irradiance.
- G3b live ProMotion presentation and recovery are open. G2S still needs game-side Resolve
  staging and permit handling. The older quiet runs and tripwire baselines are linked from
  the build plan. There is no pending owner decision for the next atmosphere step.

## Pick up here

The capture route now adds clear Sun stills and 97 half-minute solar samples. Metal captures are at
`tmp/g3s-showcase/capture-18-solar-crossing/`. Full `./dev check` passed. Independent review
saw a stable roughly 9-pixel disk, then progressive ridge occlusion across frames 33–38,
disappearing in frame 39. Sky samples changed by at most 1–2 SDR codes per step. The [selected
crossing clip](g3s-first-light/solar-crossing.mp4) runs at 24 sampled frames/s.

**Next defects.** The 103-code foreground jump is fixed. Open, in order:
- The image review flagged a stair-step far ridge. In the 1280×720 west-facing sunset still,
  its edge tracks the source mesh within 0.48 px at p95. Doubling mesh density on both axes
  moves the projected edge by at most 0.74 px. Keep the mesh for this view and evaluate
  edge reconstruction in G4. Raw projection and measured rows are in `tmp/handback-review/ridge/`.
- In the same view the near-floor strip starts where depth jumps from 1.68 km to 59 m.
  The near pixels at the strip keep their RGB values with aerial colour bypassed. They
  cover a Sun-facing slope in the authored ground. Keep the aerial path. The paired sample
  and geometry probe are in `tmp/handback-review/ridge/`.
- The 100-code foreground step at solar 77→78 was a shadow leak from back-face-culled cooked
  casters with the Sun below horizontal. It is fixed ([evidence](G3S-SUNSET-CASTERS-2026-11-07.md)).
- Live cadence: `THRUM_CAPTURE_LIVE_TICKS` captures consecutive 60 Hz ticks. It exposed crest speckle at a
  0.6° Sun, now bounded by the height-map horizon below a 0.1 elevation sine
  ([evidence](G3S-LIVE-SUNSET-2026-11-07.md)). This is offscreen; on-screen ProMotion presentation is still G3b.
- Flat, bright twilight foreground: the incident meter now darkens below 1,000 lux on a
  Krawczyk key curve (about 2 stops at 10 lux, daylight unchanged)
  ([evidence](G3S-LOW-LIGHT-KEY-2026-11-07.md)). Earth radiance is still uncalibrated.
- Distant terrain shadows now come from a caster height-map horizon
  ([evidence](G3S-TERRAIN-HORIZON-2026-11-07.md)). Its +24% pass-sum is the next cost
  item; a per-frame horizon-angle map is the scalable form.
- Noon sky banding was 8-bit contouring; the display now dithers
  ([evidence](G3S-DISPLAY-DITHER-2026-11-07.md)).
- Low-Sun in-scatter over terrain hidden from the Sun is now shadowed
  ([evidence and shared-load measurements](G3S-SHADOWED-AERIAL-2026-11-07.md)).
- The penumbra's +18% pass-sum. Gathered fetches and a lit early-out are untried.

**G7.** Chrome now draws the cooked valley fetched from `thrum-cook watch --serve`
([evidence](G7-CHROME-VALLEY-2026-11-07.md); `check_v1_browser.mjs --case valley`). Its long poll swaps in edited
publications live, uploading only changed products. Device recovery with the valley attached
is a browser case (`--case valley-recovery`). The typed controls and parameter files are live.
G7 still owes the shared demo route and its startup, pacing and edit-hitch measurements.

Keep the same host Sun/time and the fixed valley camera as the comparison. Inspect
physical radiance and displayed colour independently. Use the checked CC0 Kiara photos
under `tmp/g3s-showcase/reference/` for look only. The provenance JSON is beside them.
The dense spectral probe is under `tmp/g3s-showcase/spectral-transport/`. It includes a
separate restricted reference cache. Never commit the cache or derived measured values.

Rerun from the repository root on the Mac with Metal access:

```sh
env RUSTC_WRAPPER= ./dev check
env RUSTC_WRAPPER= rustup run 1.99.1 cargo run --locked --offline -p thrum-cook -- cook examples/v1-game/assets/showcase tmp/g3s-showcase/cooked
env RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline -p v1-game --bin v1-game
env THRUM_ASSET_STORE=tmp/g3s-showcase/cooked THRUM_SCENE=valley THRUM_CAPTURE_SCENE=tmp/g3s-showcase/capture-fresh target/debug/v1-game
env RUSTC_WRAPPER= sh examples/v1-game/web/build.sh
node scripts/check_v1_browser.mjs --case sky-reflections --output tmp/g3s-showcase/chrome-fresh
```

Use new capture and Chrome directories per run. `tmp/g3s-showcase/retain-capture.py` verifies
byte-exact PNG recovery and builds contact sheets. The selected clip is 24 samples at 5 fps.
Do not present it as live motion. The ordinary sandbox may provide no Metal adapter. Use
the permitted device run. Use the pinned Cargo and clear `RUSTC_WRAPPER` if the host cache
wrapper fails. Cargo builds in worktrees sharing one target should run serially. Do not
overlap owned GPU measurements and tests.

Raw logs, build inputs, ABBA runner and per-run hashes are in `tmp/g3s-showcase/q8-production/`.
The benchmark is the existing `cargo bench --locked --offline -p v1-game --bench frame --no-run`
artifact run with `--g3b-threaded-batch`, the same CAS and `THRUM_SCENE=valley`. Build baseline
`7236b38` and candidate separately, retain each executable before replacing it, then alternate
A/B/B/A with uptime logged before and after each run. A completed `paired/runs.json` row has the
binary hash and exit status. Keep completed rows, use a fresh directory for any missing slot,
and append its observation instead of repeating finished work. The campaign completed before this checkpoint was pushed. All current capture, observer and check sessions are terminal. No run needs to be resumed. The old live Metal
System Trace scratch remains local. Review it before any raw drawable bridge.
