# N1 multi-group replica admission oracle — 2026-11-03

The first network profile permits different baseline-bound partitions to
publish at the same `commitTick` and at different tolerated sample ages. The
I1 `Replica` has one global last tick, so admitting same-tick groups by merely
relaxing its tick comparison would misstate the age of every other partition.

`crates/thrum-core/tests/n1_multi_group_replica_oracle.rs` models one
composite owner with a partition table, per-partition generation and sample
tick, one current entity index and a separate baseline-bound assignment that
survives visibility loss. Its validation builds scratch
state and publishes the partition and both entity maps together only after all
checks pass. It is a private model using tree clones for clarity, **not** the
selected production representation or a CPU/allocation measurement.

The first test admits two partition groups at tick 1, advances one to tick 3
while the other remains at tick 1, and reads `(value, sampleTick)` for each
entity. It refuses stale same-partition time, a cross-partition duplicate
spawn and an unknown despawn without changing either owner. A full generation
2 partition replacement at tick 4 advances only its own sample age.

The second test refuses a generation transition without complete partition
state, a replacement that captures another partition's entity, and a skipped
generation. Each refusal leaves the entire composite replica unchanged.
A third test removes an entity from visible partition 20, refuses its attempted
reassignment to partition 10 without a replacement baseline, then permits it
to re-enter partition 20.
A fourth test cycles 254 fresh wire IDs through one partition while only two
entities remain visible. It fills the first-profile 256-entry persistent
assignment table, refuses ID 257 without changing replica state, and permits
another partition to advance. The actual N1 producer must trigger a
replacement baseline or close at this pressure point; the private model has
neither lifecycle.

Command: `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-core --test n1_multi_group_replica_oracle`.
All four tests passed locally. The model does not parse the wire schema, validate
dependency closure or lifecycle generations, carry results/events, apply a
whole replacement baseline, or establish bounded allocation and throughput.
Those remain production N1 gates; this proof selects the need for explicit
per-partition admission metadata in the one replica owner.
