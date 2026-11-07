# F4 native out-of-space during pinned materialization — 3 November 2026

One opt-in macOS ARM64 regression exercised the existing F4 verified-copy
path on a task-owned 256 MiB APFS image with Rust 1.99.1. The selected F3
store and 12 MiB accepted master remained on the host volume; only the F4
destination and competing filler used the image. This isolates F4's local
copy/cleanup behavior, not a same-volume T2 aggregate account.

The test-only checkpoint runs immediately after F4's real manifest/space
preflight and before destination creation. It adds no public API or per-chunk
callback to the production copy. The checkpoint writes a bounded filler until
reported free space falls below 14 MiB. A bounded-volume check refuses an
accidental host-volume path. F4 then copies the unchanged accepted pin through
its ordinary SQLite verified reader and native file writer.

In four local runs, preflight preceded a free-space change from **266,850,304**
to **14,143,488** bytes. F4 had written **4,194,304 bytes** of the 12 MiB
master when a later native output write returned errno **28** (`ENOSPC`). The
error was F3's typed consumer-output failure inside F4's `Source` error, not
an F4 planning refusal. The test-only pre-cleanup observation verified the
partial file length; F4 then removed the complete failed input directory.
After removing the filler, the same accepted pin materialized the full master
and exact source-root identity at the same destination. The final run streamed
all 12 MiB of the retried master and checked every byte against the admitted
fixture value; explicit close removed that input. No ready build or
source-root change was involved.

Reproduce on a separately mounted, exclusive 64–512 MiB APFS image:

```sh
THRUM_TEST_ENOSPC_VOLUME_DIR=/absolute/path/to/task-owned/mount \
  RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-cook \
  native_enospc_after_f4_preflight_removes_partial_input_and_allows_retry \
  -- --ignored --nocapture --test-threads=1
RUSTC_WRAPPER= ./dev check
```

The test shares the existing volume lock name used by F3's opt-in native
space-fault tests. The task image was detached and deleted after use. This
does not prove T2 registration/claim refund under native ENOSPC, a same-volume
F3/F4 competition, cleanup failure under full-disk pressure, transient
physical high-water, another filesystem, power-loss durability or compiler
output handling. The complete T2 account remains open.
