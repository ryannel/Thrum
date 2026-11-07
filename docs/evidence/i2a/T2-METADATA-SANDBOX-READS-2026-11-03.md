# Private-cache metadata sandbox read reduction — 3 November 2026

The first-profile private Cargo cache preparer now generates a deny-default
macOS metadata sandbox from its selected source, private cache and Cargo tool
root. It grants file metadata inside the selected read subtrees and along
their path ancestors, and file data to the selected subtrees, named
system-library/cryptex roots, `/dev/null`,
`/dev/urandom` and the exact LibreSSL config file. It removes the previous
global metadata permission and broad `/System`, `/usr`, `/Library`,
`/Applications`, `/opt`, `/private/etc`, `/var/select` and `/dev` data grants.
The resolved materialized-source and private-cache paths are used in both the
sandbox parameters and the Cargo metadata command; an initial trial failed
because macOS resolved the `/var` alias to `/private/var` while Cargo still
received the alias.

A focused real `sandbox-exec` test reads selected source and tool files and
denies data and metadata for existing `/usr/bin/uname` and
`/private/etc/hosts`. Two opt-in admitted A/B builds then passed, one using
installed tools and one using exact-inventory APFS clones. Both prepared the
same 2,923-file, 67,792,387-byte private cache digest, reconciled five A
compiles and three probes, observed the expected invalid-B game compile
failure, and reloaded a runnable retained A. Uncontrolled private-cache
preparation samples took 1,274 ms installed and 1,693 ms cloned; A compiler
stages took 4,115 and 6,193 ms. These are correctness-run observations, with
no speed or latency-tail claim.

The metadata policy still grants selected system-library/cryptex subtrees,
`allow process*`, `mach-lookup` and `sysctl-read`. It does not trace all
implicit Cargo/Rustc/OS reads, authenticate the local candidate root
digests, protect the clones from a same-user writer, settle compiler-job
containment, or publish a ready `BuildRevision`. The candidate remains
`buildReady:false`.
