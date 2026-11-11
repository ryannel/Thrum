# ATMOSPHERE design review

Reviewed integrated commit `156912c341ffbfbf8d74c48bfc6cb028d6f187be`, 11 November 2026.
Result: **design approved — no blocking findings**.

This review judges the design and future acceptance criteria, not completion of M5 or M6. It follows the
previous design approval at `631e222b6a5ef4c7ca3fed88d85a2da247f54a0b` and reviews the precision-bound
revision in `848ef71c9fd4bd5b66e6617eee584753eb83e352`. No other agents were launched.

## Previous findings

The previous review had no Must Fix findings. Its suggestion to make the exposure control discriminating
remains open in the design wording; it is carried below without promotion to a blocker.

The earlier Must Fix findings remain resolved:

| Earlier finding | Disposition in this revision |
|---|---|
| Numerical acceptance was not defined product by product | P1–P11 and H1–H5 retain their references, cases and scope. Only the H2/H4 filtered-decode allowance changes. |
| Reflection was missing from the independent handoff proof | H5 still covers the lent reflection operation, split-sum pair, night term, source scaling and rewritten sky. |
| Cooked-volume cloud attenuation was unspecified | The shared cloud factor still applies at each lit march sample's own world position; off preserves existing lighting. |
| Lookup support and generation semantics were unspecified | The support table and replacement-versus-recompute distinction remain explicit. |
| M6 cost comparisons used unmatched history | M6.5 still requires a fresh, matched, order-alternated rerun of the milestone's starting commit. |
| The texel-range rule accepted a wrong aerial lookup | The independent filtered value remains the reference. Wrong lookups must still fail at every interior query on both targets; the stronger known field can meet that requirement with the new allowance. |

## Findings

### Suggestion — qualify the relative-precision and visibility wording

**Verified wording issue.** [DESIGN.md](../../plans/ATMOSPHERE/DESIGN.md), lines 241–244 and 298–302,
and [PLAN.md](../../plans/ATMOSPHERE/PLAN.md), line 12, describe a stored step as at most 0.1% of the
value and reject explicit interpolation as giving no accuracy or visible gain. A half-float step has that
relative scale for normal values near the relevant maximum, not universally for a filtered result. Subnormal
spacing is absolute: at the smallest positive half value, `2^-24`, one step is 100% of the value. A blend near
zero can also be much smaller than the maximum texel used to calculate `Q`. Explicit interpolation does improve
accuracy relative to the filtered stored field; it does not improve the physical accuracy of its input texels.
No new visual comparison establishes that its gain is invisible.

Describe `Q` as an absolute allowance derived from the contributing stored channels, qualify the relative
percentage to its normal-value range, and leave visibility to M6 as the design already requires. This does not
block building: the operative formula uses the actual stored spacing rather than a relative 0.1% shortcut,
and M6 still rejects visible banding. The chosen hardware-filtered path need not change.

### Suggestion — make the exposure control visibly discriminating to the numerical check

**Verified, carried from the previous review.** [DESIGN.md](../../plans/ATMOSPHERE/DESIGN.md), lines
407–409, still permits any exposure scale other than 1. A scale arbitrarily close to 1, or a zero-valued
reference, need not reject an incorrectly scaled lookup. Use a nonzero case and a scale sufficiently far from
1 that the wrong scaling exceeds the row's allowance, and demonstrate that rejection. Scale 2 is already used
by the partial H2/H4 fixture at WIP `7911c75b`; that supports the approach but does not settle all five rows or
Chrome. This remains a proof-strengthening suggestion, not a new condition for design approval.

## Request and inspection scope

Read all four [accepted planner briefs](../../plans/ATMOSPHERE/ACCEPTED.json), whole and oldest first,
and the [project brief](../../plans/BRIEF.md) before assessing the revision. The original commission calls for
a continuous road to V1 and readiness for the sea/cloud modules. ATMOSPHERE must provide one shared sky, true
Sun, transmittance, aerial and reflection handoff; apply cloud optical depth once to each direct-Sun receiver;
and close the sky/time-of-day review on Metal and hardware Chrome. The two retry briefs require the previous
review gaps to be closed. The fourth brief specifically asks for a planner decision on the measured H2 Metal
precision miss. It does not authorize weakening the preserved physical referees or declaring the visuals done.

