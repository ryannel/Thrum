# Safe source request-credit ledger — 3 November 2026

**Result:** the bounded safe model passes its credit and owner-close protocol checks.
Retain this as a prerequisite and reference for a later native adapter; it does not
justify production unsafe callbacks, admitted-source authority or physical reservation.
The [requested-I/O design](../../design/research/SOURCE-REQUESTED-IO-FENCE-2026-11-03.md)
remains the authority. Its exact input text, executable source, deterministic inputs,
raw logs and tool identities are retained in the [selected proof archive](SOURCE-REQUEST-CREDIT-LEDGER-PROOF-2026-11-03.tar.gz).
The archive is **35,103 bytes**, SHA-256
`e9c58a34b5ad9127a8e4920fbfd0bce5ae66b9590007be01464eddf7d464de1a`,
with 26 members (25 payload members plus its manifest). Independent isolated
debug/release replay reproduced the tests and source hashes with no actionable
finding in this safe-model scope. The experiment changed no production code,
source schema or contract.

The scratch crate is dependency-free safe Rust with `forbid(unsafe_code)`. One mutex
protects fixed arrays: **8 owners, 8 objects, 8 handles, 16 ordinary receipts and
8 separate cleanup receipts**. Its inline `Ledger` occupies **7,320 bytes** on this
macOS arm64 build. There is no payload buffer, allocation per request, interval tree
or per-page write set in the ledger. The mutex implementation, caller-held Arc,
thread stacks and reference-model event vectors are outside that inline measurement;
this is neither a process-wide memory cap nor a production C12 budget.

Each reservation validates capabilities, binding, arithmetic, role caps, slots and
the entire W/E/S/M vector using local copies before publishing any state. Refusal
changes neither balance nor dispatch state. A returned receipt represents credit
already burned before a hypothetical external call. W charges every requested write,
including repeats; E retains each object's maximum reserved end through failure,
shrink and unlink; S retains mapping capacity; M prepays close and mapping cleanup.
The mapping-page policy is immutable per owner, validated at registration, and cannot
be selected anew on each map. The caller must still qualify the real OS/VFS policy.

Unlinked old WAL objects retain their extent while old handles exist. A new object
at the same role cannot reuse their retained capacity. Only an explicit idle actor
barrier, confirmed unlink and zero handles/flights permit logical retirement.
Generation counters retire rather than wrap. Whole-request native failure never
refunds credit or HWM and blocks further allocating work pending reconciliation.
Retiring a logical reservation makes no claim that APFS released physical blocks.

Owner/object/handle/receipt keys bind a **process-local nonwrapping ledger instance**
and slot generation; grants add a nonwrapping sequence. Different instances cannot
alias within this process. Tokens have no serialization or reconstruction API.
These are not restart credentials: a new process needs store-bound durable owner
identity, lock and reconciliation, or must reject old tokens. The model does not
implement a process-global registry across distinct ledger instances.

Cleanup receipts cannot be consumed by normal work. Partial opens with native
methods retain a wrapper capability and prepaid close; failed cleanup, unsafe open
results, close panic or false success quarantine the owner. Quarantine is permanent
in this model, finite, and refuses further owner allocation when its eight slots are
held. This favors refusal over speculative recovery. The close runner invokes its
external function outside the mutex while holding Closing; completing the last file
callback cannot retire the owner before the outer function returns. Confirmed return,
zero handles/flights, settled outcome and the explicit barrier precede retirement.

An uncertain-outcome token binds exact store/project/base generation/revision,
publication key and target. The verifier runs outside the mutex and its result is
revalidated against the same active grant after reacquisition. Unknown/mismatched
results and a slow result for an already regranted operation cannot release credit.
Committed evidence must name precisely base generation + 1 and the target revision.
The model owns no selected source root, record bytes or lease. Its synthetic verifier
is a trusted witness, not proof of authenticated durable SQLite status.

