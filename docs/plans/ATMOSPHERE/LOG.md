# ATMOSPHERE log

- 10 November re-plan from [the module list](../ARCHITECTURE.md). M5 and M6 added and moved onto the loop's done-when
  file; M1–M4 keep their prose criteria above. M5 is commissioned now because sea and clouds read the sky through it.
- 10 November revision after [the design review](../../evidence/ATMOSPHERE/design-review.md) at `4bbe44a4` (changes
  requested). Finding 1: the design now carries a numerical acceptance table.
  [Full entry](../../evidence/ATMOSPHERE/log-entries.md#entry-1).
- 11 November revision after [the second design review](../../evidence/ATMOSPHERE/design-review.md) at `27473469`
  (changes requested, one Must Fix). Finding 1: the texel-range rule of H2 and H4 accepted a lookup that returns one
  depth slice. [Full entry](../../evidence/ATMOSPHERE/log-entries.md#entry-2).
- Status before this re-plan (8 November). Now: the stock sunset graph ships. M3’s no-flash crossing is a parked diagnosis spike assigned to a fresh-context Claude session. Blocked: the parked spike has no production candidate. Re-entry requires a diagnosis and review against the frozen crossing and pan, with the stock controls below.
- Reviews of M1–M4, to 8 November. Optical design and plan over `a91cdd0e`, final design `f9d0c12a` and plan `c3cc5748` · [optical brief](../../evidence/ATMOSPHERE/review-briefs.md#optical-review-brief) · initial changes required, final M1 implementation cleared. Frame graph and ownership at the same revisions · [graph brief](../../evidence/ATMOSPHERE/review-briefs.md#graph-review-brief) · initial changes required, final design cleared. First valley captures, Metal build `dfae5b21` and Chrome Wasm `bc15ea7d` (SHA-256 prefixes) · [capture brief](../../evidence/ATMOSPHERE/review-briefs.md#capture-review-brief) · held ground view passes, M1 remains open. Chrome late-twilight Wasm `134e271c` · [sweep brief](../../evidence/ATMOSPHERE/review-briefs.md#sweep-review-brief) · silhouette and reflection visible, warm contour and live motion open. Packed reflection candidate over `c2ac94d8`, Wasm `132f992f` · [reflection brief](../../evidence/ATMOSPHERE/review-briefs.md#reflection-review-brief) · visual improvement, rejected on measured Metal cost. Four-lane candidate over `4fecd095`, Metal `c6e7b102` and Chrome `4afa8b38` · [four-lane brief](../../evidence/ATMOSPHERE/review-briefs.md#four-lane-review-brief) · held visuals pass with faint residual streaks; no code findings; live cadence open. Live captures over `16dc9da2` · [follow-up brief](../../evidence/ATMOSPHERE/review-briefs.md#four-lane-review-brief) · late-twilight progression passes; early object shadow pop verified on both targets; physical cadence open. Shadow partition over `9f13c5f6`, Metal `510e62ce` and Chrome `c6500ef7` · [shadow brief](../../evidence/ATMOSPHERE/review-briefs.md#shadow-review-brief) · fixed three ownership and coverage gaps; matched visual checks pass. SDR comparison over `1f4e6932`, Metal `41d11c1d` and `1baf73d5` · [tone-scale brief](../../evidence/ATMOSPHERE/review-briefs.md#tone-scale-review-brief) · keep Neutral Highlights. Local ground material over `ec78a0d7`, Chrome `8d832924` · [ground brief](../../evidence/ATMOSPHERE/review-briefs.md#ground-material-review-brief) · selected, existing lookout lip remains conspicuous. M1 closeout source `b8f230e7`, Chrome `46d3eb04` and Metal `4928bca0` · [closeout brief](../../evidence/ATMOSPHERE/review-briefs.md#m1-closeout-review-brief) · sampled crossing and exposure pass on both targets, including the completed foreground Metal sweep through midnight. The callback-tail tripwire stays open. M2 look criterion over `590cee61` · [midnight look brief](../../evidence/ATMOSPHERE/review-briefs.md#midnight-look-review-brief) · added an unlabelled shipping-frame witness and a same-build star toggle. M2 integration over `6fa34365`, corrected WIP `1fa3228` · [star integration brief](../../evidence/ATMOSPHERE/review-briefs.md#star-integration-review-brief) · camera, opt-in and altitude findings fixed on recheck. Tight footprint over `a61bb80`, Chrome `568edbaa` · [tight footprint brief](../../evidence/ATMOSPHERE/review-briefs.md#tight-footprint-review-brief) · compact held stars and sampled tracking pass. Required a positive central night witness. Live faint-star stability remains open. Harvard source pack `3a3536c7` and opt-in valley `98e0be1`, Metal `a7588eed` and Chrome `ced775e0` · [source and valley brief](../../evidence/ATMOSPHERE/review-briefs.md#catalogue-and-valley-review-brief) · original values reproduced, sampled midnight, occlusion, pan and emergence pass. Late sky rebound and live cadence remained open. Twilight shoulder WIP `6a2dbf6`, Metal `c4700ab7` and Chrome `80cd702c` · [shoulder brief](../../evidence/ATMOSPHERE/review-briefs.md#twilight-shoulder-review-brief) · sampled look passes for the checkpoint, with a small residual rise in one sky patch. The inline twilight cost overlaps the baseline. Daylight selector WIP `3407173`, Metal `564cccbc` and Chrome `4109e560` · [daylight brief](../../evidence/ATMOSPHERE/review-briefs.md#daylight-selector-review-brief) · bound and held image comparisons pass; live motion and production integration remained open. Main integration `00c2645`, Metal `0acd58ff` and Chrome `fcc21034` · [integration brief](../../evidence/ATMOSPHERE/review-briefs.md#main-star-graph-integration-review-brief) · cross-graph ownership passes and default twilight changes are isolated on the integration branch. Continuous Chrome build `fcc21034` · [motion brief](../../evidence/ATMOSPHERE/review-briefs.md#star-motion-integration-review-brief) · tracked stars persist through the pan, with measurable core brightness variation. Native window motion remains open. Normal valley `197d152`, Metal `1384a38a` and Chrome `b870c1ee` · [normal launch brief](../../evidence/ATMOSPHERE/review-briefs.md#normal-valley-launch-review-brief) · same-build controls and the held composition with G7 ground E pass, with the expected old midnight baseline drift recorded. Production star optics over `197d152`, revised fixture SHA-256 prefix `aacba07b` · [optics brief](../../evidence/ATMOSPHERE/review-briefs.md#production-star-optics-review-brief) · centroid and two-axis depth fixes cleared on recheck. The remaining one-code contrast allowance was raised to eight. The Chrome slider witness supplies game continuity separately. Reconciled star graph `d2a5342` · [merge review](../../evidence/ATMOSPHERE/review-briefs.md#reconciled-star-graph-review-brief) · no merge-induced findings. The final Metal test exercises timed Frustum and Occlusion capture. Canonical valley Metal `c5c5b84a` and Chrome `b497346f` · [held route review](../../evidence/ATMOSPHERE/review-briefs.md#canonical-star-route-review-brief) · clear night and same-build controls pass. Midnight refreshed at the unchanged route tolerance. Metal emergence build `7ecc56cc` · [offscreen brief](#offscreen-emergence-review-brief) · timing and encoded frame pairing pass. Held and decoded frames show emergence. Continuous playback remains open. Ground E pan build `313d423c` and held route `check-20261107-230802` · [pan brief](#ground-e-pan-review-brief) · decoded pan and terrain masking pass. Midnight-only baseline approved by review. Sunset prototype build `bc237e22` · [sunset brief](#sunset-horizon-review-brief) · foreground depth edge and thin stripe improve, shadow pop still visible. Metal builds `7ecc56cc` and `313d423c` · [all-frame decode brief](#independent-all-frame-decode-brief) · held night and decoded emergence and pan pass, with a one-code late sky lift. Playback remains open. The authored spur on clean `ba45ff60` · [paired route brief](#authored-spur-review-brief) · held route and controls pass; source discontinuity, flecks and crossing band hold promotion. The vertex-baked far-horizon Metal proof over `ba45ff60` · [far-horizon brief](#far-horizon-review-brief) · fixed-camera band removed; caster union and baked eligibility hold promotion. M2 review on Metal builds `313d423c` and `7ecc56cc`, Chrome Wasm `e64183ed` and timing revision `59e76f3d` · [M2 closeout brief](#m2-closeout-review-brief) · D1–D3 pass, with timing and bandwidth outstanding at that revision. D4 over `172ce6fe`, Chrome Wasm `429ade00` and Metal pan build `313d423c` · [timing brief](#m2-timing-review-brief) · scope and evidence pass with wording changes. Scene-only Metal rows and physical counter follow-up are distinct from full-frame timestamps. M3 first design draft over `007e8f88` · [coverage brief](#m3-coverage-design-review-brief) · hold runtime. Aperture union sound; closure, precision, capacity and publication need explicit proof. M3 edge support follow-up over `79b726ec` · [edge brief](#m3-edge-support-review-brief) · real-arithmetic pruning passes. Closure, portable precision and capacity remain open. M3 receiver-face proposal over `290d53da` · [face brief](#m3-receiver-face-review-brief) · dominance proof passes with fixed world sectors and an enclosed in-footprint origin. Packed capacity and caster work remain open. M3 directional reuse over `c7937efb` · [sweep brief](#m3-directional-reuse-brief) · advance source-side hull sharing to offline capacity proof. Fixed-line samples do not certify a sector. M3 coverage split over `4461bff6` and the block-join/caster census · [split brief](#m3-terrain-coverage-split-brief) · residual composition is sound. Decoded coverage, capacity and complete caster admission need proof. M3 fit witness over `d063f2e8` · [fit brief](#m3-decoded-fit-brief) · broad holdout passes. Refine the raised crest and independently bound the terrain cut used by the residual. M3 fit stop over `9e811aa2` · [roof brief](#m3-terrain-roof-brief) · select nested terrain bounds and original geometry for an offline cost screen. Hold runtime. Ray reference over `9e811aa2` · [ray brief](#m3-ray-reference-brief) · geometry and Metal tolerance accepted. Added coincident and scaled controls for union and intersection precision. Projected envelope over the same checkpoint · [projection brief](#m3-projected-envelope-brief) · proceed to one paired CPU proof, with near clipping and above-cap terrain coverage kept. Disk clipping over the raised-crest projection screen · [disk reference brief](#m3-disk-reference-brief) · CPU reference candidate accepted with finite window and apex conditions. Runtime capacity remains open. M2 requalification at `9c1021bb`, Metal `56948ba5`, Chrome `f50141b4` · [orientation brief](#m2-orientation-review-brief) · D1–D3 pass at the captured route’s scope. Physical cadence remains separate. Bounded disk coverage over `846c4fba`, initial diff `4ce8cc61` and revised dyadic mechanism · [review brief](../../evidence/ATMOSPHERE/review-briefs.md#bounded-disk-coverage-review-brief) · numerical and work bounds required, revised offline evaluator cleared, GPU integration held. Current-Sun strip sweep over `360f93c1`, initial diff `90481778` and revised endpoint/transition proof · [review brief](../../evidence/ATMOSPHERE/review-briefs.md#current-sun-sweep-review-brief) · one bounded offline proof cleared; raw-band rejection independently checked, disk-aware representation remains open. Frozen source-edge packet draft `bfbbdd36` (SHA-256 prefix) · [packet brief](../../evidence/ATMOSPHERE/review-briefs.md#frozen-source-edge-packet-review-brief) · revised finite screen cleared. Continuous omission bound over `524eb86f` · [packet brief](../../evidence/ATMOSPHERE/review-briefs.md#frozen-source-edge-packet-review-brief) · independent-q refinement inconclusive at the distant crest; fixed near partition and same-ray screen over `86750ebd` · [packet brief](../../evidence/ATMOSPHERE/review-briefs.md#frozen-source-edge-packet-review-brief) · bounded residual inconclusive at all eight domains; stop packet trials. M3 rescope and first SMRT draft over `9d1cabbb` · [production brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-production-shadow-review-brief) · scope split approved. Reject the binary ray screen on quantization and hold shader work for a defined fractional resolve, field support and composed cost. M3 contour-area design over `000ffcdb`, drafts `073e32d9` and `ece6c04a` (SHA-256 prefixes) · [contour brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-contour-area-review-brief) · literal query fails the expansion cap. Merged-interval preflight cleared and passes the saved-field screen. Shader admission held. Earlier crossing archive over `50d33835`, screen `ce431c37` and log `543b1893` (SHA-256 prefixes) · [trajectory brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-early-crossing-review-brief) · frozen check fails on stock's zero-motion plateau. Preserve the failure and use a separately fixed extinction and handoff witness. Production PCSS over `268f8a93`, draft SHA-256 prefix `8bce8a15` · [review brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-production-pcss-review-brief) · conditionally clear one game screen once the matched arms and full-frame motion witnesses below are frozen. No promotion. Matched PCSS screen over `efa82534` · [review brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-production-pcss-review-brief) · reject rotated-32 on native-size Metal and Chrome crossing. The original coherent step is smaller and new crosshatching is visible. Current-frame resolve over `8a4cdfbd`, draft SHA-256 prefix `6d559395` · [resolve brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-current-frame-visibility-review-brief) · independent and Apple tile reviews clear raw receiver diagnostics only. Require exact tied-winner, compact-shadow and variant traffic proofs before integration. Cooked seam review pending. Raw visibility diagnostic over `5984ca51` · [resolve brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-current-frame-visibility-review-brief) · grain reduction confirmed on the fixed Metal record. Compact caster and independent opaque-winner control remain open. Hold integration. Caster atlas trace over `bded4d69` · [resolve brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-current-frame-visibility-review-brief) · independent projection matches both blocking depths to the original terrain. Retain the upstream miss and hold the live resolve. Caster scope and stock control over `b4ec6eac` · [control brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-resolvable-caster-review-brief) · keep D1 and the historical overlap. Approve one separately authored stock qualification at 160 m, with an observed wide-filter edge and clean controls. Offline 3×3 preservation over `d5768222` · [preservation brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-offline-preservation-review-brief) · held 160 m profile passes. Separate opaque-winner proof required before live integration. Isolated Metal winner over `25ebe50c`, marker fixture `6a05778a` (SHA-256 prefix) · [winner brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-opaque-winner-review-brief) · raw depth and tags pass in both orders and both camera entries. Production Uint32 multidraw repeat is byte-identical. Live graph and browser remain open. Packed WGSL resolve over `81f55ef2`, kernel `980aaa95` (SHA-256 prefix) · [GPU resolve brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-packed-gpu-resolve-review-brief) · saved Metal and Chrome masks agree with the CPU oracle in the three fixed regions after correcting displacement precision. Integrated forward depth guard and graph remain open. First live receiver diff over `293c0524`, WIP `d7738029` · [live brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-live-receiver-review-brief) · graph cleared for a smoke run, G5 clears shared draw selection and whole-view hook fallback. Native startup passes. Integrated guard, motion and tripwire investigation stay open. Live depth and timing over `d7738029` · [guard brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-live-depth-guard-review-brief) · stored depth matches, fragment Z rounds one ULP high at rejected Metal samples. Clear a bounded guard proof. Live cooked winner, Chrome depth and shaded fallback stay open. Live tie and shaded fallback over `77e84128` · [integrated brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-live-tie-and-fallback-review-brief) · both backends agree on 2,916 true ties and ±2 ULP stock pixels. Tied guides are ineligible. Require one filterable tie and keep the frozen motion open. Frozen game motion over `77e84128`, Metal `24fb37a5`, Chrome `0799f9d8` and `dbd1562f` · [motion brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-frozen-resolve-motion-review-brief) · D1 fails on introduced ribbing. The 235 live tie controls confirm guard acceptance on both backends, with zero raw visibility at the tied centres. M3 scope over `c0ae64dc` and its archived motion · [scope brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-scope-review-brief) · keep D1–D3 together. The shipped valley lip and the authored spur flash are distinct observations. M3 parking and successor over `282cc039` · [handoff brief](../../evidence/ATMOSPHERE/review-briefs.md#m3-parking-review-brief) · approved after clarifying the seam channel and separating the authored flash from the shipped terrain lip.

- 8 November, 13:05 CET steer. Exact matching consumed roughly five hours with no M3 done-when closed. Move that admission and its witnesses to M4. Keep M3's visible no-flash crossing, transition controls and frame integrity. The exact union is an offline referee for M3. No later V1 consumer is known to require it: aerial visibility has its own interval contract, and tile, reconstruction and the showcase consume the resolved image. Independent review agreed. It rejected the first fixed binary-ray adaptation before a run and held shader integration.
- 8 November contour follow-up. The literal four-contour cap rejects all eight rows. The independently rechecked preflight resolves uniform tiles first, with every attempt charged to the unchanged cap. All eight pass the D32 screen. This establishes fractional support on the stored field only.
- 8 November extinction check. The matched 21-frame trajectory rejects the whole-receiver fallback. Stop after three failed screens and retain the fractional-area results. The visual cue is foundational for M3. The next design must localize missing support rather than raise the contour cap.
- 8 November, 13:42 CET steer. Review shipped shadow filtering and stable cascade handoffs before designing a new coverage model. Select a fixed-phase-per-pixel directional PCSS adaptation for independent review. The existing transitions remain witnesses. The independent review confirmed the in-map sampling defect and required a matched fixed-phase arm and complete motion capture. No renderer change or new screen ran.

- The 8 November daily review found an orthogonal reflection in the game basis. Reopen M2 D1 and D3. Choose a right-handed local frame at −45° to keep the authored Sun path while correcting star orientation. M3 design proceeds alongside the recheck.

- M3 remains foundational. The uniform, adaptive-time and conforming fits miss the raised crest. The conforming screen keeps its seams but exhausts the next refinement within its diagnostic ceiling. Park pure fit trials and review an original-geometry complement. Raw rows and the method decision are in `tmp/g3s-sunset-lip/M3-RESEARCH.md`.

- 2026-11-07 — started the moonless-night milestones after the cubic LUT probe produced no appreciable SDR improvement.
- 2026-11-07 — the owner accepted M1 and added the midnight look check to M2 after inspecting the foreground sweep.
- 2026-11-08 — parked the sunset-shadow trials after repeated failures. They do not block the star milestone. M3 owns the separate shadow outcome and retains the witness and review evidence. Apply the revised engine-module-design stop-and-decide rule before restarting.
- 2026-11-08 — M3 treats coverage composition as foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the published alternatives, the archive population check and the shared-aperture choice. A clear/full-only fast path cannot cover the difficult partial frame.
- 2026-11-08 — one-aperture Ground E sizing rejects the flat layout. Use pairwise slope bounds on the receiver face, with sparse angular refinement. The research note records why this is a different certificate and what still needs proof.
- 2026-11-08 — the independent review cleared one bounded disk-cell screen. It exhausted the read cap on every archived receiver. Park fragment-local integration and keep the crossing foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the design, review and raw rows.
- 2026-11-08 — after the independent sunset brief, select a shared Sun-aligned strip sweep for review. The research note gives the band bounds, cost ceiling and fixed witness. SMRT, SDF cone minima and temporal history do not supply the required current-frame coverage.
- 2026-11-08 — park the raw-extrema sweep after the fixed entrance proof. The crossing stays foundational. `tmp/g3s-sunset-lip/M3-RESEARCH.md` records the required within-band support and the independent verdict.

### Offscreen emergence review brief

> Independently inspect the game capture's frame submission, clock and
> readback in `bin/window/{capture,motion}.rs`. Check every video frame
> against `offscreen-emergence/capture/frames.jsonl` for a tick and
> valid GPU time. Judge the fixed valley view during star emergence
> against D1 and D2. Distinguish decoded frames, continuous playback
> and camera motion, and report visible defects without pacing claims.

### Ground E pan review brief

> Compare the twelve held frames from `check-20261107-230802` with
> ground E's route baseline at `8f1dcf76`. Review both 600-frame
> lossless clips in `ground-e-night-pan{,-off}` for clear-night read,
> bright and faint star stability, and terrain masking. Verify paired
> ticks, pose and timings. Distinguish sampled and decoded review
> from playback. Check the pan uses the game's camera control.

### Sunset horizon review brief

> Inspect four matched 1080p60 clips in `tmp/g3s-star-catalog/current-main/`
> for current and prototype shadows on Ground E and the connected spur.
> Separate the terrain and shadow effects. Identify visible discontinuities
> and give frame indices and timing provenance.

### Independent all-frame decode brief

> Inspect the raw Metal emergence and matched midnight pans from the
> offscreen and Ground E briefs. Judge D1/D2 at native size, including
> temporal coherence, skyline masking and the clear-night read.
> Check video timestamps and submitted GPU samples. Separate actual
> playback, decoded motion evidence and held frames. Read only.

### Authored spur review brief

> Compare both six-stop `ba45ff60` route runs and the 300-frame Metal
> pair under `tmp/g3s-sunset-lip/`. Judge the west depth edge, the
> moving solar shadow, the ridge and other cameras at native scale.
> Inspect the two sources for geometric discontinuities and check
> the stock route and control results. Distinguish decoded samples
> and measured frame changes from physical presentation. Read only.

### Far-horizon review brief

> Review the baked far-horizon game comparison over `ba45ff60`,
> patch, CPU rows and interleaved timings under `tmp/g3s-sunset-lip/`.
> Judge the visible edge and the scope of the bypass. Check near and
> far composition, mesh identity, azimuth wrap and measured pass identity.
> Read only. Distinguish decoded samples from playback.

### M2 closeout review brief

> Independently judge D1–D4 on the shipped Ground E valley. Inspect the
> Metal pan, stars-off control and emergence records, the current Chrome
> motion and six-stop route, catalogue provenance, optical fixtures, and
> raw on/off timing rows. Separate visual samples from playback and
> distinguish logical attachment area from measured traffic. Read only.

### M2 orientation review brief

> Requalify D1–D3 independently on `9c1021bb`. Inspect the
> exact-build Metal and Chrome clips, independent named-star
> pixels, matched midnight and slider-wrap controls, and the
> production Metal optics log. Judge the actual route and
> distinguish captured motion from physical presentation.

### M2 timing review brief

> Audit the M2 D4 evidence independently, read-only. Check eight paired
> Chrome arms on Ground E, exact browser build and preset, both paired
> Metal sources, render CPU and full-frame timing scope, and the visible
> performance tripwire. Check attachment formats and graph operations
> against the logical byte totals. Reproduce the packed catalogue from
> the audited rows and verify its checksums. Inspect the saved trace
> inputs and distinguish deferred physical counters from graph-declared
> bytes. Judge what the numbers support without an attributed speedup
> or physical display claim. No new build or GPU run.

### M3 disk reference brief

> Check the finite Sun disk projection and Jacobian bound against
> the 289 raised-crest receivers and paired logs in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge geometric coverage,
> apex and clipping semantics, event capacity and continuity.
> Rule separately on the CPU reference and runtime admission. Read only.

### M3 coverage design review brief

> Read-only independent review over `007e8f88` and the first M3 draft.
> Derive the required outcome from M3 and challenge the spherical
> union, terrain closure, thin caster, work bounds, Chrome limits and
> edit or exhaustion lifecycle. Read the archive population and prior
> failure witnesses in `tmp/g3s-sunset-lip/M3-RESEARCH.md` and
> `tmp/g3s-sunset-lip/NOTES.md`. Evaluate the next design decision and
> any established cheaper method. No new GPU trial.

### M3 edge support review brief

> Check the prepared original-edge atlas against the upper-envelope
> proof and the raw 1080p archive rows. Challenge whole-triangle
> partitioning, grazing and collinear hits, the terrain prefix and
> portable floating-point accuracy. State what the CPU witnesses prove
> and which work remains before a runtime change. No GPU or build.

### M3 receiver face review brief

> Review the design diff over `290d53da` against the full-source CPU census in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge the receiver-face Bernstein
> certificate, origin and terrain closure, shared-list capacity and independent
> caster admission. Identify the smallest offline proof before runtime work.
> No GPU run or build.

### M3 directional reuse brief

> Review `c7937efb` and the raw face-sector census in
> `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Find an established mechanism to
> share terrain work across continuous receivers. Account for Sun aperture
> bounds and compact-caster union. Return one recommendation and a
> falsifier before runtime work. Read only.

### M3 terrain coverage split brief

> Review the block-join and caster-cone census over `4461bff6`. Challenge
> prepared terrain-only coverage with common-aperture correction on
> detached overlap, continuity in space and time, edit ownership and
> caster completeness. Keep D1–D3 unchanged. Read only.

### M3 decoded fit brief

> Review the uniform and adaptive-time fit logs and detached residual
> over `d063f2e8` in `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge what
> the holdouts and detached-ray check establish and whether the spatial
> mechanism warrants a continuous proof. Read only.

### M3 terrain roof brief

> Review the conforming fit and independent union over `9e811aa2`.
> Compare an exact collar to nested terrain solids and original
> residuals. Recommend one work/ownership contract and a falsifier.
> Preserve D1–D3. No fit trials, builds or GPU work.

### M3 ray reference brief

> Check the 3D ray reference and its use in the unchanged Metal
> plateau fixture over `9e811aa2`. Challenge weighting, overlap
> controls and recorded per-column roof counts. Read only.

### M3 projected envelope brief

> Review the projected edge-envelope proposal and the prior shared
> beam in `tmp/g3s-sunset-lip/M3-RESEARCH.md`. Challenge support
> clipping, terrain fill, the cap and the paired work falsifier.
> Keep runtime integration held. Read only.

- 8 November, 14:12 CET. The matched 32-tap rotation cuts the coherent step but introduces visible crosshatching in the west-spur penumbra on Metal and Chrome. This is the first failure on the reviewed production PCSS comparison. Keep the shipped filter, matched drivers and all witnesses. The visible cue remains foundational. No seed or radius tuning. Review a resolve before another screen.

- 8 November, 14:18 CET. Select a current-frame shadow-mask resolve as a diagnostic. The independent review confirms the extra raster and packed guide design can be evaluated without changing the opaque winner. It flags compact-caster smear and coincident opaque ties. Apple tile review keeps three stored transients confined to the optional variant. No shared graph change or new game screen is cleared.

- 8 November, 14:28 CET. The frozen receiver check reduces grain without moving the strongest strip-average edge, but several raw zeros become positive. That is unresolved recovery versus false light. Both authored caster arms match the no-caster D32 receiver and guide bytes, so they cannot clear compact-shadow preservation. Hold integration and keep the next check on the existing positive witness.

- 8 November, 14:38 CET. The wide caster covers two last-map centres but both already hold closer depths. All three fitted layers are byte-identical with and without the caster. This explains the silent witness without a new candidate screen. The no-attachment quad estimate leaves substantially more regular grain than the 3×3 resolve and stays parked. D1 and D2 stay open.

- 8 November, 14:38 CET steer. Measure the exact compact fixture against the cascade that can receive it. It spans 0.20 × 0.47 far texels and is clipped by the near map. Keep it in M4. The reviewed stock-only 160 m replacement control shows a positive wide-filter penumbra. It adds a preservation check and leaves every D1 game view and the earlier overlap witness in place.

- 8 November, 16:40 CET ruling. The overseer declined the foundational call after repeated design and review cycles failed the frozen transition. Park all of M3 as a later diagnosis spike, with D1–D3 open, and transfer shadow and resolve work to the fresh-context Claude session. G3s remains open on the build plan.

- 8 November, reference-first steer. Propose treating the continuous D1 penumbra as foundational and keep the cue parked pending the overseer or owner ruling. Contour fallback, rotated taps and the current-frame resolve all failed on the same visible transition. In the independent saved-field check, freezing each receiver’s radius across 226–227 still gives stock zero at 227 while dense D32 integration finds 2.47–3.63% light. Review the estimator choice and its cost before any new runtime change. Keep the original D1 views, the 160 m control and the distinct M4 reference question. The research note is in `tmp/g3s-sunset-lip/M3-RESEARCH.md`.

- 8 November. The live 3×3 resolve reduces the fixed red-code step but creates regular ribbing and leaves a warm band later in the extinction. Park this graph. All 28,840 pixels in the audited Metal crop accept the mask on each adjacent witness, so intermittent forward guard acceptance cannot explain the ribbing there. The showcase crossing is foundational for D1. Revisit the underlying visibility mechanism before another screen.

- 8 November. The held 160 m comparison clears the fixed offline resolve. The independent reviewer confirmed matched inputs, preserved edge and interior values, and a small lift at one raw-zero row sample. Require a separate winner-ID check before a new game screen.

- 11 November plan reshaped for orchestrate 1.7.0. Status lines removed. The M1–M4 sections moved to
  [records](../../evidence/ATMOSPHERE/M1-M4-records.md) and the review briefs to
  [review briefs](../../evidence/ATMOSPHERE/review-briefs.md). M3's and M4's open criteria stay in the plan. The
  open assumptions moved from this log to the plan's `Open for the owner`. No item or scope changed.
- 11 November plan change for M5's blocker: H2 missed its filtered-decode bound on Metal. The planner's decision:
  the lookups stay one hardware-filtered read, and the bound of H2 and H4 gains one stored half-float step `Q`.
  M5.2's question names it. The design needs a new review before M5 resumes.
  [Full entry](../../evidence/ATMOSPHERE/log-entries.md#entry-3).


## 11 November 2026 — M5 H2 admission blocker

Added the environment identity and on-demand lent products, optional Sun-path optical-depth variant,
per-march cooked-volume factor and opt-in diagnostic provider. CPU tests and the Metal handoff smoke
pass. H1 low-altitude and H4 filtered arms pass. H2 fails its unchanged allowance on the live Metal
chart. A controlled 257-position measurement isolates low-contrast fp16 filtering precision.
[Evidence and reproduction](../../evidence/ATMOSPHERE/m5-h2-blocker.md) record the measurements,
partial proof scope and remaining work. The design requires a planner decision before numerical
admission continues. M5 stays open; integration is pending.


## 11 November 2026 — M5 independent cloud-factor qualification

The production cloud variant passes seven Metal cases and 56 receiver-point queries for zero optical
depth, uniform ln2, spatial variation, height-dependent Sun-ray projection, displacement and clear
validity limits. This qualifies the shared lookup; receiver-kind and cooked-volume tests remain pending.
The packed surface light and lent identity now share one prepared CPU transmission result.
The identity tests pass, and the repeated handoff tests retain the H2 miss with four other tests passing.
Origin/main at `be9ea93c` retains the design instruction requiring this miss to return to planning.
[Selected evidence](../../evidence/ATMOSPHERE/m5-h2-blocker.md) includes the cloud-factor log.


## 11 November 2026 — M5 target qualification checkpoint

Cherry-picked the previous M5 work and merged the dependency conflicts. Metal and hardware Chrome
pass H1–H5 under the revised H2/H4 allowance, with independent references and wrong-lookup controls.
The full milestone checker passes all nine commands, including shared surface/cooked-march cloud
cases, base/current/rerun parity and order-alternated costs. Captures are complete and packaging
continues. M5.7 and M5.8 await the required 1× review because Computer Use's browser host is
unavailable and IINA access was rejected. M5 stays open. Newly integrated APPLE-PRESENTATION
changes to shared shaders require reconciliation and fresh affected proofs before the claim.
[Target evidence](../../evidence/ATMOSPHERE/m5-claim.md).


## 11 November 2026 — M5 rebased proofs and resumed motion review

All nine milestone check commands pass on the reconciled source, including Metal and hardware Chrome
handoff rows, byte-identical no-provider controls and paired cost observations. The recorded parity
base includes the concurrently integrated motion inputs, with the initial base preserved. The resumed
session can access IINA. Three native and four Chrome source packages are hash-verified while the
remaining captures and 1× review queue for the shared GPU. M5.7 and M5.8 remain open.
[Current evidence](../../evidence/ATMOSPHERE/m5-claim.md).
