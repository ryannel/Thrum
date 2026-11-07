# N1 rejected-group baseline credit oracle — 2026-11-03

## Question and boundary

Can a rejected StateGroup block cumulative application ACK while a maximum
256 KiB replacement baseline completes through two bounded chunk credits,
without calling the rejected state admitted or dropping its mandatory IDs?

`crates/thrum-core/tests/n1_baseline_ack_oracle.rs` is a private, no-socket
sent-frame ledger model. It uses the selected 64-byte envelope, 120-byte
BaselineBegin, 12-byte chunk prefix, 8-byte BaselineEnd and eight 32 KiB
chunk payloads. It is not N1 production storage, transport, codec, baseline
validation, multi-group replica or a measured four-peer workload.

## Observed assertions

- Sequence 1 is an admitted group. Sequence 2 is rejected and remains in the
  exact frame ledger with result 9 and event 90. A cumulative admission ACK
  naming unsent output, sequence 2 or any later baseline frame is refused
  without releasing a mandatory ID.
- BaselineBegin receives sequence 3. Two chunks occupy both credits; a third
  submission is refused without consuming a sequence. Independent
  BaselineChunkAck for the first chunk frees only its frame bytes and credit.
  The eight chunks then stage with sequences 4–11 and BaselineEnd takes 12.
  Wrong-baseline, wrong-index and duplicate chunk ACKs free nothing.
- The peak retained baseline-frame bytes are **65,872** (Begin plus two
  `64 + 12 + 32,768` byte chunk frames), below the 96 KiB reserved sublimit.
  The rejected ordinary frame adds 8,000 bytes. The final baseline ledger
  retains ten entries (Begin, eight chunk sequence tombstones, End), within
  the 16-entry reserve. All counts include the rejected ordinary entry.
- Wrong BaselineAck leaves both mandatory identities retained. Valid
  BaselineAck retires rejected sequence 2 as **superseded**, distinct from
  admitted sequence 1, and releases only baseline-included result/event IDs.
  It does not reset application sequencing or admit the rejected group.
- A separate failed replacement omits a required result. Its BaselineAck is
  refused, no mandatory ID is released, and explicit close clears framed
  storage while leaving those outcomes marked uncertain.

Command: `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-core --test n1_baseline_ack_oracle`.
Both tests passed locally on the named development Mac. This is deterministic
arithmetic and state-transition proof, not a timing or RSS measurement.

## Remaining gate

The production N1 owner must enforce simultaneous unsent, sent, sync,
transport, baseline-image and journal limits; bind exact result/event sets,
baseline identity, generation and epoch to validated wire bytes; handle
uncertain socket writes and timeout; and drive one same-tick multi-group
replica. The four-peer mandatory burst and whole-path CPU/RSS remain unmeasured.
