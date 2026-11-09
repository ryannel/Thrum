# Mac M2 current-path qualification

**M2.1 passes builder inspection; M2 remains blocked on attended M2.3/M2.4/M2.2.** No panel proof is claimed.
The current release on Apple M3 Max/Metal retains the complete 1920×1080 paused-noon elapsed-time pan through the
ordinary semantic control/presenter path, at 120 Hz requested opportunities. The console was locked before/after;
launcher and driver preflights were false. Binary SHA-256: `c281cde11af9dd0cf86062c8d7516cecd8b2aaa79d44cc1756f11042198c8769`.

[Inspection](inspection.json), [manifest](manifest.json.gz) and [runtime](runtime.json.gz) retain all 481 image/control/
submission joins: zero skipped opportunities, ring omissions or writer omissions; tick 0 and history epoch 0 throughout.
The initial hold is 1.003418 s; 36° first submits at 3.004168 s and holds through interval end 4.010894 s (1.006726 s).
Largest adjacent source gap: 12.008083 ms. No repeated accepted yaw or exact repeated image bytes inside the sweep.
Every original RGBA SHA-256 and every PNG round trip verified independently; preview PTS error ≤0.5 μs.

Builder inspection of all selected full native frames/crops, [navigation sheet](contact.png) and
[host-time 1× sequence](pan-1x.mp4): ridges and rock preserve coherent relative placement, shadows stay attached, the
sphere moves left and exits as authored, and both settled endpoints agree with yaw 0°/36°. No cut, snap or reversal,
visible gross pause or anomalous local hold. Ground E identifies scene/framing only. The background browser played
4.011388 s at rate 1 in about 4.02 s; it counted eight decode/compositor drops across the first two passes and 122
cumulatively after screenshot sampling. These are review-observer drops, not omissions in the verified sources or
panel swaps. Native sources/crops were checked separately. Fresh independent lead review is still owed.

The selected originals are `full-0/1.5/2/2.5/3.5.png`, with native `crop-*` views; IDs, crop rectangles and full RGBA
hashes are in inspection.json. Local complete `frames.zst` plus PNG/timeline/preview is `tmp/mac-m2/qualified-01`;
its launch context is `tmp/mac-m2/ungranted`. The new reproduction helper also ran successfully in
`tmp/mac-m2/command-check`, on the same binary with false preflights and locked console; its complete archive verified.
Neither run changes display-link, compositor or panel maturity.
The [attended continuation](RUNBOOK.md) supplies the one resumable command and exact evidence gates.
Full `./dev check` under the shared GPU lock passed on 9 November; new native image/input/submission-join refusal
controls also passed. The native harness is compiled and regression checked; its actual-window proofs remain open.

The normal Codex process's Screen Recording preflight is now true. Two complete locked-console baseline/pan archives
therefore do not pass the denied-permission condition. A separate, locally built LaunchServices app has denied
preflights without requesting a grant. Its initial run failed before rendering because LaunchServices supplied the
wrong working directory; corrected before retry. Every attempt is retained under `tmp/mac-m2/`.

Reproduce the unattended M2.1 collection in a new directory, under the GPU lock, with the console already locked:

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.4.0/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/capture_mac_m2_ungranted.py --output tmp/mac-m2/fresh
python3 scripts/capture_v1_runtime.py --output tmp/mac-m2/fresh/session --finalize
```

Build/cook commands and source-format bounds are in [CAPTURE](../../../CAPTURE.md). The local release binary,
`tmp/mac-m2/valley` and complete compressed source/timing records are retained through lead review. The storage record
is `tmp/mac-m2/STORAGE.md`. No raw archive is a durable backup or compositor/panel observation. Historical
[`0d53d3aa` motion and 38.3/51.7 ms holds](../mac-m2-pan/README.md) remain historical only.

The attended preflight [record](panel-preflight.json) is the actual refusal on this console. M2.3 window pattern/lease/
stale-session proof, M2.4 fresh readable trace and M2.2 OFF-attributed cadence remain open. No new trace recording was
made; the two-attempt commission remains available. Native collector/packager behavior on an active window is still
unqualified; compile/tests are not promoted into a live pass. All remaining evidence collection/review needs that
attended panel setup; no other module delivery is missing.
