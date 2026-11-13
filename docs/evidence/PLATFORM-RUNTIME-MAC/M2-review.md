# PLATFORM-RUNTIME-MAC M2 independent review

**PASS — all five items.** Reviewed 13 November 2026, builder checkpoint `a024524b6bc2`, from the supplied checkout `e119993ac1c0`. No Must Fix findings or additional Suggestions.

The builder has replaced the old-binary proof with a matched release assembly: executable `688f378f96d33c61f5f21b70582610ff9673c63e7f3c5af6b5eb859e6b4f8422`, source `7a791b20`. I compared every entry in its retained build-source map with `a024524b`: **zero differences**. The supplied checkout also includes subsequent changes in other modules, beyond the plan migration. I rebuilt and exercised that checkout as requested; its executable is `4e62fe113996617b1eae5edf1bc6ff2546ac969b7b33c8eac786fb980f7ca135`. Its live checks corroborate function and motion, but do not relabel the builder's trace as cadence of a different executable. Mac M3's requalification obligation remains.

## Item verdicts

| Item | Verdict | Independent result |
| --- | --- | --- |
| M2.1 | pass | Fresh denied-permission realtime run, complete holds/sweep, native images and host-time sequence inspected. Builder originals independently decoded. |
| M2.2 | pass | Builder's three arms match its reviewed assembly; complete ON motion and OFF presented witness inspected, misses investigated. Fresh untraced runtime/window checks corroborate. |
| M2.3 | pass | Fresh active-window pattern, pending-readback replacement, valley continuation and OFF runs; source correspondence/lifetime review and supplied full-check result. |
| M2.4 | pass | Both historical inputs rerun, all known misses reproduced, truncated coverage rejected; implementation and refusal tests inspected. |
| M2.5 | pass | Fresh builder OFF/ON exports independently recomputed: complete markers, all marked submissions joined, validated conversion, single built-in display, recording budget accounted for. |

### M2.1 — elapsed engine motion

I [built release](M2-review/fresh-build.log) `v1-game` and `thrum-cook`, cooked the checked-in showcase and ran `capture_mac_m2_ungranted.py` under the repository GPU lock. The command initially queued behind active tests, then completed normally. Launcher and driver before/after preflights were all denied under the recorded fresh LaunchServices identity. Console observations were unlocked before/after and were not admission conditions. No grant was requested.

The fresh run has **481** verified originals at 1920×1080, realtime monotonic opportunities requested at 120 Hz, tick 0 throughout, zero ring/writer omissions, zero skipped requested opportunities and no exact repeated images inside the sweep. Initial hold is **1.003008166 s**, first final-yaw image **3.003660375 s**, final hold **1.007266416 s**. Maximum image gap is **12.102625 ms**. Movie PTS error is at most 0.5 microseconds; that is a movie/source alignment observation, not display cadence. Requested, accepted and submitted yaw remain joined by submission ID in the manifest/runtime records.

The builder's matched offscreen run separately has 480 originals, holds 1.003472/1.008173 s, no omissions or sweep repeats, and one skipped opportunity/22.935667 ms image gap during the stationary initial hold. I independently decoded and checked every original against its recorded hash, rather than trusting selected PNGs. Historical `0d53d3aa` motion and its 38.3/51.7 ms holds remain historical only.

[Fresh analysis](M2-review/fresh-motion-analysis.json), [manifest](M2-review/motion/session/manifest.json), [launcher](M2-review/motion/launcher.json), [host-time movie](M2-review/motion/session/preview-1x.mp4), [10 fps inspection](M2-review/motion/session/review-10fps.png), [native crop](M2-review/motion/session/crop-2.png), [retained-original verification](M2-review/originals-verified.json).

### M2.3 — real window and tap

The fresh `--window-only` run completed on the unlocked, awake built-in display under `gpu`, without consuming a trace recording. Pattern and valley replacement arms produced **823/830 real presentations** and **820/827 verified captured originals**. Both retire exactly IDs 357–359 with **three readbacks pending**, change surface lease generation 2→3 and presenter generation 1→2, refuse stale CPU-image recycling (outstanding count remains 1→1), and resume healthy presentation. The pre/post replacement boundary images are byte-identical in each arm. The explicit retirement gaps are 48.925/64.662375 ms; these diagnostic replacement sequences are not claimed continuous.

All pattern originals match the independent RGBA channel/row oracle SHA-256 `8eabe66441bcbce1d3a92072454250853ce6ad99a2c1eec46d682f677e5fff15`. The shader writes exact code values; the normal output/present binding performs the declared RGBA-to-BGRA surface channel mapping. The proof concerns engine output consumed by presentation, not equality with colour-managed compositor pixels. The separately granted screenshot observer captured an intact real window before and after; no new permission was requested.

The OFF probe has **836 presentations**, no images or spool, and `tap_enabled=false` throughout. In source, `Observer::start` does not enable capture or create a writer for OFF. The normal graph variant omits capture copies; capture reservation returns none. The actor encodes, acquires one drawable late, submits and consumes it in present before recording the observation. The tap retains its own staging/CPU resources, not a drawable, and introduces neither another acquire nor an extended drawable lease. Surface replacement retires the capture session; session/owner/device checks reject stale recycling. Existing device-loss checks and the no-old-queue abandonment path remain intact.

