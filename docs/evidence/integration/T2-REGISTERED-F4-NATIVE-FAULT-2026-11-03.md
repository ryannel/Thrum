# Registered same-volume F4 native space fault — 3 November 2026

Two opt-in macOS ARM64 runs exercised the [first Unix T2 job
host](T2-F4-JOB-HOST-2026-11-03.md) with the source project, accepted SQLite
store, deterministic external job parent, F4 input and competing filler on
one task-owned **256 MiB APFS image**. The source was a valid four-file
accepted A project with a 12 MiB master. The host selected a 24 MiB aggregate
managed-temporary target and a 16 MiB F4 scratch target; these are test
planning values, not a physical cap.

The test-only T2 checkpoint runs after its registration record and parent
barriers have synced, before calling F4. It finds one registered job root with
no input directory, then writes an external filler within the bounded image.
Reported free space falls from **241,319,936** to **13,762,560** bytes.
F4's own manifest and available-space preflight runs afterward and admits the
copy; a later native output write returns errno **28** (`ENOSPC`) through the
typed F3 consumer-output/F4 materialization error. This is a write fault,
not the host's initial planning deferral or F4's preflight refusal.

F4 removes its failed input. T2 verifies the job root contains only its
registration, syncs the cleanup/retirement boundaries, removes the root and
only then reduces its active claim to zero. The parent contains only its owner
lock. After removing the filler, the same live accepted pin produces a second
registered input; the test streams and checks every byte of the 12 MiB
retried master, finds the same selected F3 root, and closes the job with no
claim remaining. The image was inspected for owned run files, detached and
deleted after testing. The independent [F4-local fault
proof](../i2a/F4-NATIVE-MATERIALIZATION-FAULT-2026-11-03.md) observed a 4 MiB
partial file before F4's cleanup on a separate image; this T2 test does not
measure partial length.

Reproduce on an exclusive task-owned 64–512 MiB APFS image:

```sh
THRUM_TEST_ENOSPC_VOLUME_DIR=/absolute/path/to/task-owned/mount \
  RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-jobs \
  registered_f4_mid_copy_enospc_retires_root_and_refunds_only_after_cleanup \
  -- --ignored --nocapture --test-threads=1
RUSTC_WRAPPER= ./dev check
```

The test shares F3/F4's opt-in native-fault volume lock name and caps the
volume size and filler writes. It proves one synchronous registered F4 copy
and checked refund under a real native fault. It does not race F3 stage/store
work with F4 retirement, inject failure into F4 cleanup or T2 sync, account
compiler output/cache/ready roots, recover an uncertain registered fault, or
establish APFS physical high-water or a reserve. The larger T2 account item
remains open.
