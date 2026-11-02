# Candidate native tool-root inventory

2 November 2026. This is a local macOS ARM64 preparation observation for the
[first headless builder design](../../design/F4-FIRST-HEADLESS-BUILD.md). It
identifies installed bytes; it is neither an authenticated SDK release nor a
`BuildRevision`.

The [inventory tool](../../../scripts/inventory_headless_tool_roots.py) uses
Python 3.11, sorted relative paths and a `thrum.tool-root.v1` domain prefix.
Each directory contributes its path/type; each regular file contributes path,
type, length and a streamed SHA-256; each symlink contributes path, type and
raw target bytes. Symlinks resolving outside their selected root refuse.
The POSIX walker opens directories and files relative to held descriptors with
no-follow flags. It checks entry identity before and after reading, then
resolves the recorded links to confirm containment. Five small fixtures check
content and directory changes, contained and escaping aliases, duplicate
roots, a regular file replaced by an escaping symlink, and a replacement FIFO
that must refuse without waiting for a writer.

The local invocation selected the installed Rust 1.99.1 ARM64 Apple toolchain,
Xcode 27.2's `XcodeDefault.xctoolchain`, and its macOS SDK. It wrote a candidate
JSON manifest under ignored `tmp/f4-toolchain-inventory/`:

```sh
mkdir -p tmp/f4-toolchain-inventory
python3.11 scripts/inventory_headless_tool_roots.py \
  --rust-toolchain /Users/ryannel/.rustup/toolchains/1.99.1-aarch64-apple-darwin \
  --linker-toolchain /Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain \
  --sdk /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk \
  --output "$PWD/tmp/f4-toolchain-inventory/candidate.json"
```

| Root | Files | Symlinks | Logical bytes | Candidate SHA-256 |
| --- | ---: | ---: | ---: | --- |
| Rust toolchain | 394 | 0 | 1,030,074,151 | `00d56ad367ace4cdbae132ce52f66a0cb49ef7f7b432dc1fd493421b947ae368` |
| Xcode linker toolchain | 5,730 | 83 | 2,883,809,896 | `cb8dbb37004fbbc31eb5de9401c5c57f3019831d9c440add9675ed1e9b9d8c3d` |
| macOS SDK | 33,507 | 8,190 | 949,417,470 | `e92b280633607b7d71d0807c13d18d33255665bbb5fbf89a37aec9920bb247f0` |

The first combined run took 12.7 seconds; the final descriptor-bound run took
11.0 seconds on this host, with the same three digests. Its working buffer is 1 MiB
per file stream; sorting paths and resolving symlinks adds memory and metadata
work proportional to tree size. This belongs to selected release/toolchain
preparation, not every edit or world update. A single pass cannot prove an
instantaneous snapshot against a concurrent installer. The candidate roots
are not yet connected to the compiler adapter, signed/release selected, or
checked against all system libraries the selected commands read. The native
A/B build remains a qualification, not a ready artifact.
