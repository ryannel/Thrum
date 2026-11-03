# T2/F4 external compiler-output boundary — 3 November 2026

This opt-in macOS ARM64 fixture tests one placement needed by the [provisional
same-volume account](../../design/research/T2-SAME-VOLUME-JOB-ACCOUNT-2026-11-03.md).
It uses the real pinned A/B Cargo path and held four-root sandbox qualifier,
but does not implement T2 job registration or an aggregate space account.
The tested code checkpoint is `ce05f76147a4dcc633eff55b80b62640d1278295`
on macOS 26.6.2 with Rust 1.99.1. The qualifier commands were:

```sh
RUSTC_WRAPPER= python3.11 scripts/qualify_held_macos_native_ab.py --external-output
RUSTC_WRAPPER= python3.11 scripts/qualify_held_macos_native_ab.py --external-output --fault-guard
```

The qualifier creates a mode-0700 private `compiler-job` root under its owned
scratch, outside the complete admitted source project. The Rust fixture checks
that root is on the same device as the project, has the same owner, is private,
and has absent `output-a` and `output-b` names before writing. A and B use
distinct output subdirectories. `TMPDIR` is `OUTPUT/tmp`; the sandbox permits
writes under `OUTPUT` and denies a sibling under the private job root and a
write to the input tree. A positive `OUTPUT/tmp` probe confirms the allowed
write. Existing candidate publication remains separately under project
`.engine` and retains its own lifecycle.

On the exact integrated code checkpoint, five A rustc compiles and three
probes were verified. Invalid B had three successful dependency compiles,
one failed game compile and three probes. A's candidate was reloaded and ran
after its original compiler output was removed and B failed. The A/B process
took 28.993 seconds, including fixture preparation and checks; the post-build
four-root inventory took another 10.741 seconds. These are not isolated compiler
or placement costs. A prior restricted-shell attempt stopped at macOS
`sandbox_apply: Operation not permitted` before Cargo and preserved its owned
scratch; the privileged local rerun passed. An earlier A/B pass on the same
fixture changes before commit took 28.691 seconds. The reviewer found and we corrected
a false guard-loss candidate-location assertion, stale external output-name
reuse and a non-macOS unused-parameter warning before the final code gate.

An injected held-guard loss on exact code commit `d7495cc` returned an
`interrupted` unready supervisor report with one pending compiler receipt and
`candidate_absent=true` at the explicit retained-candidate path. Its A/B
process took 25.566 seconds and the later full four-root inventory took
10.949 seconds. The exact rebase added an independent external-input test;
the native output path and qualifier code did not change.

This proves an executable external compiler-output and nested temporary path
for the selected local fixture. It does not prove a durable T2 registration,
restart quarantine, concurrent claims, compiler overrun handling, native
`ENOSPC`, complete compiler read closure, reviewed release origin, ready
publication, a physical APFS cap or an end-to-end speedup. The full repository
gate passed after the code changes; independent defect-first review had no
remaining findings.
