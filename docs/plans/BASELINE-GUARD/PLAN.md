# BASELINE-GUARD plan

A change that alters another module's exact reference images is refused before it reaches main.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[the rules](../ARCHITECTURE.md#exact-baselines)

## Open for the owner

- Assumption: the guard is its own one-milestone module, not an item added to `APPLE-TILE` M3, so it does not sit on the critical path or wait behind that milestone's attended items. It needs nothing and can be built first.
- Assumption: every merge on the Mac pays the guard's run time in `./dev check`; the time is recorded, not budgeted.
- Assumption: the gate guards the Mac only. Chrome's exact baseline is checked at the claim, by the items that the module list's rule 3 adds.

## M1 A change that alters a registered exact image fails the land gate on the Mac

Criteria: M1.1–M1.4 in the done-when file.

- [ ] First, find out whether the gate's debug build draws the baseline's release bytes at the guarded checkpoints. If it does not, diagnose the cause before choosing anything.
- [ ] Add the registry file and the guard test, through the game's own host, scene source, route and capture tap, for both recipes, and make `./dev check` run it.
- [ ] Run the two mutants in scratch (jitter on the default views; one changed code value in a retained file), keep their logs, and merge neither.
- [ ] Add the verifier command, wrapping the owners' scripts, and name it in the registry table of the module list as each baseline's full verify command.
- [ ] Record the time the guard adds to `./dev check` from three runs.

Stop rules:
- Never refresh, regenerate or edit a file under a registered baseline. If the baseline does not match on untouched main, end `BLOCKED` with the comparison; the refresh is its owner's.
- No tolerance, nearest-frame search, retry or skip, for any reason. If exact bytes cannot be reproduced in the gate, stop at the diagnosis and report it.
- Do not change `scripts/apple_tile_m0.py`, `apple_tile_m1.py` or `apple_tile_m2.py`; the verifier calls them.
