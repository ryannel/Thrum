# CONTENT-LOOP M1 review

**PASS — all ten items pass. No Must Fix findings or Suggestions.**

Reviewed `84a1158ec37f3c8ff83c0f0a321050e262c89f7a`, builder endpoint `0e2feaeab813`,
on 13 November 2026. This replaces the previous M1 review.

## Scope and method

I judged the requirements, design, implementation and test assertions before reading the builder's evidence,
log and previous review. I read the M1 changes from the initial lease implementation (`8bab155a`) through
this endpoint: residency, loader, staging, lease API, diagnostic and GPU tests, browser implementation and
harness, generated fixtures, benchmark, public documentation and proof script. I also checked the ancillary
historical-link migration and added tripwire record. The endpoint commit itself only records final checks.
I followed both sides of the lease interface and the contribution list through normal and capture submission
in the frame host, against C05–C07 and C12. No reading or judgement was delegated.

The supplied clean-checkout result establishes `./dev check` and all nine automated item checks on
`64a5b4c27adf`. The changes from there to this checkout do not change the implementation or its tests.
I accept those runs, as the review skill permits, rather than rerunning passing checks without a reason to
doubt them. I examined what each command actually proves, including the exact verifier's build, comparison,
failure propagation and content-key publication. Archived builder logs corroborate earlier runs; they are
not substituted for the supplied current-file check result.

M1 has no Reviewer Inspection or human-feedback proof. No inspection list, screenshot or motion judgement
is applicable, and I made no new device run in this review. The supplied tests exercise discard, capture,
supersession, overlapping edits, missing/invalid/device-refused products, staging failure, device replacement,
and both contribution orders. Low sun, storm, distance, fast camera and physical full-refresh presentation
were not exercised here. M1 qualifies lease function and bounded streaming; the design assigns the real-time
reconstructed visual edit sequences and broader frame-time qualification to M3. No panel-cadence or physical
residency claim follows from this pass.

## Item verdicts

| Item | Verdict | Proof assessment and answer |
|---|---|---|
| M1.1 | pass | `LentDiagnostic` receives only a public lease and owns its own GPU bindings, pipelines and diagnostic output. It has no store or loader. Its compute pass fetches all mips of twelve distinct generated 128³ volumes and a 2D texture. The independent oracle in `assert_texels` maps the portable products through `thrum-asset`, checks the portable bytes at every mip and the total output length, and checks each preferred BC4 volume's base level against the existing 30 dB floor. Both profiles are exercised. The consumer's output is computed from the lent views, not supplied by the oracle. |
| M1.2 | pass | Offers are published only in residency's `submitted` callback after the final copy commits. The last-copy recording uses the old offer; a ready set then remains protected until its switch submits. The `until` helper discards each upload recording, including the last, and asserts the offer remains the same before submitting a replacement recording. First-load checks confirm no drawn set in that final-copy frame. The test also stages generations 3 and 4 without allowing generation 3 to finish; the ring cannot hold that publication and generation 3 never swaps. These are real tentative-frame paths through the host. |
| M1.3 | pass | The isolated edit checks exactly one changed product, retained hashes/serials/textures/views for the others, and upload bytes equal to the edited product's padded mip rows. Bind-group counts increase by that product's mip count while pipeline counts remain fixed. A discarded first-use recording followed by capture leaves invalidation pending for the next history-advancing submission; the following frame clears it. The overlapping-edit test extends this to a newer publication prepared behind a ready switch, on native and fetched feeds, portable and BC4, with either contribution order. Its checks cover retained resources, serial/view correspondence, exact upload bytes, sampled texels and invalidation. The implementation now rejects stale-base plans before recording copies and replans against the submitted resident set. |
| M1.4 | pass | Placements and leases share `StagingPlan`, ring and upload recorder. The coupled test asserts the per-frame copy maximum and four-chunk bound. `cooked_edit` contains a real leased scenario beside placements, first loading twelve unique volumes and then making uncached one-volume edits. The retained release run records its quiet load class and individual upload-frame CPU times beside steady frames. Not all upload times fit the steady spread; the causes and scope are explicitly recorded below. This satisfies the item's question without turning the benchmark into a cadence claim. |
| M1.5 | pass | The optional probe returns an error at `write_chunk`'s mapped-write boundary. Production `fill` handles it through `abandon`, marks the upload failed and removes its ring slot; the loader records the failure and residency drops the pending set. The test recovers, checks four total failures, and stays within the ring/copy bounds. Missing leased name, wrong dimension and refused 256³ extent are coupled with placement-texture edits: the old placement pixels and lease view remain, and errors name `lease density`. The strengthened dimension case proves its invalid D3 request resolves to an already resident D2 placement product, so reuse cannot bypass validation. The next good generation switches. Ordinary compilation excludes the probe and its test unless the feature is enabled. |
| M1.6 | pass | The native test prepares a real replacement device through the host, destroys the old device and publishes the replacement. Both lease and consumer witness are empty immediately afterward. Consumer rebuild clears bindings, observed keys and recorded keys. The set is staged anew on device generation 2 and all portable texels/preferred quality are checked again. A retained integer on old handles cannot pass these checks. |
| M1.7 | pass | The Chrome case serves cooked HEAD/CAS data and uses `DevClient`/`RemoteFeed` with the same consumer. Its WebGPU readback verifies every portable mip after first load, one-volume edit and device replacement. The harness requires exactly two distinct post-edit CAS requests, each once: the changed product and manifest. The edit compares product identities and must change exactly one. Recovery destroys the old device, acquires a fresh adapter/device and publishes it through the host, checks the lease empty, then verifies the restaged texels. The supplied Chrome check passes; the archived record reports 28,766,400 checked bytes in each phase. |
| M1.8 | pass | The public guide gives request construction, lease acquisition, contribution attachment in either order, empty/loading state, whole-set readiness, protected ready switches, serial comparisons and committing observation only in `submitted(true)`. It explains discarded frames, capture, rebuilding and dropping old bindings on device replacement, pinned portable CPU identity, and consumer responsibility for meaning/history. Together with the design this is enough for SEA/CLOUDS to design a sampling pass. Public lease types contain generic product/GPU data, with no game, sea, cloud or placement type. |
| M1.9 | pass | The supplied verifier run passed on the current implementation files. The verifier builds native and Wasm candidates, cooks the fixture, runs both native matrices and four Chrome sets, propagates comparison failures, and publishes its content-key proof only after success. M1 changes neither retained references nor comparison tolerances. Older archived proof revisions remain historical evidence rather than being presented as current-file verification. |
| M1.10 | pass | The coupled test captures every submitted frame from the edit through the frame after the switch. Each whole placement image and consumer output must jointly equal the all-old pair or the all-new pair. The latter comes from a separately prepared host on the edited store at the same paused view; explicit inequalities establish that both outputs actually changed. The test runs both contribution orders and discards the ready switch before retrying it. There is no mask, tolerance or direct mutation around the frame-host token. |

