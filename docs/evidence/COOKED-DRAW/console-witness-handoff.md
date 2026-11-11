# COOKED-DRAW: console witness correction and verification handoff

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. The handoff is as the planner wrote it on 9 November 2026; M1 has since passed [its review](M1-review.md).

The builder owns `scripts/cooked_draw_ios_sim.py` and the affected cases in `scripts/test_t2_capture.py`. Remove
`wait_for_console_lock`, its 120-second manual setup wait and every before/after lock state refusal, across ordinary,
`--lifecycle`, `--rotation` and `--visuals` modes. Retain actual before/after observations and raw probe output,
recording unavailable observations with a reason. Add read-only denied Screen Recording preflights before and after,
with explicit launcher/harness permission identity, following the existing ungranted-launcher attribution pattern
where needed. An unrelated denied helper is insufficient. No permission is requested or changed. Existing simulator,
content, advancement, lifecycle, cleanup and validation failures still fail their rows.

Replace the manual-lock admission/timeout tests with controls accepting locked, unlocked and changing observations
without waiting, retaining unavailable-probe reasons, and refusing missing, failed or granted permission preflights
for the independence proof. Run `./dev check` and the exact standalone M1.7 and M1.8 commands under `gpu` on fresh
owned simulators. Re-run any other mode affected by the shared harness change as required for the refreshed claim.
The loop stopped verification before review, so the existing M1.7/M1.8 pass fields and evidence are historical
claims under the superseded wording. Fresh evidence and loop verification are owed before review. Preserve prior
lock witnesses in the history. The loop records the revised definitions and clears obsolete proof. This owner scope
correction spends no failure re-plan and leaves `fails`/`replans` to their existing values. Independent review of
the revised definitions and refreshed claim is owed to the lead/loop; no other agents may be launched here.
