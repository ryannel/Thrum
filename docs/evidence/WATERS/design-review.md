# WATERS design review

14 November 2026. Reviewed checkout `f3c0021c8b361702c2d29048e8bfd8b59f17b477`, against the previous review at `b2d0778af38e5f685e9291d68486aab5cbc98e0c`.

**Design Changes Requested: two Must Fix findings.** The module split and shared water system address the human's request. The revision resolves disconnected-water isolation, the finite-depth update and its bounded depth ladder, and the pinned beach waterline. Two parts of the previous findings remain: joins do not include the new body-specific swash depth, and the commissioned shoreward foam motion still has no transport mechanism.

This judges the design, not implementation, visual quality or device performance. I read and judged it independently, without delegates. No GPU, screenshots, motion or panel cadence were inspected.

## Previous Must Fix findings

| Previous finding | Result against this revision |
|---|---|
| **Body precedence does not produce a continuous join** | **Partly resolved; still Must Fix.** The map now mixes levels, currents, gains and water types, and the ends follow their neighbours' stages. That resolves the old raw-field and stage examples. The new per-body swash depth is outside the mix, so the assembled, depth-limited surface can still step at a shallow mouth. Finding 1 below retains this name. |
| **Zeroing dry cells does not stop this FFT operator transmitting across land** | **Resolved for disconnected wet regions.** Each region is transformed with other regions at rest and retains only its own output. Part J reproduces zero cross-region transmission. Leakage around thin obstacles inside one connected region is now an explicitly declared approximation. M6.4 reads the water beyond the bar and exercises refusal at the solve cap. |
| **The beach mechanism pins the waterline that the items require to run up and recede** | **Partly resolved; still Must Fix for its foam-transport portion.** The shore margin and swash depth move the waterline, shared wet-now support reaches the query and births, and the film supplies the missing drawing route for exposed lace. Part L demonstrates the moving line. However, the previous finding also identified the missing wave-driven foam wash; the revision expressly omits shoreward transport while M7.9 retains it. Finding 2 below retains the previous name for tracking; the waterline itself is no longer pinned. |
| **The bounded finite-depth solver is not the solver its proof measures** | **Resolved at design level.** The height update now uses `D = k tanh(kd)` as well as the finite-depth frequency. The two explicit ladders end in a deep operator and define interpolation below their first depth. Part H advances height and potential fields over varying beds, checks long packets in a mixed-depth window and compares one- and eight-tick steps. M6.2 follows those cases; M6.12 includes region/solve counts and CPU labelling cost. |

## Must Fix findings

### 1. Body precedence does not produce a continuous join

**Must Fix — verified specification/composition gap, with an independent calculation.** Repeat of previous finding 1. Affects W6, M1.5, M1.12, M9.1, M9.3 and M10.4.

