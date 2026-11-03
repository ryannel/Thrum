# F3 admission with a live external F4 input owner — 3 November 2026

The ordinary Unix regression
`tests::external_same_volume_input_owner_survives_source_admission` in
[thrum-cook](../../../crates/thrum-cook/src/lib.rs) proves one root-placement
interaction from the [provisional T2 account design](../../design/research/T2-SAME-VOLUME-JOB-ACCOUNT-2026-11-03.md).
It uses existing public F3/F4 APIs without changing their policies. The local
run used baseline `9489a37`, macOS 26.6.2 build 25G83 arm64 and Rust 1.99.1.

## Fixture and result

A is a four-file accepted project with a 12 MiB opaque master and a small
valid scene. Its provisional stage closes before F4 starts. The selected
test policy is 1 GiB source main, 64 MiB WAL, **16 MiB managed temporary**
and zero desired free headroom; F4's per-tree target is also 16 MiB. These
values distinguish placement effects in this fixture and select no T2 default.

The harness creates a separate owned sibling root, canonicalizes both roots,
checks that neither contains the other and that both have the same Unix
`st_dev`. The external parent and job root have checked `0700` modes. F4
materializes the exact A pin into the new absolute `job-a/input` directory;
a fixture registration sentinel lives beside that input directory.

| Placement while the external A owner remains live | F3 result for tiny B scene edit |
| --- | --- |
| Additional complete A input in ordinary project subtree | Inventory refuses an undeclared file |
| Project symlink to the external job root | Inventory refuses the unsupported symlink entry |
| Additional complete A input under project `.engine` | Typed `TemporaryPreflight` refusal |
| External A input alone | Complete inventory, stage, validation and B admission succeed |

The `.engine` control reports 12,673,024 bytes of existing charge and
12,644,352 bytes of projected complete B stage against the 16,777,216-byte
target. The external input contains 12,583,454 copied bytes. Its complete
owner stays live through all controls, successful B capture/admission and
source-store reopen. Thus the same source inventory and managed-temporary
scanner remain strict about project-owned paths, while external placement
avoids their visibility without a scanner exemption.

Every A input file is SHA-256 compared with streamed verified A-pin bytes
before the edit and after B admission. A's old pin still returns the entire
exact A tree. After closing the pin and store, reopen recovers exact A and B
retry outcomes and B's selected generation/revision. A historical pin again
returns the entire A tree; B's pin returns the expected complete B tree with
only the scene hash changed. The original F4 A tree still matches throughout.
Checked F4 close removes its input directory and preserves the sibling
registration sentinel. Fixture owners remove their own remaining roots.

## Reproduction and limits

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-cook external_same_volume_input_owner_survives_source_admission -- --nocapture
RUSTC_WRAPPER= ./dev check
```

The focused test and full repository gate passed locally. The test is
enabled on Unix; Windows root/domain identity remains unqualified. Its
operations are sequential with overlapping owners. It does not race partial
construction or F4 close against a scan, exercise native space faults,
enforce root placement in the F4 API, implement durable T2 registration or
aggregate claims, supervise a compiler, publish a ready build, or measure
physical allocation. Same-user deliberate path mutation remains outside this
fixture. The sentinel tests cleanup scope, not a durable job registry.
