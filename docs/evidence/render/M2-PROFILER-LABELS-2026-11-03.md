# M2 profiler label reuse — 3 November 2026

The prepared GPU timestamp batch now retains immutable pass labels across frames.
`pass_writes` accepts borrowed text, reuses an unchanged label slot, and replaces a
slot only when that pass role changes. Results share the label owner, so an older
result remains valid after a later batch changes its role. The active count resets
after collection or a failed map while bounded slot capacity remains available.
The public timing result name is now `Arc<str>`; callers needing an owned `String`
must convert it explicitly. This is profiler plumbing, not clustered-forward or
whole-frame M2 readiness.

## Checks and local observation

- Unit test checks pointer-identical reuse across two batches and replacement
  without altering another slot. Strict all-target Clippy and `./dev check` pass.
- Existing real-adapter test passes pixel, depth, disabled-timestamp and repeated
  timestamp-batch cases on Apple M3 Max, Metal, macOS 26.6.2. The test required
  a normal host run because the sandbox exposed no Metal adapter.
- One 10,000-cube 1920×1080 debug diagnostic, 32 sequential samples: GPU opaque
  pass median 0.122666 ms, p95 0.125375 ms; CPU encode plus timing finish median
  5.584 µs, p95 7.833 µs. These samples are noisy and not an isolated allocation
  or speedup measurement. Label reuse follows from retained `Arc` slot identity;
  a counting-allocator assertion for the complete profiler path remains open.

The timestamp callback, resolve and readback protocol is unchanged. The next
profiler slice must report CPU tick, CPU encode and GPU passes together without
blocking frame submission, then compare sustained allocation and frame tails.
