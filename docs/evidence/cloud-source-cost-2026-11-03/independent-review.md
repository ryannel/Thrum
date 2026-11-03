# Independent review: final Cu source CPU patch

3 November 2026. Read-only review of the two-file combined candidate in the
frozen CPU proof. I followed the `review-agent` skill and inspected the full
patch, surrounding source model and math, admission and renderer callers,
tests, and retained differential claims. I did not author this CPU patch.

**No findings.**

The reviewed target is `frame/src/source/model.js` SHA-256
`e21f9f5163f0a183136ce02a2d2cc9d6d5f6459943acfc5fa3254ba8ec8ae70e`
and `frame/src/source/math.js` SHA-256
`7f85fe25b759b7e1d37770b3e4bcea13199ef49dbb66362b0d79c621556bd862`
inside the retained CPU proof. Its `combined-source.patch` SHA-256 is
`5383d47b89ad7b80962432f465aaf1f1a0f724ab580cee91c18d174dbb801cf3`.
The comparison was the live c3e0 baseline: model SHA
`675edc8de1d0ad197434be3ae1b04ea4f842362fa1ec8d227e1ae878a0a7133b`
and math SHA
`1eca92cb5966bd5f58c416a0f28a37a3a02c37f6a22cfdff0b685bc3b640c553`.
I verified those bytes, confirmed that these are the only two changed `src/`
files in the final candidate, and ran `git apply --check` against the baseline.

The birth-frame cache is keyed by a newly copied, deeply frozen Q array. Its
dual is computed once from the existing `fieldColumns` arithmetic and is
shared through current/previous copies, endpoint and old-member geometry.
Fresh birth and model reset create new Q identities; slot and ID reuse do not
select a prior frame. Arbitrary mutable poses, including whole-Q replacement,
take the uncached path. Public presentation Q arrays remain independent frozen
copies. The fast bounds path checks their nine values against the birth Q and
evaluates the same eight support corners in the original operand order.

The exposure rewrite selects the same current and retained previous slot data
as the element loop, including a terminal previous part with the same active
ID. It fills every F32 output slot even after `allZero` becomes false, and it
tests source F64 values before conversion, so a positive value that underflows
to F32 zero still prevents the zero flag. I independently ran a no-write Node
check of signed-zero frame/bounds equality, mutable-Q fallback, and that
underflow case; all passed. I also inspected the retained author's finite
exact differential and contract receipts, without relabeling them as my runs.

This is code correctness review and a small independent CPU check. The retained
19,000-step/57,005-tuple and native-point suites are finite; they do not prove
continuous field equality. The matched Node timings are CPU workload evidence,
not browser playback, GPU, display, process peak or M2 admission. Actual
renderer/query images and motion require a separate integrated run. The
reported host for the author's timings is Node 24.16.0 on Darwin arm64 with
Apple M3 Max; my no-write checks do not qualify any hardware speed claim.
