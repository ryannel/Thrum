# Exact-inventory APFS tool-root clone candidate

3 November 2026. This qualifies one local macOS ARM64 **preparation helper**
against [T2's prepared-environment design](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md).
It is not a release-selected immutable environment or a ready build. The
[earlier feasibility probe](../../design/research/T2-MACOS-APFS-TOOL-ROOT-RETENTION-2026-11-03.md)
measured raw per-file clone cost separately.

## Selected mechanism and local proof

[`prepare_macos_tool_root_clone.py`](../../../scripts/prepare_macos_tool_root_clone.py)
accepts one installed source root, an absent destination under a caller-owned
parent on the same volume, and an independently supplied exact inventory. It
opens source directories and regular files with no-follow flags, uses
`fclonefileat` on held regular-file descriptors, recreates relative symlinks,
and refuses unsupported entries, aliases at root, source/destination overlap,
cross-volume use and counts or bytes above the selected inventory. It has no
physical-copy fallback. It streams the complete clone with the existing
descriptor-bound inventory and returns only when the entire descriptor
matches. A failed preparation removes its newly created root while its
directory identity remains owned; a changed root identity requires the caller
to quarantine the parent rather than delete an unknown replacement.

Seven small macOS fixtures passed: exact content and relative-link copy,
clone-private mutation, wrong digest cleanup, existing destination
preservation, source alias and absolute-link refusal, extra-file refusal,
changed source during cloning, and FIFO refusal without opening it. The test
command was `python3.11 scripts/test_prepare_macos_tool_root_clone.py`.

The local selected inputs were Rust 1.99.1 ARM64 Apple and Xcode 27.2's linker
toolchain and macOS SDK. The helper compared each clone with the previously
recorded [candidate inventory](F4-TOOL-ROOT-INVENTORY.md), not a reviewed
release manifest. One full run produced:

| Root | Inventory digest prefix | Clone plus full clone-byte inventory |
| --- | --- | ---: |
| Rust | `00d56ad367ac` | 0.626 s |
| Xcode linker | `cb8dbb37004f` | 8.788 s |
| macOS SDK | `e92b28063360` | 14.604 s |

These elapsed times include per-entry creation and a complete hash of every
cloned file; they are one uncontrolled cold run, not paired benchmarks or a
warm edit-to-ready estimate. A second run retained all three clones at once,
ran cloned `rustc -vV` and `cargo -vV`, and compiled a small C object through
cloned `usr/bin/clang` with `-isysroot` pointing at the cloned SDK. It then
removed only those owned clones and probe files. No compiler sandbox or
admitted game source consumed this retained selection yet.

## Boundary and next proof

The exact clone digest can reject a mixed or changed source tree even though
file clones are atomic only one at a time: the selected final bytes must match
the independently reviewed whole-root digest. The current caller supplied a
local candidate digest, so that review has **not** happened. A successful
helper return also does not prevent another same-user process from writing to
the clone afterward, and the helper does not sync a durable release root.
The selected Xcode support files outside these roots and macOS runtime reads
are absent. The current native A/B build still reads installed roots and
continues to report `buildReady:false`.

Next, the release preparer must bind the expected digest to a reviewed
exact-hash manifest, retain an owned root under an explicit no-writer
lifecycle, include selected support files, and generate the sandbox's read
grants and tool paths from that same retained selection. Mutations before
launch and while compiling must refuse or be prevented; no new source, cook,
world or multiplayer authority should be created by this helper.
