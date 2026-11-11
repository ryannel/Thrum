# PLATFORM-RUNTIME-MAC: console witness correction

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. The builder's commission is as the planner wrote it on 9 November 2026; the design's `Engine motion and console witnesses` section holds the policy.

The builder changes `scripts/capture_mac_m2_ungranted.py` to stop requiring
`locked_permission_unavailable_qualified` from `scripts/capture_v1_runtime.py`. Qualify capture independence from
successful child execution and denied attributed launcher/driver Screen Recording preflights, retaining the existing
launcher before/after checks and adding a driver after-run preflight alongside its existing before-run observation
in `scripts/capture_v1_runtime.py`. Separate that proof from the runtime's combined lock/permission summary. Preserve the combined field as historical metadata if retained; its lock values cannot
gate open M2.1/M3.1. Preserve actual console witnesses, including false or changed values and reasons for unavailable
observations. Missing, failed or granted permission preflights still fail capture-independence qualification.
Add focused acceptance/refusal controls, rerun the documented M2.1 pan under `gpu` and retain fresh visual/timing/
completeness proof before review. M3.1 repeats this corrected protocol after APPLE-TILE M3 on its new build.
