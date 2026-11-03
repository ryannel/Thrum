# Held macOS tool-root candidate

3 November 2026. macOS 26.6.2 (25G83), ARM64, Python 3.11. This is a local
unready proof for **one** APFS tool-root clone under the selected trusted-local-
user development profile. The implementation is
[`hold_macos_tool_root_clone.py`](../../../scripts/hold_macos_tool_root_clone.py)
and its six fixtures are in
[`test_hold_macos_tool_root_clone.py`](../../../scripts/test_hold_macos_tool_root_clone.py).

The cold preparer acquires an exclusive advisory lock on a permanent,
user-immutable private lock inode named by the expected content digest. It
uses the existing exact
inventory clone helper, removes write and special mode bits while preserving
read/execute bits, sets owner-clearable `UF_IMMUTABLE` on regular files and
directories from leaves to root, fsyncs them, verifies the complete inventory
and seal, and publishes a canonical sidecar last. A failed seal or record
publication leaves an unusable digest name; it is not overwritten or cleaned
up by this candidate. The sidecar has `buildReady:false`. Cold acquisition
takes a shared lock, checks sidecar identity and bytes, the whole seal and the
whole byte inventory, and returns held root and lock descriptors. A warm
`assert_live` checks the root pathname, descriptor identity and root flag.
Published roots are append-only; this helper exposes no unseal/delete path.

Six fixtures passed with
`python3.11 scripts/test_hold_macos_tool_root_clone.py`: a nested executable
and relative symlink still run from a sealed clone; append writes fail; a
modified but resealed sidecar refuses; interrupted child sealing and
interrupted sidecar sealing refuse cold selection and overwrite; two separate
reader processes hold shared leases while exclusive preparation refuses; and
a directly owned worker continues to read the root after a separate lease
owner process exits. The worker is controlled by the test process and reaped
with a bounded wait. `RUSTC_WRAPPER= ./dev check` passed after the fixture
additions. These tests do not launch the real Cargo/compiler A/B consumer with
this lease.

Three local 1,001-file samples (256,030 logical bytes) separated cold and warm
costs. Cold clone/seal/full-hash preparation took 276.0, 264.9 and 282.7 ms;
cold shared acquisition with full seal and byte verification took 44.4, 42.2
and 48.2 ms; 100,000 held-root `assert_live` checks averaged 3.14, 3.13 and
3.54 microseconds each. These are uncontrolled small-fixture samples, not
compiler throughput or an edit-to-ready claim.

One additional owned probe used each full local candidate root. A fresh
inventory of the installed source preceded preparation; the timed preparation
then cloned, sealed and fsynced every file/directory, rehashed the sealed
root, and published the sidecar. Fresh shared acquisition rechecked every
seal and byte. Paths were resolved before selection (the Xcode SDK name is an
alias), and only the probe-owned clone was unsealed and removed afterward.

| Root | Files / directories | Logical bytes | Source inventory | Cold preparation | Cold shared acquisition | 10,000 warm checks, mean each |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Rust 1.99.1 | 394 / 30 | 1,030,074,151 | 0.583 s | 1.033 s | 0.412 s | 3.44 µs |
| Xcode 27.2 linker | 5,730 / 2,208 | 2,883,809,896 | 6.403 s | 13.761 s | 5.365 s | 3.88 µs |
| macOS 27.2 SDK | 33,507 / 11,068 | 949,417,470 | 6.772 s | 19.687 s | 3.875 s | 3.48 µs |

Cloned Rustc `--version` and Clang `--version` also ran; the SDK has no
executable in this probe. The three source digests matched the earlier local
candidate prefixes `00d56ad367ac`, `cb8dbb37004f` and `e92b28063360`.
These are single uncontrolled runs, not paired installed/clone build timings
or power-loss tests. The Xcode pair alone took 33.45 seconds to prepare after
its source inventory and 9.24 seconds for a fresh full cold acquisition.
Physical allocation and concurrent-build overlap were not measured. Warm edit
work should reuse a held selection rather than run cold acquisition for every
edit.

The selected inventory is still caller supplied and unreviewed. No exact
reviewed release manifest, dependency/support/runtime selection, complete
compiler read closure, real Cargo consumer under the held lease, OS-owned
compiler-job containment, durable ready publication or `BuildRevision` follows.
Same-user deliberate flag clearing or path replacement remains outside this
development trust profile. Advisory locks coordinate cooperating Thrum jobs;
the append-only rule keeps a published root available after an owner exits,
but future garbage collection requires a separate child-lifetime proof.
Apple's [file flag guidance](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html)
and [flock manual](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html)
define the owner-clearable flag and advisory lock behavior used here.