## Timing and quality bar

The retained [leased benchmark](M1/benchmark.md.gz) is a release, 640×360, waited offscreen run,
load 3.6 on 14 CPUs, classified quiet. First load copies 20.43 MiB in three upload frames; each edit copies
1,398,528 bytes. Maximum per-frame copies are 8 MiB and peak staging is 24 MiB against a 32 MiB bound.
Steady render CPU p50/max is 1.015/1.673 ms; upload p50/max is 1.393/4.203 ms.

The [evidence's cost explanation](M1.md#cost-and-limits) names cold host/copy preparation for the initial
4.203 ms frame and first consumer output-buffer/mip-bind-group allocation for the 2.140 ms zero-copy switch.
The three copy-bearing edit frames are 1.209, 1.393 and 1.250 ms, inside the steady spread. The benchmark source
classifies pending and switch frames as uploading, so those costs are not hidden by excluding zero-copy frames.
This answers M1.4's explicit request for named causes when times exceed the steady spread. It does not prove
that every displayed frame meets 120 Hz. The new leased tripwires are still seeding, as the evidence states.

For this milestone's defined scope, copies are bounded, products are never exposed half loaded, all readers
switch together, and refused loads keep the last good content. I found no additional regression, weakened
oracle, canned GPU result, default-enabled fault path or violation of the shared publication design.

## Previous Must Fix

After forming the item judgements above, I read the previous review and `milestones.M1.reviews` in
`RECORD.json`. Its sole Must Fix, **`unchanged-product-reuploaded-behind-ready-publication` (M1.3), is resolved**.

`PendingSet::base_head` records the resident resource base used to prepare an upload. In
`CookedAssets::encode_upload`, a mailbox plan with a different base from the now-current resident HEAD is
rejected before encoding any copy. The native loader replans the latest HEAD when it receives a confirmed
resident set; the remote path includes the confirmed HEAD in its preparation key. Thus the newer publication
prepared behind generation 2 is rebuilt after generation 2 switches, reusing its unchanged products.

The new `overlapping_edits_reuse_the_ready_publication` regression covers the prior reproduction and expands
it to eight combinations: native/fetched, portable/BC4, and both contribution orders. It stages generation 3
behind ready generation 2, discards the switch, switches through capture, and checks that only generation 3's
edited volume gets new handles and uploaded bytes. This regression is part of the supplied passing current-file
check. I did not rerun the old standalone reproduction; its retained failure log describes the earlier revision,
not this candidate. No previous finding remains and no new finding is raised.

M1 is set to `done`, with all ten item verdicts `pass` and their review links pointing here. The script owns
clearing the previous findings and updating review history/counters. Builder fields, other milestones and
production code are unchanged. The review changes are left uncommitted for the script.
