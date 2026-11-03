# Narrower macOS compiler support grants — 3 November 2026

The earlier [metadata proof](F4-NARROW-METADATA.md) granted installed Xcode
`libxcrun.dylib` support files to the opt-in compiler sandbox. Direct Cargo,
Rustc and Clang selection made their need during **compilation** uncertain.
The sandbox profile now grants neither data reads nor metadata for those
installed support files. The test probes both operations on each existing
selected file and requires denial before building. The files remain identified
and byte-checked by the local selection because the private-cache metadata
preparation phase still uses installed host tools outside this compiler
sandbox.

Two separate opt-in A/B qualifications passed on macOS ARM64:

| Compiler roots | A | Invalid B and retained A |
| --- | --- | --- |
| Installed Rust/Xcode | 5 successful compiles, 3 probes; fresh-output build 3,621 ms | 3 dependency compiles, 1 failed game compile, 3 probes; A reloads and runs |
| Exact-inventory APFS Rust/linker/SDK clones | 5 successful compiles, 3 probes; fresh-output build 8,261 ms | Same failed-B shape; A reloads and runs |

The clone run used the previously recorded **local candidate** inventory
digests, not reviewed release inputs. The three roots were cloned and hashed
under owned ignored scratch, passed the selected-file and installed-file
controls, then that scratch was inspected and removed. These are uncontrolled
correctness runs; their time difference does not establish a performance
effect from removing support grants or using clones.

This closes one unnecessary **compiler sandbox** read exception for the
bounded fixture. It does not close the installed-tool metadata preparation,
Xcode support provenance, selected macOS runtime/system reads, concurrent
clone writes, complete compiler read tracing or a ready `BuildRevision`.
Apple documents that the file owner may clear `UF_IMMUTABLE` flags
([file-system flags](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemDetails/FileSystemDetails.html));
applying that flag or read-only permissions to same-owner clones alone would
not establish the required no-writer lifecycle.