The supplied orchestrate result establishes `./dev check`, including M2.3's identical check, passed at `a024524b`. I inspected its registration and relevant real-Metal correspondence, disabled-path and stale-generation tests and found no reason to repeat that full suite. Tests alone were not used as a substitute for the fresh live-window run.

[Fresh window analysis](M2-review/fresh-window-analysis.json), [run log](M2-review/live-window-run.log), [pattern after replacement](M2-review/live-window/grid-replacement/window-6s.png), [valley before](M2-review/live-window/valley-replacement/window-2s.png), [valley after](M2-review/live-window/valley-replacement/window-6s.png).

### M2.4 — witness command

I read `scripts/mac_m2_presented.py`, its collector/calibration inputs and tests, then ran both named historical TSV inputs. OFF reproduces **526→527 and 544→545**, ON **558→559**. OFF's second interval joins the game's **15.470792 ms** present-call gap. Both commands correctly exit 2 because retained coverage starts after Whole begin.

XML parsing selects `ca-client-presented-handler` rows for the recorded game PID. Presented time comes from converted `at-time`, never handler timestamp or delta. Four same-process signpost brackets independently constrain the clock offset. Missing ordinals, zero/absent times, unfamiliar status and duplicates are unmatched. The 14 focused tests pass, including changed callback/delta values, PID attribution, bad conversion, coverage refusal, missing image/control joins and permission attribution. A fake callback-time witness would fail those tests.

[Historical OFF](M2-review/known-off.json), [historical ON](M2-review/known-on.json).

### M2.5 — complete fresh witness

I recomputed the builder's 13 November raw compressed XML exports, inspected the TOCs, marker brackets, window/display identities and commission. Both recordings are **Deferred**, with a 16-second limit and bounded finalization/exports. OFF retains 841 joined rows and ON 845; each joins **all 488 marked submissions**, with no unmatched rows affecting Whole. Retained handler and converted presented spans cover both Whole markers. The four marker brackets have an intersected offset width of 583 ns and stated converted presented-time error bound **7.996 ns**; converted values are checked between the game opportunity and OS handler.

Each display table lists only **Built-In Display 1**, main, 3456×2234, maximum 120 Hz; the game window independently records 1920×1080 BGRA output and 120000 mHz. Raw display-swap/queue/VSync and derived display tables have zero rows, explicitly counted. The permission-independent OS presented witness does not silently substitute those empty tables.

These are recordings **five and six** of the 11 November commission, with the preceding failed/successful attempts preserved. No further recording was made by this review. The builder's fresh OFF recording also serves as the OFF cadence arm, as the design permits.

[Recomputed OFF](M2-review/fresh-retained-off-traced.json), [recomputed ON](M2-review/fresh-retained-on-traced.json), [original exports and counter](../render/mac-m2-20261113/RUNBOOK.md).

### M2.2 — matched content and cadence

I verified the build/content/recipe/extent identity of the builder's ON/untraced, ON/traced and OFF/traced arms. The explicit current-build `--identity` replaces the old hard-coded identity. The record orders window qualification and witness-tool verification before OFF/M2.5, then the matched ON arms. All **851/846 ON originals** independently decode with correct hashes and image/request/surface joins, no omissions, monotonic accepted yaw, no exact repeats inside the sweep and complete endpoint holds. The 1.996286 s ON-traced pan-marker span reflects callback polling around the authored two-second elapsed sweep; source and normal control records cover both boundaries. It is not a fixed-step substitute.

The reviewer's fresh untraced window run additionally verifies 843 originals with no omissions or sweep repeats, holds 1.004215/1.104514 s and coherent onset/sweep/exit/stop in the [fresh strip](M2-review/live-window/on-untraced/review-10fps.png). Its largest gap, 44.641167 ms, is in the unmarked startup hold; a 16.672667 ms gap is in the final stationary hold. [Fresh untraced analysis](M2-review/fresh-untraced-analysis.json) retains both. These supplemental observations do not become a new cadence measurement.

OFF alone supplies cadence. The four long intervals inside Whole are:

| Drawables | OS interval | Refresh periods | Present-call gap | Destination yaw |
| --- | ---: | ---: | ---: | ---: |
| 365→366 | 16.666667 ms | 2.000000 | 8.238958 ms | 0° |
| 546→547 | 20.832417 ms | 2.499890 | 8.178458 ms | 11.249991° |
| 547→548 | 12.500041 ms | 1.500005 | 9.023125 ms | 11.399996° |
| 602→603 | 16.666708 ms | 2.000005 | 7.562292 ms | 19.949999° |

All 238 strict over-period OFF intervals retain drawable/time/yaw, game-call and opportunity/target joins, GPU samples and host load. Destination named GPU-query sums for the four above are 3.262/3.747/3.313/4.040 ms; later host render durations rise to 8–17 ms. These constrain the investigation without isolating a driver/compositor/scheduling cause. Run-wide load includes simulator paired-sync processes; it is not falsely treated as instantaneous CPU attribution. The ON-traced source gap 569→570 is **23.815875 ms**, beside a 17.042709 ms host render duration and 4.034125 ms named GPU sum. Its source pixels change and requested/accepted yaw continue monotonically.

