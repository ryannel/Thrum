# macOS preferences grant reduction — 3 November 2026

The bounded native compiler sandbox previously allowed data and metadata
reads across `/Library/Preferences`. The profile now omits that subtree and
its ancestor grants. An existing system preference file is readable outside
the sandbox; explicit `head` and `stat` controls require denial inside it.
The installed-root admitted A/B build then passed: five successful A compiles
and three probes, one ordinary failed B game compile after three dependency
compiles and three probes, and a retained A that reloaded and ran. The A
fresh-output compiler stage took 3,859 ms in one uncontrolled local run.

An earlier trial also removed `/private/etc`. The denial controls passed, but
the compiler supervisor reported Cargo exit 1 before the A compile completed.
The supervisor previously discarded bounded stderr on such an unexpected
status. It now retains and hashes bounded stdout/stderr with `buildReady:false`.
Replaying the refusal identified LibreSSL's attempted read of
`/private/etc/ssl/openssl.cnf` (`Operation not permitted`). The compiler
sandbox now grants **that regular file only**, includes it in selected-file
hash checks before and after builds, and denies data/metadata access to the
existing `/private/etc/hosts`. A positive sandbox control reads the selected
config bytes; the system positive control uses an existing file under
`/System/Library`. With this exact grant, the complete installed-root A/B
qualification passed again: five A compiles and three probes, failed B game
compile after three dependency compiles and three probes, and retained A
reload/run. The last uncontrolled A compiler stage took 3,950 ms.

This is a local sandbox policy reduction, not a complete read trace or a
performance comparison. This one observed LibreSSL read is bound to local
selected bytes but not to reviewed OS provenance or an immutable host policy.
Other host runtime reads,
immutable tool selection, release review and ready build remain open.
