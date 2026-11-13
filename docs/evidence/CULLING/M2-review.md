# CULLING M2 review

**FAIL — four items pass; M2.2 fails its complete-scene proof.** No outside-item Must Fix.

Reviewed 13 November 2026 at `85a3212c50a6bbe951cf82550c5a040909716fa0`, with builder boundary `04e0e7d2ff51`. This is M2's first review. Applied review-milestone, Thrum engineering review guidance and prompting-agents for actionable findings. No delegation, production edits or commits.

## Findings

### Must Fix — `simulator_complete_scene_unchecked` — M2.2

**Verified proof defect, not a demonstrated missing-geometry defect.** In [scripts/culling_sim.py](../../../scripts/culling_sim.py#L49), the only rendering assertions are eight increasing `presented` values and requested/resolved/reason strings. Lines 63–64 save a screenshot and immediately set `pass = True`. No screenshot pixels, actual draw population or Direct reference are checked. A renderer which reports Direct but omits instances can pass this check. Thus the command does not establish the item's “draws the complete scene” clause.

I ran the actual command against this checkout, under the GPU lock, with `--output docs/evidence/CULLING/M2-review/simulator-retry`. It returned success: [report](M2-review/simulator-retry/report.json). Presentations advance from 115 to 955; all eight samples report Occlusion → Direct, `MissingIndirectExecution`. The [reviewer screenshot](M2-review/simulator-retry/direct-fallback.png) visibly contains the dense grid and foreground cubes. That corroborates functioning rendering, but neither the screenshot nor its unchecked capture establishes completeness. The Mac masked-capability image comparison is useful independent evidence for M2.1, not a complete-scene assertion on the simulator target named by M2.2.

Make the simulator check reject incomplete rendering while retaining the real capability fallback and progressing presentations. A same-input explicit-Direct image oracle, or an equivalent witness tied to the actual complete draw, would settle this. Demonstrate that omission of expected scene geometry makes the check fail. The requirement is completeness, not a particular new harness or a simulator speed target.

### Suggestion — stale timing values in the maturity ledger

**Verified documentation discrepancy.** [maturity.json, G3c platform performance](../../design/maturity.json#L112) records cover O−F medians −7.844/−7.835 ms and open −0.231/−0.255 ms. The two retained current logs and M2 evidence instead give −7.827/−7.822 and −0.239/−0.256 ms. Update the ledger to the current rows, or identify the older run it describes. This does not reverse the default decision or fail M2.4.

## Item verdicts

| Item | Verdict | Assessment |
| --- | --- | --- |
| M2.1 | pass | The automated target genuinely prepares and executes all three requested modes against adapter-read, indirect-masked and unknown records. It asserts selection reason and actual prepared mode, requires nonuniform pixels, and compares fallback images with explicit Direct. The request-to-pipeline trace below answers the reviewer question. |
| M2.2 | fail | Actual simulator launch, capability reason and progressing presentations work; complete-scene rejection is absent from the named automated proof. See Must Fix above. |
| M2.3 | pass | The checker builds the current default web bundle and supplies `field-meshes` to the real field setup. It compares all 16 RGBA readbacks at matching tick, extent, yaw and pitch against bucket-ordered Direct without tolerance. Resolved-mode and nonzero/reduced-count assertions prevent Direct masquerading as culling. Retained results have zero differing pixels for both modes, with reported Frustum/Occlusion counts 20,789/1,415 of 100,513. |
| M2.4 | pass | Both independent retained 1080p runs support Occlusion under the written selection rule, in cover and open groups. The benchmark rotates mode order per tick, records segment load, includes every timed scene pass, and asserts nonempty prepasses and actual hiding. Native and browser defaults call the same mixed-mesh selector. |
| M2.5 | pass | The named verifier rebuilds native and Web artifacts, runs the registered native matrices and four Chrome digest sets, and refuses changed checkout/content identity before publishing proof. It retains exact comparisons rather than weakening baselines. The script's supplied pass at the builder boundary is accepted. |

### Request, preparation and reporting trace

`selected_setup` / browser `create_field_game` pass the requested mode through `GameSetup` into the retained `SyntheticScene`. `PreparedSynthetic::prepare_internal` resolves it once using `CullingSelection::resolve` and the immutable capability record. The resolved value controls the vertex entry, visibility resources, bucket depth pipelines and prepass creation. Unknown or missing indirect facts select Direct; explicit Direct stays Direct. The final camera draw uses ordinary indexed draws when no visibility resources exist, including the bucket route. Caster culling is also absent on that Direct route.

The scene's original request is preserved. Device replacement calls the common preparation body with that scene and the new capability generation. Recording does not query adapter facts. The stored selection reaches frame samples and native reports, browser reports and the simulator state file. I read producers and consumers across these boundaries, including capabilities capture, runtime preparation/replacement, mesh drawing and reporting.

The cube equality helpers, mesh helper, `visibility_buckets` and full-field host helper assert prepared/resolved modes. The repaired mesh assertion compares each prepared renderer to its own request, correctly avoiding the shared frame input's mode marker; it does not relax pixel equality.

### Measurement assessment

[Run 1 rows](M2-review/bench-1-rows.txt) and [run 2 rows](M2-review/bench-2-rows.txt) are excerpts from the retained builder logs, not new reviewer timing runs. Both measure 1,800 ticks after 60 warm-up ticks, with 920 cover and 880 open samples; both report 1,800 nonempty prepasses and 1,065 hiding ticks. Cover is classified by Occlusion retaining at most half of Frustum's list; pitch changes halfway through to expose open views.

| Run | Cover paired O−F median | Open paired O−F median | Open paired p95 |
| --- | --- | --- | --- |
| 1 | −7.827 ms | −0.239 ms | +0.023 ms |
| 2 | −7.822 ms | −0.256 ms | −0.000 ms |

Occlusion also has lower whole-GPU p50/p95 than Direct in both groups. The small open-view median win satisfies the written rule; it is not a claim of winning every sample or providing full-refresh cadence. Rows are explicitly contended, and the sum excludes untimed transfer/query work and physical presentation. The [source audit](M2-review/source-audit.json) confirms the benchmark source hash matches the retained measurement identity; the relevant renderer/game difference since that measurement is the mesh test assertion, not production rendering.

## Review execution and inspection limits

I read the M2 implementation changes, follow-up test repair, default selection, simulator/browser harness changes, shared Wasm receipt repair and M2 documentation changes. Unrelated concurrent module implementations were not treated as CULLING changes. `git show 04e0e7d2ff51` itself changes STATUS only; the M2 implementation is in its ancestors. The checkout also includes the later browser receipt repair, which I inspected. No hard-coded render result, weakened pixel oracle or test-only production path was found.

Per the handoff, I accepted the script's successful `./dev check`, M2.1, M2.3 and M2.5 runs rather than repeating them. I repeated only M2.2 to investigate its output-proof gap. The first attempt built successfully but could not reach CoreSimulator inside the sandbox; the escalated retry succeeded. [Initial access report](M2-review/simulator/report.json) and [successful target report](M2-review/simulator-retry/report.json) keep those outcomes distinct. Access is not blocked.

M2 declares no `inspect` item. For the supplemental simulator check I wrote an [inspection list](M2-review/inspection-plan.md) before looking at the capture:

- **Scene content:** dense cube grid and large foreground cubes are visible; the frame is neither blank nor a fault screen. The image does not prove every expected instance was drawn.
- **Identity and decision:** report names this checkout, the built binary hash and the real simulator adapter, with the requested/resolved/reason values above.
- **Direct draw mechanism:** source submits the full active range, corroborated by the Mac fallback equality test; no target-side completeness assertion exists in the simulator check.

I viewed a whole-frame simulator still scaled to fit, not elapsed-time motion or native-size edge crops. No claim is made about physical iPhone speed. Rotation and background/foreground were not exercised in this supplemental run and are not inferred from its counters. Mac low sun/storm/full-refresh motion and Chrome temporal inspection were not rerun: M2's accepted checks establish bounded capability/equality results; the explicit motion and temporal-culling items remain in M3.

Set M2 back to `in_progress`, with only the M2.2 finding above. Builder status and all counts remain unchanged for the script.

FAIL M2: 1 fail, 0 could_not_judge, 0 outside the items