Median host render durations OFF/ON-traced/ON-untraced are 1.275/1.311/0.822 ms; named GPU sums 3.946/3.936/3.980 ms. Run order and host load confound causal observer-cost percentages. The evidence records this limitation and investigates hitches rather than inventing a numeric performance gate.

Callback targets, opportunity times, acquire duration, submit/present-call records, GPU completion/query values, image times, movie PTS and OS presented times retain separate meanings. ON intervals are explicitly under capture. Kernel display swaps, scan-out, photons, input-to-photon and physical iPhone timing remain unmeasured.

[Recomputed content analysis](M2-review/recomputed-analysis.json), [independently extracted traced sequence](M2-review/builder-on-traced-sweep.png), [untraced sequence](M2-review/builder-on-untraced-sweep.png), [all missed-refresh investigations](../render/mac-m2-20261113/cadence.json.gz).

## Inspection list, observations and limits

The [inspection plan](M2-review/inspection-plan.md) was written before viewing images. Each cue was judged separately:

- **Rock/ridges and shadows:** coherent relative placement across native sources and sampled sequences; no separation or sudden rearrangement.
- **Sphere sweep/exit:** continuous leftward authored progression, including partial clipping then full exit; no reversal or replacement.
- **Endpoints/onset/stop:** the noon scene settles at declared 0°/36° yaw; both holds are complete. Ground E identifies framing only, not a pixel oracle or owner-approval gate.
- **Temporal faults:** no cut, snap or reversal in the sampled sequences. Full source timestamps and hashes expose local holds/repeats and omissions; the recorded gaps above remain observations, not hidden by slowed playback.
- **Live replacement:** intact known-pattern window and cooked scene before/after, exact boundary pixels, declared retired images and fresh-generation continuation.
- **OFF and cadence:** no capture resources/work in OFF, complete OS witness, all misses retained and investigated.

I inspected full 1920×1080 images, native-size crops and independently extracted **10 fps host-time frame strips** from the complete movies. I sampled motion; I did **not** perceive continuous 1× playback or physical-panel photons. This uses the project's prescribed frame-sequence route for an agent that cannot watch video. Black trailing contact-sheet tiles are padding, not captured black frames. Engine output and the separate actual-window screenshots are labelled separately.

Edges exercised: pan onset/stop, sphere leaving view, 120 Hz requested opportunities, capture ON/OFF, surface replacement with pending readback and stale refusal. Low sun, storm and fast camera were not rerun: these items prescribe a fixed noon slow-pan qualification. No Chrome or physical-iPhone result is inferred.

## Previous review, source scope and human feedback

I formed the [independent item view](M2-review/independent-notes.md) before reading the preceding review or its rationale. The item file necessarily exposed old verdict/name fields; they did not define the inspection scope. All three preceding Must Fix findings are resolved:

- `current-build-matched-arms-unverified`: new matched arms' entire retained build map agrees with the builder checkpoint; the hard-coded old identity is removed.
- `current-release-motion-unverified`: fresh builder originals and this review's fresh denied-permission realtime run are complete and inspected.
- `current-live-window-proof-unavailable`: the panel was accessible, the supplied full check passed, and fresh actual-window replacement/OFF proofs completed and were inspected.

I read the M2 implementation changes: AppKit pan/observer/signpost integration, actor lifecycle hooks, display-link window identity, pattern shader, permission launcher/driver, window packager, collector and recording-budget revisions, witness/tests, build/dependency/check registration, current-build identity fix and evidence analysis/qualification changes. I traced affected controls, FrameHost submission/capture, graph/presenter and recycle consumers under C02/C06/C07. No canned result, bypass of real submission, weakened test or default-enabled diagnostic was found. M2 is not this module's final milestone; this is not an exhaustive review of unrelated intervening module changes.

`RECORD.json` records exactly **“The screen is unlocked if you need it”** for M2.2 at `2026-11-12T06:57:44Z` and M2.5 at `2026-11-12T06:57:53Z`. The builder quotes that accurately. It authorizes access; it does not approve an image, waive cadence or choose an old executable. Current panel access was checked separately. Migration-broken links to old plan names are not attributed to the builder.

Final validation: `git diff --check` passes, all links in this review resolve, and the JSON audit confirms only M2 reviewer-owned fields changed. `./dev check-docs` exits 1 solely on existing links to renamed `ACCEPTED.json` plan files ([log](M2-review/fresh-docs-check.log)); none of the reported broken links is in the new review. This is the migration break explicitly excluded by the review task, not a builder finding.

Only the review, its evidence directory and M2 verdict/review/status/findings fields are changed. No code, other milestone, counters, builder evidence/result, commit or push is changed. Large reviewer originals and reconstructible expansions are retained locally as described in [retention](M2-review/retention.json); selected sources, records and movies remain here. Local masters are not claimed to have a durable backup.

PASS M2