The revised [join rule](../../plans/WATERS/DESIGN.md#the-water-map), lines 309–318, mixes level, current, four gains and water type. At weight 1 it claims that matching this row also matches the displaced surface. But the revised [limiter](../../plans/WATERS/DESIGN.md#waves-on-a-body-gains), lines 377–381, depends on another input: swash depth, explicitly **one number per body each tick**, derived from that body's waves before limiting. A chart's own waves exclude swell; a mouth admits it through the spatial join. No rule makes the chart's swash depth or wave support follow the joined body's at the end. One constant per chart cannot generally represent the differing conditions at both its ends and through its reach.

Take no wind, a regular sea swell of amplitude 0.5 m, and a shallow chart mouth with 0.25 m depth on both sides. Choose a margin height of at least 1 m. At the end the join makes both raw swell gains 1 and both mean levels equal. The sea's significant height is `2√2 × 0.5`, giving swash depth `0.989949494 m`; the chart's own waves have zero height, giving zero swash depth. Applying the specified limiter gives:

| Side | Limited crest above the common mean level |
|---|---:|
| Sea | 0.263033009 m |
| Chart | 0.053033009 m |
| **Step** | **0.210000000 m** |

The same calculation gives no step at 4 m depth, so a deep-mouth example can conceal it. This is an admissible local shallow-mouth case, not a GPU measurement. [Source](design-review/check_revised_join.py) and [output](design-review/revised-join.txt) are retained.

[Part K](design-proof/water_proof.py), lines 425–460, compares only `level + gain × crest`. It never applies the swash-dependent depth limiter, so its agreeing end heights do not establish the revised surface's continuity. Computing swash depth from the locally mixed row could change this result, but that is a further design rule, inconsistent with leaving it as one constant per body; it is not specified or exercised by K.

**What resolves it:** make the complete physical surface calculation, including its depth limit and wave support, agree across both chart ends. Qualify the assembled calculation at a shallow end under swell and stage changes, rather than just the unbounded raw field. Reconcile M1.4's unchanged absolute “Swell enters no basin and no chart” with the join exception too. Keep the one map/query authority; the already-correct raw-row mix need not be discarded.

### 2. Wave-driven shoreward foam wash still has no transport

**Must Fix — verified specification/item mismatch.** The tracking name remains **“The beach mechanism pins the waterline that the items require to run up and recede”**, previous finding 3. Only its foam-transport portion remains. Affects S4 and M7.9.

M7.9 still requires foam to **ride the sheet of water up the sand**, then remain as lace when it draws back. The revised [transport rule](../../plans/WATERS/DESIGN.md#whitewater), lines 693–699, carries stocks with the map's current and wind drift, spreads them by diffusion, and explicitly says: “Nothing carries foam shoreward”. Instead, new foam is born at successive places covered by the advancing sheet. That supplies foam coverage at the top of the swash and leaves a persistent residue; it does not carry the existing material there.

An ordinary discriminating case is a swell approaching the beach with no wind and no mean current. The revised swash moves the waterline several metres, but the prescribed foam drift is zero. A pre-existing patch may spread and age; it has no wave-driven shoreward displacement. Newly born cover farther up the beach cannot establish that the patch rode the sheet. Wind drift, when present, is not coupled to the incoming and retreating waves either.

The new film fixes the old rendering omission: exposed foam can now be seen. The swash fixes the old pinned shoreline. Those are substantive repairs, but neither supplies this remaining motion. This finding comes from the declared operator and the retained inspection requirement; I have not judged whether the substitute could look convincing in a rendered sequence.

**What resolves it:** reconcile the commissioned foam wash with a mechanism that carries the material with the swash, including how it is left on exposed ground. If the intended scope is instead foam born locally beneath an advancing sheet with no wave-driven transport, make that change explicit in the assumptions and acceptance wording so a builder and reviewer are not asked for conflicting outcomes. This does not require overturning surf or a general live flow solver.

## Changed-item review

I read all 118 current items, their proofs and milestone quality bars. The following accounts for each of the 21 planner-item changes named in the task. These are judgments of whether the items can fairly accept the proposed work, not milestone verdicts.

| Item | Assessment against the previous findings |
|---|---|
| M1.2 | Stage-range and bank/end admission now make the permitted topology explicit. Basin merging is deliberately left to the named PUDDLES seam discussion. |
| M1.3 | Bounded stages, counted clamping and reversible fills are judgeable on the CPU in M1. |
| M1.5 | The swash remainder fixes the isolated shoreline limit; its per-body input still needs composition with joins, finding 1. |
| M1.6 | Wet-now answers connect the moving surface to physical support and are judgeable without a GPU. |
| M1.12 | The raw-row and stage comparisons reject old precedence. The full surface claim needs the missing limiter composition, finding 1. |
| M1.13 | The independent closed form rejects the pinned shoreline and is available in M1. |
| M4.2 | Rest-place ownership states how overlapping meshes select one displaced sheet; the waves-off classification is judgeable in M4. |
| M4.5 | Run-up and recession now have a mechanism. The gale/trade-wind inspection remains responsible for whether it reads as a beach. |
| M4.10 | Submitted depth compared with the CPU moving line adds a meaningful guard against a merely animated edge. See suggestion 1 concerning the later film. |
| M5.1 | Query wet/dry classification now includes swash at the use tick; M1's model and M4's surface precede it. |
| M6.2 | Tests the selected finite-depth update and bounded ladder, including varying fields and long waves. Resolves the old proof mismatch. |
| M6.4 | Tests state beyond dry land and solve-cap refusal, which rejects the old masked global solve. |
| M6.10 | The lake/pond/sea inspection makes disconnected-water isolation visible, in addition to the numeric check. |
| M6.12 | Counts the newly multiplied solves and region-labelling CPU work; timing remains a baseline, as the brief requires. |
| M7.5 | Births now use wet-now support and lost power; the lake comparison is proportional rather than an unjustified absolute zero. |
| M7.9 | Lace and sheen now have a drawing route, but riding the swash still contradicts the selected transport, finding 2. |
| M7.15 | Film is explicitly residual, physically dry and without births; query/birth inspection can reject a cosmetic layer masquerading as water. |
| M9.1 | Swell and endpoint stages strengthen the join check. A shared CPU/GPU limiter defect must not be accepted merely because the two agree, finding 1. |
| M9.3 | The temporal join inspection is appropriate, but the specified assembly still permits a shallow-end step. |
| M9.4 | Crossing to the sea without a level/current step is judgeable from the existing query and example integration. |
| M10.4 | Both controls now exercise connected endpoints, but rely on resolution of finding 1. |

The other items retain a sensible proof order: CPU authority, first frame and cost, Chrome, inland water, queries, interactions, foam, optics, a stand-in channel, temporal controls, and Apple qualification. The governor and MetalFX gate remains explicit. The revision does not require a new physical device or move numeric performance into acceptance gates.

## Suggestions

1. **Distinguish sheet depth from residual-film depth in M4.10 — verified wording risk.** The moving-waterline test identifies water by written depth in front of terrain. M7 later adds a depth-biased film on physically dry terrain. State how that test distinguishes the sheet from the film so the new residual cannot be counted as physical water or make the earlier test fail. This is a proof clarification; C07 support and M7.15 already state the intended distinction.
2. **Keep the thin-obstacle leakage figures scoped to their measured cases — verified evidence limit.** Lines 596–600 describe a spit, pier or rock as leaking “at most a few percent”. Part J establishes values for specified 1D barriers, depths, one pulse and a ten-second run. It supports that approximation as a design example, not a universal bound for every obstacle, source and duration. Preserve those conditions when describing the figures; no additional broad experiment campaign is required by this review.

Both previous suggestions were taken: the module list explicitly gives PUDDLES the merge/separation question, the plan now has RIVERS planned before M9, and M7.5/M7.9 allow the lake's physically smaller foam contribution.

## Request coverage and scope

I read the approved [project brief](../../plans/BRIEF.md) and both verbatim tasks under `briefs` in [RECORD.json](../../plans/WATERS/RECORD.json), oldest first. The second task asks the planner to resolve the previous review; it does not replace the human's request or the approved project brief.

The human asks for cohesive water covering puddles, rivers, waterfalls, streams, lakes and more. The design assigns one shared map, wave composition, query, foam and optics to WATERS; SEA supplies the wave field; RIVERS and PUDDLES have explicit future ownership and seam requirements. This division remains a reasonable answer. Prepared flow, no underwater camera, no stacked surfaces, no overturning surf and screen-space reflection limits remain explicit assumptions. I found no new reason to reject that scope or require all later modules to be built inside WATERS. Water stays outside the V1 exit.

The review covered the full revised design and plan, the prior review, the changed proof source and retained results, relevant C02/C04/C06–C09/C12 clauses, the module-list seams and later-module briefs, and the SEA/EFFECT-GRAPH interface descriptions relevant to this revision. I did not independently requalify predecessor implementations, Broadsides, the inherited A–G reproductions or external research claims. The numerical evidence below proves only the stated specification cases.

## Validation

- `python3 -B docs/evidence/WATERS/design-proof/water_proof.py` completed successfully. Its [rerun output](design-review/revised-proof-rerun.txt) is byte-for-byte identical to the retained `water-proof.txt`; H–L were read before rerunning them.
- `python3 -B docs/evidence/WATERS/design-review/check_revised_join.py` passed its assertion and produced the retained [shallow-mouth calculation](design-review/revised-join.txt).
- `./dev check-docs` passed: 838 Markdown files, 5,959 local links, 30 module records, crate policy and current design hashes. Existing source-size notes reported no errors. Output is in [check-docs.txt](design-review/check-docs.txt). `git diff --check` also passed.
- Only this review and files inside its `design-review/` evidence folder were changed. Plans, DONE.json, records and implementation were left untouched; no commit was made.

DESIGN CHANGES: 2 must_fix findings
