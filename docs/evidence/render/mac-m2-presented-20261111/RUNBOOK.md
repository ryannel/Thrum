# M2 presented-time sitting

The reviewed `ab66bca5` commission grants six trace recordings total. The durable counter is
`tmp/mac-m2/presented-commission-20261111.json`; preserve it across output folders and restarts.
The older `tmp/mac-m2/trace-commission.json` remains exhausted history. No security setting changes.

The Mac must be unlocked with the built-in main panel awake. The collector checks this before consuming an
attempt and every 0.5 seconds while recording/exporting, terminating only its owned process group on refusal.
Other builders’ CPU work may continue; retain the load and iostat records beside every result.

Two attempts are used: full Game Performance exceeded its process deadline; the smaller explicit-window run
finished but retained only the final 1.075 seconds. The current changed configuration removes the ring window:

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/qualify_mac_m2.py --presented-minimal --presented-deferred --presented-arm off-traced --output tmp/mac-m2/presented-deferred-20261111
```

Never overwrite an arm. The command refuses an unchanged failed hypothesis. It pins the retained release binary,
preset and cooked store. It records at most 16 seconds with a 75-second finalization allowance, and finite
30-second exports. It saves its executed collector source and hash. The minimal configuration retains the
Metal Application, Display and Points of Interest instruments, omitting `--window`; the installed Blank template
declares `windowLimit=0`. Verify the fresh TOC mode and actual retained coverage before accepting it.

Inspect the fresh result without another recording:

```sh
python3 scripts/mac_m2_presented.py --input tmp/mac-m2/presented-deferred-20261111/off-traced/pan.trace --arm tmp/mac-m2/presented-deferred-20261111/off-traced --output tmp/mac-m2/presented-deferred-20261111/off-traced/witness.json
```

Also inspect TOC recording mode, game PID/window, display identity (only the built-in panel, or identify the
window’s display separately), raw/derived display-table counts and the full marker coverage. Every submission
must join or be explicitly unmatched. Read every missed-refresh row beside the game call gap, yaw and host load.
A qualified bounded OFF probe can serve M2.2’s OFF arm. After M2.3/M2.4/M2.5 pass, collect matched
`--presented-arm on-untraced` and `--presented-arm on-traced` with the same output root/configuration under GPU.
Retain and inspect complete native ON originals, crops and host-time 1× movies. No claim from movie PTS or
callback intervals. A locked/asleep panel stops the sitting immediately; unattended analysis can continue.


Resume after the second sitting: M2.5 now has builder proof from attempt 3; three of six traces are used.
The `on-untraced` arm exists and all 850 originals verify. Preserve its measured interval brackets (including
1.992085 s between sweep markers) for final qualification. `on-traced` does not exist: its disk preflight refused
below 16 GiB, before launch or attempt consumption. Restore at least 16 GiB of shared-volume headroom, keep the
built-in panel awake/unlocked, then run the same command above with `--presented-arm on-traced` and the same output
root under GPU. Do not overwrite completed arms. Complete its witness/source review and observer comparison;
recheck all ON brackets before passing M2.2.
