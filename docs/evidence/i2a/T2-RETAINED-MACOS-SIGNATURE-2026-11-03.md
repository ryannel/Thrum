# Retained macOS executable launch fault and fixture correction

3 November 2026. This diagnoses the previously unchecked retained-A rerun
failure in the opt-in native A/B qualifier. It does not establish release
readiness or a general macOS launch guarantee.

## Failure and cause

After A compiled, its executable was retained, content-hashed, reloaded and
run. The fixture then opened that **same executed inode** for writing, flipped
its first byte to check candidate corruption refusal, and wrote the original
byte back. After invalid B, candidate verification and SHA-256 still matched,
but a later A launch sometimes exited with signal 9. One diagnostic run
reported `SIGKILL (Code Signature Invalid)` in the macOS 26.6.2 crash report;
an earlier saved report had the same reason. The exact preserved failed inode
passed `codesign --verify --verbose=4` after the kill and ran 100 times from a
diagnostic path. Recreating the old path as a hard link also ran it 100 times.
Those later successes do not invalidate the earlier kernel failure.

[Apple's signed-code update guidance](https://developer.apple.com/documentation/security/updating-mac-software?language=_4)
explains the mechanism: macOS caches code-signature information for an inode
and does not flush that cache after an in-place content change. Restoring
identical bytes can therefore leave a later execution vulnerable to a
hard-to-reproduce signature crash. This directly matches the fixture's
mutate/restore sequence. A controlled APFS-clone retention experiment also
failed at the same later launch, so changing the candidate copy method was
rejected as the fix.

The fixture now copies the candidate to a **different directory and inode**
for each binary and wrapper tamper test, verifies that altered copy refuses,
then removes it. The successful retained A inode is never opened for writing.
Its original byte and report checks, fresh-process reload, invalid B, source
drift and later A execution remain in the same native A/B workflow. A
failure-only diagnostic can hard-link the failed inode into owned scratch and
run `codesign --verify`; diagnostic failures are best-effort and cannot hide
the original exit status. Successful runs create no retained diagnostic.

## Qualification and limits

Two fresh-owner native macOS ARM64 A/B runs with the corrected fixture passed:
one admitted A/B process took 32.759 s with 11.810 s post-build four-root
inventory; the second took 28.502 s with 11.531 s postinventory. Each saw
five successful A compiles and three probes, one expected failed B game
compile after three successful compiles and three probes, a runnable retained
A after B, 1 ms held-cache reuse checks, and unchanged selected roots. These
are uncontrolled correctness runs, not speed measurements. Those qualifiers
label the prior `HEAD` because the source correction was still uncommitted.
An exact committed-source rerun at `422d499` also passed: the admitted A/B
process took 28.046 s, the four-root postinventory 10.019 s, A's retained
reload 24 ms, and the two held dependency-key checks 1 ms each. It again
observed five A compiles/three probes, expected invalid B, and retained A
runnable after B. No paired performance claim follows from these runs.

The previous failed scratch was quarantined by the qualifier. The selected
failure log and crash reason are summarized here without retaining the full
multi-gigabyte clone or private macOS crash report in the repository. No
retry, re-signing step or hidden launch fallback was introduced. Release
selection, complete compiler/system reads, durable ready publication and
device stability remain separate gates.
