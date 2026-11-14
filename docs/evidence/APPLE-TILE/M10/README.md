# APPLE-TILE M10 builder record

14 November 2026. D10.1–D10.4 pass on their named targets. D10.5 remains open for
attended OS-window access. M10 stays `in_progress`; no MetalFX, speed, panel or
physical-iPhone qualification is claimed.

## Target and build identity

The small same-submit fixture passed at `cdede389` before route integration
([fixture result](fixture-cdede389.txt)). Integrated lifecycle tests first passed
at `d29d2667` ([result](lifecycle-d29d2667.txt)). The full official M10 check and
`./dev check` pass on clean source `07282556025cc4fbeb3293b5b106a87c79b37d22`,
outside the sandbox under GPU exclusivity ([stdout](check-07282556.stdout.gz),
[stderr](check-07282556.stderr.gz)). The earlier coordinator attempt stopped
before pixel comparison because three deliberate discards were expected to cause
zero capture omissions; `342151fc` requires exactly three. The corrected test
passes, without a changed pixel metric.

The completed coordinator/simulator/T2 campaign used clean source
`8078205cc09de145f58348af42a8e2524cb01891` ([actual log](target-8078205c.log.gz)).
Its engine sources are unchanged from the passing repository checkpoint.
Native executable SHA-256 is
`94718c6c972b55634b89e02e9d3863ed6a0190b9a95cbd955cabe5be32b3dc0f`.
Per-run manifests retain tool/host, source-file hashes, cooked/route identity,
exposure and original writer/decoded-frame hashes.

## D10.1 and D10.3: boundary, retention and reset

The [independent scoped design review](../design-review-1.md) approved the boundary
before implementation. Graph-native refusal/hazard tests and actual locked-wgpu
mixed-API refusal tests pass. Native writes require initialized retained resources;
the late-bound surface and attachment operations are rejected. The provider
initializes distinct output/history textures through wgpu. Its native backend
owns no queue, surface or completion. Recording ends the producer wgpu encoder,
uses one whole raw encoder, then resumes wgpu; the host submits that ordered list
once on its existing queue. Completion retains the binding/provider/textures.

All four integrated Metal tests pass: exact SDR/scene/output/history parity and
five omission/stale/reset controls; two submissions held before completion;
failure/loss/extent replacement; old-generation retention through completion
delivery. Cuts and discarded candidates reset internal history. Acquire refusal
drops native commands and flushes healthy uploads; background cancellation and
observed loss avoid queue work. Submission failure does not commit image history.

## D10.2: exact stateful diagnostic and visible output

The [campaign manifest](diagnostic/manifest.json.gz) passes all sixteen paired
still/fixed comparisons on portable and Apple tile: 3,856 paired frames across
SDR, scene-linear, native output and native history. The [retained checker](validate_half.py)
and [result](half-validation.json.gz) validate every half texel in all 24 half-float
runs, decoding four unique exact streams and checking 7,995,801,600 values.
Command: `python3 docs/evidence/APPLE-TILE/M10/validate_half.py tmp/g3t-m10/diagnostic --output tmp/g3t-m10/half-validation.json`.
The binary-exact native tile begins 1, 0, 1; prior/next histories are distinct.
NoCommands, NoTile, NoReset, StaleSource and StaleHistory all fail exact output
and history comparisons in the coordinator test.

Both realtime routes pass complete source verification: portable has 960 samples
through 8.018789542 seconds; Apple tile has 963 through 8.019857917 seconds.
Both cross history epoch 1→2 at sample 524. Original host-time movies, contacts,
full-size cut-adjacent frames and clocks are retained under
[portable](diagnostic/portable-realtime-native/) and
[Apple tile](diagnostic/apple-tile-realtime-native/); [selected identities](diagnostic/selected.json)
locate each original frame. Builder inspection covered full-size fixed endpoints,
terrain/sky, rock, reflective sphere, cubes, lights/shadows, contacts and before/after-cut
samples on both recipes. Geometry and lighting remain intact; no stale full-frame
image or ghosted region was seen after reset. Motion judgement uses the source-frame
sequence and measured host times, not a claim of uninterrupted video playback.

Ordinary witness-Off scenes pass the [exact regression campaign](baseline-07282556.json.gz):
38 native route comparisons, four surface/letterbox comparisons and 1,924 Chrome
day/night SDR/linear frames. References and tolerances are unchanged. This verifier
retains manifests/digests and releases successful bulk sources; its hashes establish
regression equality, not a fresh visual review.

## D10.4 and D10.5: simulator function versus attended window proof

The actual iPhone 18 Pro / iOS 27.2 simulator
[report](simulator/report.json.gz), [source identity](simulator/source-files.json)
and [route log](simulator/route.log) pass. Simulator executable SHA-256:
`8bdff183b5436110e4ce1f506e700082662acf1aa67e55a78eef0bad461aaa8e`.
During the encoded-frame background race, submitted/presented stay at 1,103,
empty submits stay zero, encoded rises 1,103→1,104 and discarded 0→1.
The suspended counters remain quiescent. Same-process resume retains world tick
1,151 and advances provider resets 1→2 before resumed presentation.
Builder inspected the [background endpoint](simulator/m10-native-refused-background.png)
and [resumed endpoint](simulator/m10-native-resumed.png). This is diagnostic
function/UI/lifecycle proof only, without cooked dependencies or submitted temporal,
speed or physical-device claims. The owned simulator was shut down and deleted.

D10.5 still needs the owner's unlocked active Mac and existing Screen Recording
grant for the OS-window observer. No grant automation or actual-window pass is claimed.

## Retention and integration

Selected original images, movies, manifests and source identities are committed.
All M10 lossless spools remain in `tmp/g3t-m10/diagnostic` through review.
[Hash-verified duplicate linking](source-deduplication.json.gz) preserves every
path and byte while sharing identical completed files. Debug cleanup released
8.8 GiB after the Cargo tests finished; release executables were preserved.
No Instruments recording or temporary Python environment was created.
The earlier [M3 release receipt](m3-expired-bulk-release.json.gz) describes only
expired historical sources; no recoverability of released bytes is claimed.
Integration and any required rebased baseline/gate run use the official merge
script. The first integration attempt rebased to `012668c2`, then stopped at the
guard because upstream proof-script changes require a fresh baseline key.
All 500 captured build-source files [still match](rebase-source-identity.json).
The [fresh baseline proof](baseline-ca8b41f5.json.gz) passes on `ca8b41f5` in
920.36 seconds with the unchanged native executable. The following full gate
stopped on the unrelated watcher test's ten-second generation-2 timeout
([stdout](check-ca8b41f5-watcher-failure.stdout.gz),
[stderr](check-ca8b41f5-watcher-failure.stderr.gz)). The exact failed test binary
then passed all four tests and three further consecutive runs
([actual reruns](watcher-rerun.json)). The timeout's cause is unconfirmed; no
test, threshold or engine source changed. A complete official gate retry is required.
The [handoff](../M10-handoff.md) records the integration retry and attended item.
