# Selected Cargo/Rustc for private-cache preparation — 3 November 2026

The opt-in F4 native A/B fixture now selects its Rust tool root before
preparing the private Cargo home. It passes that root's exact `bin/cargo` and
`bin/rustc` paths to the metadata preparer, uses the same paths for the
compiler version and cache-reuse checks, and asserts that they equal the
selected compiler root used by the subsequent build. The preparer already
hashes both executable files before and after metadata and records their
identities in its candidate manifest. The existing A/B fixture rechecks those
bytes after compiler execution.

The previous clone consumer ran the private-cache metadata step with installed
Cargo/Rustc, even though the following compile used cloned Cargo/Rustc. This
change removes that particular selection mismatch without changing the
unready candidate format or source admission semantics.

On one Apple-silicon Mac, two opt-in qualifications passed with Rust 1.99.1,
Python 3.11 and locked offline Cargo:

| Selected roots | Private-cache preparation | A fresh-output build | B and retained A |
| --- | ---: | ---: | --- |
| Installed Rust/Xcode | 1,264 ms | 3,924 ms | Invalid B failed its game compile; A reloaded and ran |
| Exact-inventory APFS Rust/linker/SDK clones | 1,726 ms | 5,620 ms | Invalid B failed its game compile; A reloaded and ran |

Each A run reconciled five successful compiler receipts and three probes;
each B run reconciled three successful dependency compiles, one failed game
compile and three probes. Private-cache reuse verification completed before
both A and B in each run (installed: 151/183 ms; cloned: 160/196 ms).
The two runs used the same 2,923-file, 67,792,387-byte private dependency
root digest. These are uncontrolled correctness-run timings, not a speed
comparison or tail-latency result.

The clone's expected Rust, linker and SDK inventories came from local
candidate digests, not a reviewed release manifest. Xcode discovery and
some support reads occur before the metadata sandbox, and that sandbox still
grants broad installed system roots and global file metadata. The clones
remain writable by the local owner, and the compiler sandbox still has
unclosed host-runtime and implicit-read boundaries. This proof does not
establish a no-writer lifecycle, authenticated release input selection, a
ready compiler adapter or `BuildRevision`; all candidate reports remain
`buildReady:false`.