The inspection list was: resolve prior findings; read the complete current design, plan and done-when items;
compare every line of the precision revision; inspect the saved failure and its measurement path; challenge
the wider bound with wrong lookups, low contrast and small values; check decode units and read counts against
the existing sampler; and trace the change into Chrome, M6 preservation and visual review.

Read README, ORCHESTRATE, the relevant module-list interfaces and build order, C08, the requested review skill,
Thrum's review rules and both graded-example sets. No plans glossary is present. This is a scoped follow-up;
M1–M4 and the future sea/cloud mechanisms are not being redesigned or newly qualified.

## Why the precision revision is acceptable

The blocker is not present as a file on this integrated tip. I read it directly from its recorded WIP commit:
`git show 7911c75b:docs/evidence/ATMOSPHERE/m5-h2-blocker.md`, together with the raw
`docs/evidence/ATMOSPHERE/m5-h2/handoff.log` and the relevant `products/tests/filtered.rs` and
`products/tests/observer.rs` source at that commit. The integrated
[planner record](log-entries.md#entry-3) accurately reports the pertinent values.

The precision observer writes `vec4<f32>` results to a storage buffer and reads them back as f32; an fp16
render target is not introducing the observed rounding. The controlled fields use the production chart and
sampler. The log reports 257 distinct high-contrast results, two adjacent half-float low-contrast results,
and a maximum GPU/reference address difference of 0.000006814 texels. Re-evaluating at the observed GPU
address does not explain the low-contrast discrepancy. This supports a stored-format result-precision
allowance for this target. It does not qualify every query, prove universal nearest rounding, or qualify Chrome.

I independently recomputed the saved failing comparisons using `Q = 2^-14` after decode:

| Saved comparison | Absolute error | Previous allowance | Revised allowance |
|---|---:|---:|---:|
| Live chart, green | 0.0000161103 | 0.0000123132 | 0.0000733484 |
| Live chart, blue | 0.0000288146 | 0.0000238544 | 0.0000848895 |
| Adjacent-value midpoint | 0.0000305176 | 0.0000025330 | 0.0000635681 |

All three now pass for the reason the planner gives. `Q` is not counted twice for the two aerial slices:
those samples are combined with nonnegative weights summing to one, so their weighted rounding error is
bounded by the largest contributing step. Separately decoded solar and night terms each contribute a step,
as the revised rule explicitly states.

The unchanged [aerial sampler](../../../crates/thrum-render/src/synthetic_aerial.wgsl) reads two layers of
each of two textures, then blends by physical distance. Thus the four filtered reads versus sixteen explicit
texel loads described by the alternatives are source-level operation counts, not measured speedup or bandwidth
claims. Retaining the current path is a reasonable cost decision while M6 still judges gradients. Filtering
32-bit float textures also requires the optional feature named in the
[WebGPU specification](https://www.w3.org/TR/webgpu/#float32-filterable); it cannot be assumed by the portable recipe.

The new allowance remains a target qualification gate. A further correct-lookup miss returns to planning,
and Chrome still compares against the independent reference, never Metal output. No P1–P11 bound changes.

## Independent challenge of the wider bound

I evaluated a possible repeating known field in Python without writing any extra files. For integer texel
coordinates, use `v_c = (1 + dot(c, coordinate mod 2)) / 8`. Use coefficient vectors `(1,1)`, `(1,2)`,
`(2,1)` for sky RGB and `(1,1,1)`, `(1,2,3)`, `(3,1,2)` for aerial RGB. Every stored value is exactly
representable in half-float; values stay between 0 and 1, so the aerial field can also represent transmission.
This is an arithmetic feasibility witness, not a prescribed replacement implementation.

Enumerate every cell parity and all quarter, half and three-quarter weights on every axis. Compare the three
specified wrong sky lookups and three wrong aerial lookups with the exact blend, using the new per-channel
bound. Nearest ties round upward. Also allow each wrong result a full `Q` of rounding toward the reference
before testing rejection.

| Field | Wrong-lookup comparisons | Minimum neighbour difference / Q | Smallest rejecting-channel error / allowance |
|---|---:|---:|---:|
| Sky, all four cell parities | 108 | 256 | 2.5189 |
| Aerial, all eight cell parities | 648 | 256 | 2.5908 |

Every wrong lookup still fails. This includes descending cells and cancellation between axes; it does not
rely on testing only a rising ramp. The 32-step minimum alone is not a substitute for these controls: the
actual builder-selected field must satisfy the mandatory rejection at every query. GPU addressing, chart
boundaries, both Sun azimuths, depth endpoints and the 32 km clamp still require the planned target runs.

## Done-when assessment

Only M5.2's reviewer question changed in this revision. I read all sixteen current items and checked their
proof order and plausible wrong substitutes. The following are design assessments, not milestone passes.

| Item | Can it be judged when built? |
|---|---|
| M5.1 | Yes: one producer, replacement/profile epoch changes, time-scrub stability and true-Sun continuity are explicit. |
| M5.2 | Yes: the revised formula and field separation are explicit; independent live/known-field references and mandatory wrong-lookup failures prevent parity with a broken forward path from qualifying the handoff. |
| M5.3 | Yes: the same H rows and negative controls run on hardware Chrome; missing or unresolved rows fail. It inherits the revised formula through the design. |
| M5.4 | Yes: base/current byte comparison, same-build control, negative control and source/work inspection can establish the absent-provider path. |
| M5.5 | Yes: the Chrome off-parity arm uses one session and the same strict comparison. |
| M5.6 | Yes: uniform and spatially varying optical depth distinguish once-only direct attenuation and per-sample volume lookup from final-colour darkening or a volume-centre substitute. |
| M5.7 | Yes: noon/low-Sun motion, receiver boundaries, geometric shadow and timed drift are observable in the Mac engine capture. |
| M5.8 | Yes: the same views and motion are required in Chrome. |
| M5.9 | Yes: paired costs and per-pipeline capability counts are measurable without an invented timing gate. |
| M5.10 | Yes: Chrome records its own paired costs and creates every counted pipeline. |
| M6.1 | Yes: after the declared TAAU dependency, fixed stops and realtime motion expose the named defects; held sunset and Ground E remain included. |
| M6.2 | Yes: Chrome repeats that visual review with reconstruction on. |
| M6.3 | Yes: final-build P/H references and bounds are preserved from M5; the 198-row report is explicitly not a new accuracy qualification. |
| M6.4 | Yes: the final Chrome H rows inherit the revised H2/H4 allowance and rejection controls. |
| M6.5 | Yes: a fresh matched starting-commit rerun and hitch diagnosis are reviewable. |
| M6.6 | Yes: the ledger must separate implementation, correctness, visuals and devices and retain named exclusions. |

The diagnostic provider and handoff fixtures are built within M5; their checks do not require the future SEA
or CLOUDS implementations. M6 follows M5 and APPLE-PRESENTATION M3. The narrower physical-accuracy scope and
parked crossing remain explicit assumptions, not new omissions introduced by this revision.

## Evidence and limits

No GPU run, image capture or motion inspection was performed. The saved WIP evidence was audited, not
rerun or promoted to proof of this integrated commit. This approval does not change M5's blocked state,
approve its partial implementation, or close G3s. No capture folder was needed.

Validation: `./dev check-docs` passed (667 Markdown files, 3,223 local links, 30 module records, crate policy
and current design hashes). `git diff --check` passed. Only this review file is changed; no acceptance fields,
milestone state or implementation files were edited, and no commit was made.

DESIGN OK
