# First pinned headless build declaration

2 November 2026. **Implemented bounded metadata gate, not a ready build.**

`thrum-cook::read_pinned_build_declaration` reads `build/headless.json` through
the accepted source pin. It enforces a 64 KiB metadata bound, strict JSON
syntax and unique keys, the exact version-one field set, portable paths present
in the authenticated manifest, a bounded Cargo package name, and the entry
scene identity of a cooked product from the same accepted root. It returns
typed paths and identities for a future T2 compiler adapter. It does not read
the mutable checkout or execute/parse Cargo.

The production-store fixture admits source A with a cookable scene and valid
build declaration. It changes the checkout declaration to duplicate a key,
then still reads A's exact pinned declaration. It admits the changed source B:
F3 accepts its exact bytes and F4 cooks its scene, while the build declaration
refuses. Presenting A's product to B's pin also refuses before parsing. It then admits C with a syntactically valid declaration naming a
missing toolchain file; F4 reports the missing input. The held A declaration
continues to name A. This separates accepted source, cooked content and
buildability without altering a running authority or creating a false ready
build.

The declaration is one bounded JSON record; parsing and membership checks run
once per build attempt outside simulation callbacks. No elapsed-time, RSS or
compiled-binary cost is claimed. The test's Cargo lockfile and toolchain file
are membership fixtures only, not qualified dependency/toolchain declarations.
The [selected build profile](../../design/F4-FIRST-HEADLESS-BUILD.md) specifies
the remaining compiler isolation, complete dependency closure, retained binary
and product bytes, trusted target manifest, `BuildRevision`, fresh run and
private save proof. The parent [build-plan item](../../plans/BRIEF.md) stays open.
