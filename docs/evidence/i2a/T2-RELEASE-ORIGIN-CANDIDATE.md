# Installed native release origin candidate

2 November 2026. **Local provenance evidence, not a reviewed release manifest or ready build.**
This follows the selected [first native release trust policy](../../design/T2-NATIVE-RELEASE-TRUST.md).

## Rust 1.99.1 channel

The installed ARM64 toolchain reports `rustc 1.99.1 (58cf8f9c3 2026-10-13)`
and `cargo 1.99.1 (5f94df478 2026-08-27)`. I fetched
[`channel-rust-1.99.1.toml`](https://static.rust-lang.org/dist/channel-rust-1.99.1.toml)
and its [SHA-256 sidecar](https://static.rust-lang.org/dist/channel-rust-1.99.1.toml.sha256)
over HTTPS into isolated temporary storage. The downloaded manifest's SHA-256
matched the sidecar:

`fb152be80975c194aa5fc684b2aec3a6e14e2b855efe2faa4e5f8ae6e590f463`

Python 3.11 `tomllib` parsed the downloaded manifest and rustup's installed
`lib/rustlib/multirust-channel-manifest.toml`. For all nine entries in rustup's
installed `components` file, the selected component URL and SHA-256 matched
the official channel record. These were Cargo, Rustc, Clippy, rustfmt and the
ARM64 macOS Rust standard library, plus the installed Wasm, iOS device,
iOS simulator and Android standard libraries. Each selected URL was under
`https://static.rust-lang.org/dist/2026-10-15/`.

Rustup's installed channel TOML is not byte-identical to the downloaded file:
rustup rewrote fields while retaining the selected URL/hash pairs. This
semantic comparison avoids treating the rewritten file as the downloaded
manifest. That comparison alone does not prove installed file bytes;
the exact installed tree is separately identified by the
[cold root inventory](F4-TOOL-ROOT-INVENTORY.md) and matcher, and archive-to-tree
provenance still needs a controlled preparation job and review.

For the two compiler-driver executables, I also downloaded the official
`.tar.xz` archives and checked their SHA-256 against the verified channel
manifest before reading their `bin` members. The Cargo archive hash was
`685e563d94db1f003ffa5f5831439eec02569b9950b69867953c8213269c1eaf`;
its `cargo/bin/cargo` and the installed `bin/cargo` both hashed to
`6d576d8058f8ca643e8fce55b64ee3c614ba64c9da2e536bae843e6a2e1f3951`.
The Rustc archive hash was
`23948a0a9c162fd2952defb3fb0462c44e8fcfd1a4ab8a274a1e1bdb2cf810a3`;
its `rustc/bin/rustc` and the installed `bin/rustc` both hashed to
`1ba7bb8e273bd64125f4048d2ecce530e0a6470e565ab1b3aecf0b82b532f7b1`.
That initial comparison proves exact archive-to-installed identity for those
two executable files on this host. The following component pass covers their
packaged support files; other installed components and system libraries remain
separate.

The repeatable [component archive verifier](../../../scripts/verify_rust_component_archive.py)
extends that comparison to all regular payload files listed by each archive's
`manifest.in`, checks the installed component manifest matches, refuses extra
payload members and paths escaping the toolchain, and reports no build
readiness. On this Mac it matched 44 Cargo files (32,688,641 unpacked bytes)
and 43 Rustc files (279,894,598 unpacked bytes). Its local runs took about
0.54 and 2.60 seconds respectively, including archive rehash, one-pass
decompression and installed-file hashing. Eight synthetic fixtures check a
valid component, altered installed bytes, archive/sidecar changes, unexpected
payload membership or second component manifest, an installed parent alias and
parent swap outside the toolchain, a PAX metadata header rejected before
buffering, and a concatenated XZ stream. These are cold release-preparation
costs, not per-edit or runtime costs. Other
installed Rust components, system libraries, Xcode and the private Cargo
cache still lack the same origin-to-byte closure.

The host `rust-std` archive was then downloaded from the selected official
channel URL over HTTPS. Its SHA-256 was
`fe79d6ab5a0321de26463999d8c112b8ad3eafbcbcf822d66a2300b45dde1031`,
matching that channel record. This archive uses 61 GNU long-name records, each
at most 156 bytes. The verifier now accepts bounded long-name records and
continues to refuse other tar extensions; a new fixture covers both an
accepted long name and an oversized refusal. It matched all 59 installed host
standard-library payload files (152,233,278 unpacked bytes). This adds the
selected host standard library to the local component origin candidate;
compiler-read closure and a reviewed release manifest remain open.

## Selected Xcode

`xcodebuild -version` reported Xcode 27.2, build `27B52`, and `xcrun --sdk
macosx --show-sdk-version` reported SDK 27.2. The selected Clang is inside
`/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain`.
Outside the command sandbox, `codesign --verify --deep --strict` completed
successfully and reported that Xcode is valid on disk and satisfies its
designated requirement. `codesign -dv` reported identifier
`com.apple.dt.Xcode`, team `59GAB85EFG`, and an Apple signing chain.
`spctl --assess --type execute` accepted the app with source `Mac App Store`.
The same signature check inside the command sandbox reported a certificate
trust error, so the successful unsandboxed result is the relevant local check.

This supports an official installed-app origin candidate; it is not a fetched
Xcode installer receipt or an Apple-published exact content hash for the
selected linker, SDK and support files. The selected release manifest still
needs human review of the preparation environment, installer provenance,
dependency cache origins and all required compiler/system reads. The
[release-input matcher](F4-RELEASE-INPUT-MATCHER.md) continues to report
`buildReady:false`.
