# Exact-inventory APFS tool-root clone candidate

3 November 2026. This qualifies one local macOS ARM64 **preparation helper**
against [T2's prepared-environment design](../../design/T2-PREPARED-NATIVE-ENVIRONMENT.md).
It is not a release-selected immutable environment or a ready build. The
[earlier feasibility probe](../../design/research/T2-MACOS-APFS-TOOL-ROOT-RETENTION-2026-11-03.md)
measured raw per-file clone cost separately.

## Selected mechanism and local proof

[`prepare_macos_tool_root_clone.py`](../../../scripts/prepare_macos_tool_root_clone.py)
accepts one installed source root, an absent destination under a mode-private
parent owned by the current user on the same volume,
and an independently supplied exact inventory. It
opens source directories and regular files with no-follow flags, uses
`fclonefileat` on held regular-file descriptors, recreates relative symlinks,
and refuses unsupported entries, aliases at root, source/destination overlap,
cross-volume use and counts or bytes above the selected inventory. It has no
physical-copy fallback. It streams the complete clone with the existing
descriptor-bound inventory and returns only when the entire descriptor
matches. A failed preparation removes its newly created root while its
directory identity remains owned; a changed root identity requires the caller
to quarantine the parent rather than delete an unknown replacement.

Fifteen small macOS fixtures passed: exact content and relative-link copy,
clone-private mutation, wrong digest cleanup, existing destination
preservation, source alias and absolute-link refusal, extra-file refusal,
changed source during cloning, and FIFO refusal without opening it. The test
command was `python3.11 scripts/test_prepare_macos_tool_root_clone.py`.
The added path fixtures refuse a `0755` destination parent and a `0777`
non-sticky ancestor before creating a clone. The helper checks the resolved
ancestor chain's POSIX owners and group/other write bits, allowing a sticky
ancestor. It queries each held directory for a macOS extended ACL, rejects
any parent ACL, and rejects an ancestor ACL with an allowing or unknown entry.
A real mode-`0700` parent and an ancestor with `everyone allow` ACLs both
refuse before clone creation; a deny-only ancestor remains usable, matching
this host's home-directory ACL. An ancestor-first regression also refuses when
an allowing ACL would be removed upon checking the private child. The helper
checks held-descriptor permissions again after each ACL query; a fixture
changes an ancestor to mode `0777` during that query and refuses. A synthetic
getter fault also refuses. The
Darwin `acl_get_fd_np`/`acl_get_entry` checks use the selected SDK constants;
[Apple's file-system security guidance](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html)
explains why ACLs must be considered alongside mode bits. The helper does not
hold the checked directory chain across clone creation or establish a
no-writer owner after return. The first profile's trusted same-user assumption
excludes deliberate same-user path replacement; other-user path replacement
is restricted by the checked permissions at selection, but this is not a
security audit of every filesystem or privileged actor.

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
admitted game source consumed that first probe's selection.

## Sandboxed admitted A/B consumer

An opt-in run then prepared the same three local exact-inventory clones under
ignored `tmp/t2-retained-ab/` and supplied all three absolute roots to the
existing `thrum-cook` A/B qualification through `THRUM_TEST_RETAINED_RUST_ROOT`,
`THRUM_TEST_RETAINED_LINKER_ROOT` and `THRUM_TEST_RETAINED_SDK_ROOT`. The Rust
test chose Cargo/Rustc and Clang/SDK from those paths for both builds, checked
cloned Cargo/Rustc hashes against the private-cache preparer's selected
executable hashes, and generated sandbox read grants from the selected clone
paths. Three small sandbox controls read a byte from cloned Rustc, Clang and
SDK `stdio.h`, while reads and metadata probes of their installed counterparts
failed. The Xcode support-file exceptions remained at their installed paths.
The receipt verifier now accepts an explicit linker/SDK pair and requires the
observed rustc environment and codegen linker flag to match it. One new
fixture accepts separate retained roots and refuses a wrong or half-selected
pair; its other 14 fixtures still pass.

The final opt-in A/B run passed on this Mac: A produced five successful rustc
compiles and three probes, a runnable retained candidate and a fresh-process
reload after deleting its materialized input and original Cargo output. B
produced three successful dependency compiles, one ordinary failed game
compile and three probes; A remained runnable. A's fresh-output compiler
stage took 4,262 ms, private-cache preparation 1,284 ms, and candidate
retention 207 ms in this uncontrolled run. Earlier cloned A runs took
10,171 and 4,297 ms, so these values are **not** a paired comparison with
the installed-root path or a warm latency claim. The clone-plus-inventory
preparation was measured separately above and is intended as cold work.

## Boundary and next proof

The exact clone digest can reject a mixed or changed source tree even though
file clones are atomic only one at a time: the selected final bytes must match
the independently reviewed whole-root digest. The current caller supplied a
local candidate digest, so that review has **not** happened. A successful
helper return also does not prevent another same-user process from writing to
the clone afterward, and the helper does not sync a durable release root.
The selected Xcode support files outside these roots and macOS runtime reads
are absent from the retained selection. The opt-in compiler can read cloned
roots under the sandbox, but the default A/B path still selects installed
roots and both paths report `buildReady:false`.

Next, the release preparer must bind the expected digest to a reviewed
exact-hash manifest, retain an owned root under an explicit no-writer
lifecycle, include selected support files, and generate the sandbox's read
grants and tool paths from that same retained selection, including the Cargo
metadata/preparation phase that still used installed executables in this
proof. Mutations before launch and while compiling must refuse or be
prevented; no new source, cook, world or multiplayer authority should be
created by this helper.
