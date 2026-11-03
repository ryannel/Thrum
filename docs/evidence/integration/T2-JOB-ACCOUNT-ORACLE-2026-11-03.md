# T2 first-host job-account reference oracle

3 November 2026. A deterministic Python 3.11 reference model challenged the
[provisional same-volume account](../../design/research/T2-SAME-VOLUME-JOB-ACCOUNT-2026-11-03.md).
The code baseline was `50024fa`; the selected input-document snapshot is
`fa8bc43`, and the result file records its exact hashes. The [archive](T2-JOB-ACCOUNT-ORACLE-2026-11-03.tar.gz)
contains the editable model, exact input hashes, JSON results, full report and
scratch ownership record. Archive SHA-256:
`bff4d27a8a3f2f3b67ce0eefcbce3e729c77ac6c17da938cc6c5f40071e362ae`.
Model SHA-256:
`0eb12f4a448f9661317d59ec210d7b61b398aaedc080e8a2118ef0e4413bd514`.
Result SHA-256:
`1c7b0b4923978b36ace1d1b387a32fb5b38bc1e533e4f8e6d3ad626be59c8834`.
The archived model and report name the exact input documents; their hashes are
in `results.json`.

Run `python3.11 model.py` after extracting the archive at the repository root
with the `fa8bc43` input documents present for a byte-identical result. Later
design edits change the recorded input hashes. The local selected rerun returned:

| Bounded check | Result |
| --- | ---: |
| Two-owner scalar admission comparisons | 3,440,640 |
| False admissions within the declared future-growth envelope | 0 |
| Conservative deferrals from lagging reports or full compiler claims | 314,464 |
| Four-class complete-plan comparisons against private sequential trial | 147,456 |
| Reachable lifecycle states / attempted operations | 14,620 / 321,640 |
| Abstract startup combinations / idempotent replay cases | 16 / 160 |
| Intentionally wrong algorithm witnesses | 13 |

Adversarial review found the first model's temporary safety oracle reused
`Job.charge()`, allowing an allocated-only charge mutant to pass its scalar
check while two admitted jobs could exceed the target. The selected rerun
computes exact temporary demand independently from allocated bytes and future
work, checks that aggregate after every lifecycle successor, and executes the
mutant as a rejection test. It rejects the mutant in both scalar exploration
and successor safety. The report in the archive records this correction.

The bounded checks support exact job-incarnation ownership, atomic all-class
registration, report versioning, no refund on cancel or family exit, keyed
restart replacement, retained-root/reader checks and unknown-orphan blocking.
They also expose conservative deferral as a real cost of uncertain compiler
growth. The model's multi-class cross-check shares the scalar policy predicate;
it checks transaction shape, not an independently derived physical allocator.

This is logical evidence only. Jobs have at most two live tokens in lifecycle
exploration, tiny byte values, abstract root and durability inputs, and a
single serialized account. It does not exercise filesystem identity, F3/F4
integration, SQLite, compiler processes, crash persistence, allocation peaks,
external writers, latency or a physical free-space guarantee. The zero-false-
admission result assumes actual future growth stays within its declared
estimate and no unrelated writer consumes availability after the observation.
Native failure handling, restart tests and the combined workload remain open.
