# APFS tool-root retention feasibility

3 November 2026. Scoped T2 research for the first CPU-only macOS ARM64
development builder. The governing [prepared-environment design](../T2-PREPARED-NATIVE-ENVIRONMENT.md)
and [release trust policy](../T2-NATIVE-RELEASE-TRUST.md) still require a reviewed
release manifest, a closed compiler/system-read set and retained exact inputs
before a target-ready build. This note tests only the cost and basic behavior
of one candidate retention mechanism.

## Local experiment

The host used APFS, macOS 26.6.2, Xcode 27.2 and the installed Rust 1.99.1
ARM64 Apple toolchain. An ignored experiment under `tmp/native-tool-root-clone/`
called `clonefile(2)` directly for each regular file into new directories on
the same volume, recreated each relative symlink, and created directories.
It did not use `cp -c`, which may fall back to a physical copy. The
[existing descriptor-bound inventory](../../../scripts/inventory_headless_tool_roots.py)
then checked each clone and its installed source against the previously
recorded [tool-root digest](../../evidence/i2a/F4-TOOL-ROOT-INVENTORY.md).
The experiment removed each clone in a `finally` block after its checks.

| Root | Regular files | Symlinks | Logical bytes | Clone time | Observed free-space change |
| --- | ---: | ---: | ---: | ---: | ---: |
| Rust | 394 | 0 | 1,030,074,151 | 0.056 s | 757,760 B |
| Xcode linker toolchain | 5,730 | 83 | 2,883,809,896 | 1.240 s | — |
| macOS SDK | 33,507 | 8,190 | 949,417,470 | 7.158 s | — |
| Xcode linker and SDK combined | 39,237 | 8,273 | 3,833,227,366 | 8.398 s | 52,809,728 B |

The free-space figures are before/after `statvfs` observations, not precise
per-root physical allocation. APFS volume activity and metadata allocation
can affect them. The Rust run and Xcode run were separate; their times should
not be read as a controlled combined-latency benchmark. All three cloned
roots reproduced the source inventory digests. The cloned `rustc` and Cargo
responded to `-vV`; cloned Clang compiled a small C file with `-isysroot`
pointing at the cloned SDK. A separate clone of a 143,609,472-byte Rust
`libLLVM.dylib` took 0.000725 s, used 8,192 B of observed free space, had a
different inode, and a one-byte change to the clone did not change the source
hash. These are local behavior checks, not durability or adversarial-race
proofs.

Apple describes APFS clones as same-volume copy-on-write files in
[About Apple File System](https://developer.apple.com/documentation/foundation/about-apple-file-system).
The local `clonefile(2)` manual says cloning is atomic **per file**, requires
an absent destination and can fail across volumes or on an unsupported
filesystem. It does not provide an atomic snapshot of a whole directory tree.

## Selection and readiness consequence

Explicit `clonefile` is a feasible cold-preparation candidate for these
installed roots: it retained identical inventories with little observed
space change and allowed direct compiler use from the cloned paths. It is not
yet an engine-owned immutable selection. A concurrent installer or writer
could change an installed tree while files are cloned one at a time; source
and clone inventories before/after cannot establish that every cloned byte
came from one instant. A same-user writer could also alter the clone after
selection. File permissions alone would not make it immutable against that
writer. The first selected release job must create/hold an owned root under
an explicit no-writer lifecycle, verify its entire inventory against the
reviewed manifest, and make the compiler sandbox read only from that retained
root. It must include selected Xcode support files outside the linker/SDK
roots, and prove refusal of missing, changed or unexpected inputs. The
selected host-runtime/system reads remain a separate open boundary.

Measure the complete cold selection and later edit-to-ready path before
claiming a latency win: clone metadata, inventory reads, Cargo-cache
integrity, release matching, sandbox startup, compile, retained-output sync
and process RSS. The clone step belongs to cold release/environment
preparation; it adds no world-update or multiplayer hot-path work. The
current adapter continues to recheck its mutable selected inputs and reports
`buildReady:false`.
