# R0 resource lifetime schedule: logical oracle, 3 November 2026

The selected [schedule candidate](../../design/research/R0-RESOURCE-LIFETIME-SCHEDULE-2026-11-03.md)
was exercised by a standalone, dependency-free [Rust oracle](R0-RESOURCE-LIFETIME-ORACLE.rs).
The source was developed from repository revision
`d06b020a5b443de692156873cf1d1f39cb202cff`; its SHA-256 is
`d0d4a76ea83386cf3a2ddef8ee50c155e5335eb11fb5c8308c53ba98875cad5c`.
The proof uses Rust 1.99.1, one logical device/queue, four fixed texture roles,
one spatial view, no UI and no capture. It makes no GPU call and renders no
pixel. All bytes below are logical payload estimates, not allocator residency.

Reproduce from repository root:

```sh
mkdir -p tmp/r0-resource-lifetime
rustup run 1.99.1 rustc --edition=2021 --test -D warnings docs/evidence/render/R0-RESOURCE-LIFETIME-ORACLE.rs -o tmp/r0-resource-lifetime/oracle
tmp/r0-resource-lifetime/oracle
```

The run passed **10/10 tests**. The oracle computes 35,209,216 bytes for the
720p V1 scratch set and 58,249,216 for 1080p. A two-frame duplicated 720p
set would be 70,418,432 bytes; the selected same-queue schedule retains one
35,209,216-byte set while two submissions are outstanding. A third reservation
refuses before it may acquire packet/staging work. This is an allocation-count
comparison, not a measured performance gain. Output is charged separately:
3,686,400 bytes at 720p, 8,294,400 at 1080p and 11,980,800 during overlap.

| Exercised transition | Observed logical result |
| --- | --- |
| 60 MiB V1 policy and 720p→1080p resize | 76,681,216-byte old/new overlap refuses; completing N alone leaves N+1, and completing both still leaves the prepared old graph. Explicit stop/release admits 58,249,216-byte new-only graph. |
| Healthy-device candidate failure | Pending view allocations roll back; old graph stays prepared. A lease on its shared shadow does not block candidate rollback. |
| Failure after old graph release | No old graph is resurrected; proof-local loading diagnostic state remains. |
| Submitted/retired generations | Published overlap keeps old view slots charged until their last submission completes; compatible shadow is charged once. Retired CPU leases block stop. |
| Device loss and stale work | Old GPU handles/credits become invalid; CPU lease reasons remain until released. Old completion and old reserved packet cannot consume new-device credit or advance the submitted sample. |
| Arithmetic and output | Extent and output-overlap overflow refuse before allocation; old mock output remains until the last old serial completes. |

Two Sol High design reviewers challenged the schedule. Their concrete findings
led to explicit stop/release progress, device-loss invalidation, two-credit
admission, requested-device checks, separate output accounting and an UI-off
scope. A Sol High oracle reviewer constructed failing adversarial cases for
stale reservations, pending/retired CPU leases, shared-shadow rollback and
output overflow; each was fixed with a regression. These are read-only design
and source reviews, not device observations.

**Unproved:** The model does not allocate GPU resources, validate pass
first/last uses or the full descriptor/usage key, measure high-water/frame
time, render pixels, own a real surface or execute the loading diagnostic.
It does not charge F4 product/upload/staging work or prove credit coverage in
that real path. Partial GPU allocation failure, incompatible shadow descriptors,
two simultaneous views, readback/capture, UI composition and the named target
devices remain separate tests. The 60 MiB policy is a deliberately chosen
test point between new-only and overlapping scratch costs, not an engine limit
or a device-memory guarantee.
