# CONTENT-LOOP M1 review

**FAIL — M1.3 fails; the other nine items pass.** One verified Must Fix, no outside-item findings.
Reviewed `cf47dfbf4068ed727f658263208e97655019fca5`, with builder endpoint `79a42debb607`, on 13 November 2026.

## Must Fix: unchanged-product-reuploaded-behind-ready-publication

**M1.3 is not true for overlapping edits.** A publication prepared behind a ready, unswitched publication
uploads an unchanged product again and replaces its texture/view. Its change serial nevertheless stays the same.
Verified on this Mac's Metal GPU with the public game frame host and lease interface; no production code changed.

Reproduction: [source](M1-review/overlap.rs), [runner](M1-review/run_overlap.py),
[output](M1-review/overlap.log), [checkout build](M1-review/build.log).
From the repository root, with Metal access, run:

```sh
python3 docs/evidence/CONTENT-LOOP/M1-review/run_overlap.py
```

1. Cook and load twelve distinct 128³ volumes plus the texture, using portable products.
2. Edit only `cloud0` and submit its final copy. Generation 2 is now offered/ready, while generation 1 is still drawn.
3. Before submitting the switch, edit only `cloud1`; wait until the loader has staged generation 3.
4. Submit the generation-2 switch, then advance normally until generation 3 switches.
5. Compare `cloud0` between generations 2 and 3 and inspect their uploaded-byte records.

Observed: `cloud0 unchanged hash=true, texture=false, view=false, serial=true`.
Generation 2 uploads **5,592,320 bytes**; generation 3 uploads **11,184,640 bytes**, although its only new
product is the equally sized `cloud1`. The unchanged-view assertion fails (exit 101).
Expected: generation 3 retains `cloud0`'s texture and view and uploads only `cloud1`'s 5,592,320 padded bytes.

