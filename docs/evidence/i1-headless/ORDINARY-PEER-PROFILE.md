# Finite ordinary headless peer membership — 3 November 2026

The ordinary `HeadlessHost` now offers explicit local and in-memory-replica
constructors with `max_registered_peers`. This is a selected finite
**controlled-peer membership** profile for one host/MatchId lifetime. It
counts distinct registered `PeerId` rows, including rows for a disconnected
peer; this first host has no retirement operation. The existing unrestricted
constructors and direct `Authority` API remain prototype paths. The caller
chooses the finite count for its game rather than inheriting a universal
two-player engine ceiling.

An oversized initial authority refuses before replica-baseline construction.
At capacity, a fresh ID returns `HostAdmissionError::PeerLimit` with the
registered and allowed counts before authority registration, queue growth or
sequence admission. A replicated same-ID reconnect still returns
`RequiresBaseline` without changing its epoch; it does not free or consume a
slot. A local same-ID newer-epoch reconnect uses the authority's existing
pending/result reset semantics and consumes no new slot; a stale epoch
refuses. No separate peer registry or second state authority is introduced.

Two focused ordinary tests passed locally. A prepopulated two-peer authority
refuses a one-peer host profile. Another host admits peer 2, refuses seven
fresh IDs at its two-peer limit, rejects an unqualified peer-1 reconnect,
then accepts sequence 1 from each existing peer in the same tick. The one
replica publication carries both results and contiguous watermarks, and its
state matches the authority. The local test first queues an old-epoch timed
action, then accepts a newer same-ID epoch, refuses the repeated epoch, old
input and a new ID, and admits sequence 1 under the new epoch. The whole
`thrum-core` and repository gates are recorded by the
normal `RUSTC_WRAPPER= ./dev check` run.

The added registration path checks one stored count and the existing sorted
peer directory; refusal allocates no new peer queue. There is one optional
`usize` in the host and no per-tick check. This is a cost model, not a measured
latency, allocator or RSS result. The profile bounds controlled membership
rows only. It does not bound arbitrary rule payloads, network sockets,
spectators, N1 projection/queue bytes, remote acknowledgements or retained
world snapshots. A replicated reconnect/retirement protocol must atomically
cover authority history, replica progress, epoch and baseline delivery before
old rows can be reclaimed. Real four-player transport and stalled-peer
qualification remain open.
