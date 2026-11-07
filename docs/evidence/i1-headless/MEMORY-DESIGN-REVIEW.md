# Publication-memory design challenge

1 November 2026. A read-only review challenged the [I1 publication-memory gate](../../design/I1-HEADLESS.md#publication-memory-admission-gate) against C02 and the current authority, world and replica code. It found four concrete gaps before integration. The governing design now selects the following mechanisms; none is an implementation or memory qualification result.

| Reviewed gap | Selected resolution |
| --- | --- |
| Later actions in one tick must see earlier effects, while capacity refusal must precede the first commit. | Reserve the conservative **whole-tick** peak first, then decide and commit actions in canonical order. Refusal leaves the tick and pending inputs unchanged. |
| Net-zero spawn/despawn can still raise slot high-water and allocate pages/root buffers. | Charge peak slot growth, dirty pages and transient roots, not only final live entities or lifecycle records. Test 65 spawn/despawn pairs. |
| Baselines, deep group copies and replica admission allocate outside or after authority publication. | Add fallible budgeted construction/baseline/copy paths; reserve in-memory replica growth before authority commit and carry payload/output charges through ownership transfers. Remote peers use their own admission and cannot stall server authority. |
| Completed results retain original input payloads for duplicate detection. | Admit bounded input payloads and carry their charge from pending to completed history until acknowledgement, expiry or reconnect. |

The follow-up review reported no remaining contradiction in those design choices. The next proof must exercise these paths and compare logical charges with actual owned allocation scope. Current tests only prove the reservation arithmetic and existing count-limited publications.