The loader reuses only `confirmed`, updated from the submitted resident set
([load.rs:203](../../../crates/thrum-asset-gpu/src/load.rs#L203)). A later HEAD is prepared immediately against
that older set ([load.rs:211](../../../crates/thrum-asset-gpu/src/load.rs#L211)), even while the ready publication
is protected from supersession in [lib.rs:787](../../../crates/thrum-asset-gpu/src/lib.rs#L787).
The newer prepared set is not reconciled after the ready one switches. Meanwhile
[lease.rs:78](../../../crates/thrum-asset-gpu/src/lease.rs#L78) preserves the serial by hashes and device generation,
without checking that the view was actually retained. This violates both M1.3's unchanged-resource/upload promise
and the design's rule that the serial moves when the view does.

Resolve reuse across this ready-to-switch boundary and cover the overlapping-edit sequence, including unchanged
handles, uploaded bytes and serial/view correspondence. The existing test edits again only after the preceding
switch, apart from superseding publications that never become ready, so its passing result misses this case.
I reproduced the defect on the portable native profile. I did not reproduce it separately on BC4 or Chrome;
those paths share the relevant preparation/offer logic, but that is source evidence, not additional device proof.
No pixel corruption is claimed: the redundant view contains the same texels.

## Scope and method

I read the M1 design and requirements before the builder's evidence and log, then read all changed source and
documentation lines in `54ab55bb..79a42debb607`, including the diagnostic, browser harness, benchmark, fault probe,
public guide and proof scripts. `git show 79a42debb607` itself adds only the final check log entry; the implementation
begins at `8bab155a`. I also followed the producer/consumer boundaries through residency, staging, the contribution
list and the frame host's normal/capture submission callbacks, and read the relevant C05–C07 contracts.

The supplied clean-checkout results establish `./dev check` and the nine item checks on the builder endpoint.
I accepted those results rather than rerunning the suite. I inspected their assertions and the retained target
records; the independent overlap reproduction addresses a gap those checks do not exercise. Its libraries were
built from this checkout with `cargo bench --locked --offline -p v1-game --bench cooked_edit --no-run`.
The GPU run held the repository-wide `orchestrate-gpu.lock`. The sandbox exposed no Metal adapter; the permitted
outside-sandbox run reached Metal and reproduced the assertion failure.

M1 has no `inspect` proofs and no human-feedback requirement. There was therefore no visual inspection list or
motion judgement. No screenshots or recordings were used to infer streaming quality. The target edges pertinent
to these items are discard, capture, supersession, failed load, replacement device, contribution order and the
added overlapping edit. Low sun, storms, distance, fast camera and physical full-refresh presentation are not
claimed by this lease/function review; the design assigns the reconstructed visual edit sequence to M3.

## Item verdicts and proof assessment

| Item | Verdict | Judgement |
|---|---|---|
| M1.1 | pass | `LentDiagnostic` holds only `ProductLease`, its own device-bound pipelines/bindings and diagnostic outputs. It has no store or loader. Its compute pass fetches each mip through public views. The separate test oracle maps `portable_hash` through `thrum-asset`, checks all portable bytes and output length, and checks each preferred BC4 base against 30 dB. Twelve seeded 128³ volumes are distinct. The ordinary test run covers portable and preferred devices. |
| M1.2 | pass | `until` records and discards every upload frame before submitting a replacement recording; it asserts the offer is unchanged after discard, including the final-copy recording. The test checks old drawing in the last-copy frame and new availability afterward. Generations 3 and 4 exceed the four-chunk ring; generation 3 is superseded before it can finish and never swaps. Source confirms offers change only after copy submission and a ready set waits intact for its switch. |
| M1.3 | **fail** | The ordinary isolated edit proves retained identities for eleven volumes, exact padded upload bytes for the changed one, mip bind-group creation without pipeline creation, and invalidation surviving both discard and capture until a history-advancing submission, then clearing. However, the independent overlapping-edit reproduction above breaks the same item's unchanged-view and changed-bytes-only promises. Passing the supplied test cannot override that counterexample. |
| M1.4 | pass | Coupled placement/lease tests assert the single copy budget and four-chunk staging bound. The release `cooked_edit` leased scenario actually attaches the consumer and loads twelve unique volumes plus placements, then makes three uncached one-volume edits. The retained quiet-load run reports 20.43 MiB first load, 8 MiB maximum per frame, and 24 MiB peak staging against a 32 MiB bound. Render CPU upload times are not all within steady spread: 4.203 ms cold first frame and 2.140 ms initial consumer allocation versus 1.673 ms steady maximum. The evidence names host/copy preparation and first output-buffer/bind-group creation; the three copy-bearing edit frames are 1.209, 1.393 and 1.250 ms. This answers the item's explicit cost question; it does not qualify panel cadence. |
| M1.5 | pass | The feature-gated probe fails `write_chunk` at the mapped-write boundary. The ordinary `fill` error branch calls `abandon`, marks failure and forgets the slot; the loader records it and residency drops the failed pending set. The test retains the old lease, recovers within the ring bound, and finishes with exactly four failures for the four injected/refused cases. Missing leased name, wrong dimension and device-refused extent are coupled with a placement-texture edit; unchanged placement pixels and lease view prove whole refusal, and diagnostics name `lease density`. Ordinary compilation excludes the probe. |
| M1.6 | pass | The native test creates a replacement device, destroys the old device, publishes the new host candidate and checks both lease and consumer witness empty. The consumer rebuild clears old bindings and keys. Content restages with generation 2 and the test rechecks portable texels/preferred quality. This is actual replacement through the frame-host lifecycle, not a changed integer on old views. |
| M1.7 | pass | The Chrome harness cooks a real fixture, serves HEAD/CAS bytes, and runs the game consumer against `DevClient`/`RemoteFeed` on hardware WebGPU. It checks every portable mip after load, edit and replacement; retained records report 28,766,400 checked bytes each time. Request accounting demands exactly one changed product and one manifest, each fetched once. Recovery destroys the old device and reacquires an adapter/device, checks an empty lease, then checks the restaged texels. This is function evidence, with no timing claim. |
| M1.8 | pass | The public guide gives request construction, obtaining and attaching the lease, empty/loading state, whole-publication readiness, order independence, serial comparison and committing only in `submitted(true)`, discard/capture handling, device rebuild and dropping old bindings. It specifies pinned portable CPU identity and consumer-owned meaning/history/pipelines. The public lease types contain generic product/GPU data and no game, sea, cloud or placement types. SEA/CLOUDS can design their pass from it; the implementation defect in M1.3 is separately recorded. |
| M1.9 | pass | The supplied final check passed the registered native and Chrome verifier at the claimed files. The verifier builds/cooks the candidate, runs native comparisons and four Chrome comparison sets, propagates failures, and records content identity only after all pass. No retained baseline or comparator is changed by M1. Older archived proof identities are labelled as such; final endpoint coverage relies on the supplied script result, not an assumption that an older log covers later edits. |
| M1.10 | pass | The coupled test first obtains all-old placement pixels and consumer words, then retains every submitted capture through upload, switch and the following frame. A separately prepared host on the edited store provides all-new outputs. Every pair must be wholly old or wholly new, with explicit non-equality of old/new references. It runs both contribution orders and discards the ready switch before retry. No masks/tolerances or direct mutation around the host token weaken the comparison. |

The M1 quality bar is not fully met because unnecessary replacement uploads occur under an ordinary supported
edit ordering. The review does not claim a measured hitch from that defect. No unrelated regression, weakened
baseline, canned GPU result or production-enabled fault probe was found. No additional Suggestions.

Set M1 back to `in_progress`, retain builder statuses, and record the single M1.3 finding. M2 and M3 are untouched.
