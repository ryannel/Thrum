# PLATFORM-RUNTIME-MAC M2 independent review

Reviewed commit: `53137ce1506a37ec27c04082701d0e9ac1061bd9`, 12 November 2026.

The witness tooling and retained full-interval recording proof pass. Current-build motion, matched cadence/content arms and live-window qualification remain **Could Not Judge**. This is not a finding that the renderer visibly stutters. The current executable was built and content cooked, but the shared GPU was unavailable for the queued runtime (9 minutes 50 seconds waiting) and mandated check (9 minutes 9 seconds waiting); the attended panel was also locked and asleep. No previous M2 milestone review was present.

## Must Fix findings

1. **current-build-matched-arms-unverified — M2.2, Could Not Judge; evidence mismatch verified.** The claimed three arms all use executable `aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, reconstructed from `10287802`. The review checkout builds `3a23303c64f776260f01e92818feee9c81fbf1933cb4aecd28e6bff054ee13cd`; 96 of the 406 recorded build-source files differ, including render/present/capture paths. See the [independent source/hash comparison](M2-review-1/build-identity.json) and the builder's explicit [binary recovery](../render/mac-m2-presented-20261111/binary-recovery.json). The [claim](../render/mac-m2-presented-20261111/README.md) is candid about the old binary, but it cannot establish cadence of this checkout. `scripts/qualify_mac_m2.py:266` pins the old identity. Settle this with M2.1 and the matched ON/untraced, ON/traced and OFF/traced arms on the reviewed assembly, with M2.3/4/5 prerequisites satisfied, full source inspection and missed-refresh investigations. Preserve the six-recording counter; this review consumes no recording and grants none. The old arms remain useful historical evidence; M3's later obligation is not being pulled forward.

2. **current-release-motion-unverified — M2.1, Could Not Judge; missing current-run proof verified.** Building and cooking succeeded, but the current-build denied-permission realtime capture could not acquire the shared GPU. Consequently there are no reviewer-owned current-build images, permission endpoints or monotonic source records to judge. The 10 November motion evidence is for the different executable above. Complete the documented fresh 1920×1080 paused-world 120-opportunity/s pan, denied attributed preflights, complete lossless sources and host-time sequence, then independently inspect the list below. Console locking must remain witness-only for this offscreen item.

3. **current-live-window-proof-unavailable — M2.3, Could Not Judge; setup unavailable verified.** The outside-sandbox preflight reports built-in display 1 inactive/asleep and console locked. The required `./dev check` was queued under `gpu` but had not started. Thus neither its automated proof nor the reviewer's active-window pattern/valley replacement inspection is established for this checkout. See [panel observations](M2-review-1/panel-witness.jsonl) and [resource/run status](M2-review-1/run-status.json). Obtain a GPU slot and unlocked awake panel; run the required check and live known-pattern plus valley replacement/OFF scenarios. Preserve real submission joins, pending-readback retirement, stale refusal and healthy fresh presentation. Earlier passing tests and the focused integration diagnostic do not replace the mandated current full check.

These are proof gaps, not requests to repair an unobserved renderer defect. No Must Fix outside the items and no additional Suggestions.

## Results and independent evidence

| Item | Verdict | Basis |
| --- | --- | --- |
| M2.1 | `cant_tell` | Build/cook passed; current runtime capture unavailable. Historical source identity differs. |
| M2.2 | `cant_tell` | Retained arms are internally matched, but not the reviewed assembly; no fresh attended arms. |
| M2.3 | `cant_tell` | Source boundary reviewed; full check and active live-window inspection unavailable. |
| M2.4 | `pass` | Command independently reproduces all known misses and refusal controls. |
| M2.5 | `pass` | Independent re-analysis establishes the retained fresh recording's full-interval witness and conversion. This qualifies the recording method/recorded binary, not current-build cadence. |

[Build log](M2-review-1/build.log), [cook log](M2-review-1/cook.log), [passing documentation checks](M2-review-1/check-docs.log), [14 passing focused tests](M2-review-1/python-tests.log), [commands](M2-review-1/reproduce.md), [witness summary](M2-review-1/witness-summary.json).

M2.4: I read the command and tests and ran both retained TSV inputs. OFF finds 526→527 and 544→545, with the latter joining the **15.470792 ms** game-call gap; ON finds 558→559. Both old traces correctly remain unqualified because coverage starts after Whole begin. XML input filters `v1-game (PID)` in `ca-client-presented-handler`; the retained TSV route consumes the already selected table. Interval values use `at-time`, not callback time or delta. Tests exercise PID/ref resolution, altered callback/delta, missing ordinal, zero/absent time, unfamiliar status, duplicates, bad conversion, truncated coverage and bounded finalization. [OFF result](M2-review-1/known-off.json), [ON result](M2-review-1/known-on.json).

M2.5: I re-ran the witness on the later raw exported XML, inspected TOCs, marker brackets, display rows and panel observations. Both TOCs say **Deferred**, Blank plus Metal Application/Display/Points of Interest, with 16-second limits. Retained handler and converted presented spans cover both Whole markers. OFF joins **491 marked submissions**, 846 OS rows; ON joins **488 marked submissions**, 843 OS rows. Neither has unmatched samples affecting Whole. Converted-time error bounds are **8.5 ns / 9.496 ns**; marker offset widths are **625 ns / 708 ns**. The conversion is bracketed by four same-process signposts and checked between game opportunity and OS handler; the check does not substitute handler times for presentation. Only built-in display 1 is listed, at 3456×2234, with 120 Hz also in the window's mode record. OFF/ON have 153/143 panel observations, all unlocked, active, awake. Display-swap/VSync/queue and derived swap tables have zero rows and are explicitly recorded. The commission has **four of six** recordings consumed, including both failed configurations. [Recomputed OFF](M2-review-1/retained-off.json), [recomputed ON](M2-review-1/retained-on.json), [original TOC/tables and commission](../render/mac-m2-presented-20261111/README.md).

For M2.2's reviewer question, the retained records distinguish callback target, opportunity, post-present call record, GPU queries, image submit time, movie PTS and OS-reported presentation. Acquisition has a host duration rather than a separately retained absolute timestamp. ON timings are labelled under capture; OFF alone supplies the retained cadence result. OFF's Whole misses are 547→548 and 548→549 (16.665250 / 16.666667 ms), with game-call gaps 8.315917 / 12.193708 ms. All 236 strict over-period intervals have retained investigations; run-wide load is not falsely presented as exact causal attribution. Four ON Whole misses remain observer-arm measurements. Kernel swaps, scan-out, photons, input-to-photon and physical iPhone timing are expressly unmeasured. These answers are sound for the retained build, but do not close the inspection requirement on this checkout.

## Inspection list and observations

The [pre-inspection list](M2-review-1/inspection-plan.md) was written before viewing output:

- Rock and ridges: coherent relative placement through the prescribed sweep, whole frame and native crops.
- Sphere: authored sweep and exit, including start/stop transitions.
- Endpoints: settled yaw 0° and 36°, correct noon valley framing.
- Temporal faults: cuts, snaps, reversals, repeats, local holds and gross pauses against host time; complete one-second holds and two-second pan; paused authority and requested/accepted/submitted joins.
- Window: channel/row correspondence, normal surface submission and single lease; pending-readback replacement, stale refusal and healthy continuation; capture OFF direct behavior.
- Cadence: full Whole coverage, every submission accounted for, conversion error, per-miss investigation, separate ON cost/content.

I inspected the retained traced native 1920×1080 frame and native crop contact, plus independently extracted **10 fps** strips over seconds 3–7.1 of both ON movies: [traced strip](M2-review-1/retained-on-strip.png), [untraced strip](M2-review-1/retained-untraced-strip.png). The rock/ridge positions stay coherent, the sphere sweeps left and exits, and sampled endpoints settle without a visible cut or reversal. Black trailing strip tiles are layout padding, not source frames. I sampled sequences; I did **not** watch continuous 1× playback. These older sampled views cannot exclude short holds in the new executable or prove current output. No Ground E pixel matching or owner image approval was used.

Target edges: retained noon slow-pan/full-refresh records and code paths for loss/replacement/OFF were examined. Fresh live replacement, full refresh, low sun, storm, distance and fast-camera runs were not reached. The latter four are broader platform edges rather than substitutes for the prescribed noon qualification. No Chrome or iPhone inference is made.

## Source and authority review

Read M2's permission launcher/driver changes, native AppKit pan/observer/signpost bridge, display pattern, window packager, collector, witness and tests, plus the changed actor/AppKit integration and their current renderer consumers. The pan uses monotonic `Instant` and normal slider target/action; requested controls do not mutate authority directly. The actor acquires once, submits, presents, then observes. Capture consumes the renderer output and does not reacquire a drawable. Replacement invalidates the tap; owner/session/device checks reject stale recycling. OFF does not enable tap or writer resources. Renderer submission/history remains on the ordinary queue, with the existing lost-device abandonment seam. The independent pattern oracle compares exact RGBA code bytes; it is not compositor observation. These source findings support the mechanism, not unavailable live proof.

The unrelated intervening renderer changes were identified by exact source hashes; this review does not claim exhaustive review or runtime equivalence of those changes. M2 is not the module's final milestone. The old full-check integration failure remains unexplained by a passing focused test; no change or fix was made here.

Owner-choice audit: `ACCEPTED.json` records exactly **“The screen is unlocked if you need it”** for M2.2 at `2026-11-12T06:57:44Z` and M2.5 at `2026-11-12T06:57:53Z`. The builder quotes these accurately. They authorize attended setup; they do not choose a look, approve the old binary for this new assembly, waive checks or assert that the screen remains unlocked now. The current preflight is separate evidence. No security settings or permission grants were changed.

Only this review, its evidence directory and M2 verdict/review/state/findings fields are changed. Counts and builder status/evidence remain for the loop. No agents, commits or pushes.

FAIL M2: 0 fail, 3 cant_tell, 0 outside the items
