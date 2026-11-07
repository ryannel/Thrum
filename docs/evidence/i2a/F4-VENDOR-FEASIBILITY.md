# Full-workspace Cargo vendoring cost

2 November 2026. This local macOS ARM64 experiment compared a supported
index-free dependency layout with the first headless builder's prepared
private Cargo home. It used the mutable checkout and shared host cache as
inputs; it is a size/cost observation, not an admitted-source build or release
authentication.

The pinned Rust 1.99.1 Cargo ran `cargo vendor --locked --offline
--versioned-dirs --manifest-path Cargo.toml` into ignored task scratch. It
completed in 3.52 seconds and produced 229 versioned package directories,
15,442 regular files and 425 MB of disk use (`du -sh`). The generated
source-replacement configuration was kept only for inspection. The bulk tree
was removed after measurement. The existing [private headless cache
preparation](F4-PRIVATE-CARGO-CACHE.md) retains 2,923 files and 67,792,387
logical bytes after host filtering; its warm local preparation took about
1.2–1.6 seconds across the recorded runs. These byte measures use different
filesystem accounting and are directional, not a like-for-like disk benchmark.

The [Cargo vendor command](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html)
vendors all remote dependencies named by the workspace, then prints the
configuration needed to use them. Cargo's [source replacement
documentation](https://doc.rust-lang.org/cargo/reference/source-replacement.html)
says a vendored directory's `.cargo-checksum.json` protects against accidental
changes and is not a security mechanism. The whole-workspace vendor tree is
therefore an expensive default for this first headless profile and does not
solve release provenance by itself. A future selected-graph vendor recipe may
be measured separately if its equivalence to the admitted lockfile and package
closure can be proved.

The installed Rust toolchain is still a local candidate. The [rustup security
documentation](https://rust-lang.github.io/rustup/security.html) says downloads
use HTTPS but rustup does not validate download signatures. A ready native
adapter must select exact toolchain bytes under the engine's reviewed release
manifest or another explicit trust root; local rustup installation alone does
not establish that claim.