The pinned Rust **1.99.1** run passed rustfmt, all-target Clippy with warnings denied,
and debug/release tests: **23 protocol tests plus one numeric exhaustion test**.
The independent sequential reference recomputes sums and maxima from an append-only
accepted-event log rather than calling ledger debit code: **4,096 seeded operations,
4,055 accepted, 41 refused**. Sixty-four concurrent three-thread trials match at least
one of all six reference serial orders; the retained release sample observed one
winner pattern, so this is not exhaustive scheduler coverage. Additional controlled
interleavings cover in-flight barriers, simultaneous eight-thread budget contention,
last callback versus outer return and stale source verification versus regrant.
Tests cover cross-instance/slot/receipt ABA, each source-binding field, cross-owner
handles, overflow, full normal receipt pool, exhausted grant cleanup, partial open,
failed close, finite quarantine and a closed owner retaining an uncertain outcome.
The expected panic-hook message in the logs comes from a caught mock outer panic.
No native panic or FFI unwind behavior is exercised.

One local optimized **synthetic request-work** sample used 100 fresh/edit rounds:

| Work | Requests per round | Requested W per round | Mean protocol time | Time/request |
| --- | ---: | ---: | ---: | ---: |
| Fresh-shaped | 13,244 | 36,016,968 B | 796,569 ns | 60 ns |
| Edit-shaped | 187 | 488,968 B | 11,320 ns | 60 ns |

An eight-thread same-offset reserve/complete worker handled **80,000 requests in
11,454,333 ns**, or **143 ns/request aggregate wall time**. Separate worker peak RSS
was **1,736,704 B** for fresh/edit and **1,884,160 B** for contention (`wait4` macOS
byte units). These requests use integer lengths and no payload copying/native I/O;
they are not SQLite publication latency, an overhead comparison, per-thread tail
latency or device qualification. The fresh/edit shape is declared in `inputs.json`:
4,471/63 WAL frames and 4,296/56 main pages. Raw timing, commands and RSS are retained.
No lower-level specialization is justified by this single model sample.

Source authority SHA-256:

| Input | SHA-256 |
| --- | --- |
| `src/lib.rs` | `cd9114e46c881f149ab2df5a7ee885a6dd56ba9039761ec503d7c5bb20552803` |
| `tests/proof.rs` | `fcc49ca1b5dc73653405e0a845d46954f82857924ffd40f4fbc01d7b6e1bda9b` |
| `src/main.rs` | `f891f7d5eda4deb189e3569e9a176992a4b0719b534da4aebc01b737e2541f59` |
| `inputs.json` | `54c44f64e44d580875b7ce024372d3e30c71b7621aded4c1aba7e4434574f868` |
| `requirements.txt` | `e71c508fa5d5e82d1da4dcbf0491c7334ff7a9541ebfa44be4802e3c5cc072f2` |
| Release binary (reconstructible, not archived) | `d7aa1090f48897db7f2e5fad7e66e0709b10ee90ec92b485d5a57da62a34d580` |

Reproduce from the repository root with
`python3 tmp/source-request-credit-ledger/run.py`; the archive retains the same
relative scratch layout. The runner checks before benchmarking, uses locked offline
Cargo, captures rustc/OS/tool identities and hashes exact source. Selected archive
members are byte-verified against its manifest before owned `target/` removal.
Repository documentation checks and whitespace validation also pass.

The next integration gate must supply the missing trusted facts: a real strong
registration root and OS lock that survive leases/quarantine, reviewed native handle
storage/alignment and panic barriers, actual file identity and existing-extent
baseline installation, exact callback/control routing, and authenticated autocommit,
close and keyed durable outcome witnesses. Existing-store zero-E bootstrap and
crash reconciliation are not implemented here. A caller can lie to the mock native
result or source verifier, or drop the entire ledger; this safe state machine cannot
keep native pointers, pins or locks alive for that caller. Promote these semantics
and reference cases only after those interfaces are designed; do not treat this
scratch API as a ready production SDK. APFS growth, the 8 GiB physical floor,
external disk consumers, process kill, power loss and durable regrant remain separate
blocked gates. No native callbacks, VFS registration, restart or disk fault was tested.
