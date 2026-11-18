# RAY-TRACING M2 handoff

The implementation and complete evidence are committed through `e5699d4c42a3e94716f4c2bf5cf9d9ad243d5951`.
Every M2 item has `builder_result` set to `pass` and the milestone is `in_progress`.
[M2 evidence](M2/README.md) links the automated Metal results, the four release T2 pairs and native crops,
the pre-change byte reference, and the contended timing trials.

`agent_loop.py check --module RAY-TRACING --milestone M2 --worktree . --items-only` passed M2.2 through
M2.7 on Metal. M2.8 returned `NOT_ON_MAIN`, as expected for the shared `exact-images` check at a builder
checkpoint. The native workspace Clippy run, the V1 draw-probe Clippy run and the repository documentation
check passed. The 58.5–76.2 ms cold terrain brackets and the 0.737–0.905 ms edit estimate are under load.
The signed comparisons for smaller meshes are sensitive to probe and scheduling overhead.

The two-hour builder turn spent its remaining time waiting behind an earlier workspace test. The queue
rejected the request for `e5699d4c4` after the handoff commit changed the worktree tip. The complete
checkpoint, including this handoff, was then resubmitted as a detached orchestrate merge job.
Check its outcome and fetch main before resubmitting. If that job exited without landing the work,
retry with `agent_loop.py merge --worktree .` from this milestone's worktree.

After a successful merge, use the fixed capture command in the evidence page on the merged release binary.
Compare the eight RGBA SHA-256 values with [the retained captures](M2/stills/capture.json), and reinspect
any changed pairs. Leave M2 `in_progress` and let the lead's script run the checks of record after `DONE`.
