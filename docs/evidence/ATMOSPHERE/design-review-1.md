# ATMOSPHERE design review

Reviewed integrated commit `4bbe44a433598650be47c1753c57da79f5d1df93`, 10 November 2026.
Result: **changes requested — two blocking findings**.

This is the first design review at this path. There is no previous round to resolve. The review covers the
10 November M5–M6 design and every item in `ATMOSPHERE-DONE.json`; M1–M4 supply historical constraints, not
new implementation claims. No other agents were launched.

## Request and scope

I read the complete sole entry in [the accepted planner briefs](../../design/modules/ATMOSPHERE-ACCEPTED.json),
then the build-plan purpose and exit, architecture brief and module list before comparing the proposed work.
The commission requires a buildable road to the existing V1 exit and foundations for finishing sea and clouds
in Thrum. For ATMOSPHERE this means:

- One shared sky, true Sun, light transmittance and environment-reflection product that independent consumers
  can actually use and whose handoff is qualified on Mac and Chrome.
- One application of cloud optical depth on direct sunlight, consistent across receiver kinds and independent
  of whether clouds are drawn; absent providers add no avoidable work.
- A reviewable close of sky, sunlight and aerial perspective with time controls, keeping correctness, appearance
  and device measurements separate.

The diagnostic optical-depth provider is justified foundation work, not an unauthorized cloud port. Waiting for
TAAU before the closing visual review is a coherent dependency. The M3/M4 exclusions are explicit assumptions
in the plan log, not hidden omissions; this review does not turn either parked outcome into a pass.

## Findings, worst first

### 1. Must Fix — numerical acceptance refers to a contract that is not defined for the claimed products

**Verified.** [M5.2–M5.3 and M6.3](../../design/modules/ATMOSPHERE-DONE.json), lines 24–35 and 135–138,
require matches to an independent f64 reference within “the tolerance the M1 referees already use” or “the
tolerances this design declares.” The design does not declare a product-by-product acceptance rule for these
claims. M1's criteria in [the plan](../../design/modules/ATMOSPHERE-PLAN.md), lines 16–17, qualify the
moonless component: 5% horizontal irradiance and 10% selected night rays and finite night in-scatter. Those
are not a specification for solar sky radiance, general aerial `(T,S)`, or twilight colour.

I traced the existing checks to distinguish a missing cross-reference from a missing acceptance decision:

- [The night referee](../../../crates/thrum-render/src/atmosphere/gpu/tests/reference/gpu/night.rs),
  lines 99–124, enforces the night bounds.
- [The Sun tests](../../../crates/thrum-render/src/atmosphere/gpu/tests/transmittance.rs), lines 235–250 and
  408–417, use different, explicitly scoped bounds: channel-dependent transmittance errors and a receiver
  tolerance of `0.05 * expected + 0.005`.
- [The all-orders solar comparison](../../../crates/thrum-render/src/atmosphere/gpu/tests/reference/gpu.rs)
  explicitly says at lines 1–2 that passing does not qualify physical sky accuracy. Its main test records the
  198 reference comparisons without imposing an agreement bound. Its separate twilight assertion at lines
  246–262 checks one fixed spectral row within 2.5%; that is not a general sky/aerial f64 agreement gate.

Consequently a green workspace check plus the existing night referees cannot establish the written M6.3
claim. A reviewer must invent the missing domain and thresholds, or accept a reporting-only solar comparison
as proof of agreement. M5.2 also leaves the builder to choose what “the” inherited tolerance means.

**Resolve before building:** identify the reference, quantity/units, supported cases, error metric and bound
for each lent lookup and each M6.3 assertion, with explicit Metal and Chrome execution. Distinguish preservation
of an existing qualified approximation from new physical-accuracy qualification. Existing scoped tests may be
reused; this finding does not require a new all-orders accuracy campaign. If broad solar accuracy remains open,
state that honestly and define the narrower correctness obligation that actually closes here. A test that only
reports disagreement must not stand in for that obligation.

### 2. Must Fix — the independent-consumer proof omits environment reflection

**Verified.** [The design](../../design/modules/ATMOSPHERE.md), lines 219–224, promises to lend reflection
products but enumerates only four lookups: Sun transmittance, sky radiance, incident diffuse and aerial transport.
M5.2 enumerates exactly those four, M5.3 repeats them, and no remaining item tests the reflection handoff.
Yet [the readiness table](../../design/modules/ARCHITECTURE.md#where-the-ports-readiness-conditions-are-met)
assigns qualification of environment-reflection products to M5, and [SEA S5](../../design/modules/SEA-BRIEF.md),
line 49, requires that product as the screen-space-reflection fallback.

The distinction matters in the current implementation. [The forward environment shader](../../../crates/thrum-render/src/synthetic_environment.wgsl),
lines 5–27, samples a roughness-filtered cubemap, uses the DFG, and decodes solar and night storage. A directional
sky lookup alone does not qualify that path. Today cooked surfaces inherit the whole forward fragment shader
through [SceneSurface](../../../crates/thrum-render/src/contribution.rs); the new commission specifically lets
other modules use their own pipelines without private chart-layout knowledge.

**Convincing substitute that passes:** implement all four tested lookups correctly while lending a stale or
incorrectly decoded reflection product to independent contributions. Keep the existing forward fragment path
unchanged. The off-parity checks, cloud-shade checks and four numerical lookups can all pass, while the first
sea consumer has the wrong reflected sky or must copy private storage/mip conventions.

**Resolve before building:** name the supported reflection operation and its decoding/roughness semantics in
the lent library, and include an independent contribution using it on Metal and Chrome in the acceptance items.
Exercise enough direction, roughness and day/night change to reject a constant, stale or incorrectly scaled
product. Reusing the existing reflection implementation and its scoped referees is sufficient; no new reflection
technique is requested.

### 3. Suggestion — make the cooked-volume integration explicit

**Verified implementation discrepancy; not an additional blocker.** The design's explanation at lines 229–231
assumes every direct-Sun receiver calls the same surface-transmittance function. The current
[cooked volume](../../../crates/thrum-asset-gpu/src/cooked.wgsl), lines 87–119, instead lights march samples from
`light.irradiance.rgb`; it does not call `sun_irradiance(world_position)`.

M5.6 already names this receiver and requires an isolating GPU check, so a faithful milestone review will catch
an omitted integration. Add its migration to the work list, including how the volume retains its existing
no-provider lighting while sharing the cloud factor. Include a spatially varying volume witness if convenient:
the uniform `ln(2)` control alone cannot expose sampling the field at the wrong world point. Account for the
lookup per lit march sample rather than assuming one read per output fragment.

### 4. Suggestion — specify the support of the borrowed lookups and cloud plane

**Verified underspecification.** The design names a validity region and receiver plane, but not the result outside
them or at a horizontal/below-horizon Sun. It also calls the aerial operation a “view segment” without saying
whether the segment must begin at the active camera. The existing
[aerial sampler](../../../crates/thrum-render/src/synthetic_aerial.wgsl), lines 14–37, is cumulative from that
camera and clamps distance to 32 km.

Document these existing limits and the fallback/refusal semantics at the public seam, including whether a
nonzero-start segment is supported. Clarify that a LUT *generation* change means resource/model replacement,
not ordinary time-driven regeneration of its contents. These are useful bounds for later CLOUDS planning;
they do not require adding multiple height bands or a general volume compositor to M5.

### 5. Suggestion — distinguish fresh paired costs from historical context

**Verified wording issue.** M6.4 asks for costs “paired against the M1 and M2 records.” Those records predate
TAAU and describe different builds and loads. Preserve them as historical context, but name a freshly rerun
comparator and matched configuration if the result is to be described as paired. The existing requirement to
investigate visible hitches is useful and should stay.

## Item-by-item design assessment

These assess whether the future proof is judgeable; they are not implementation verdicts.

| Item | Assessment |
|---|---|
| M5.1 | Judgeable: one producer, raw/apparent separation, profile/device changes and time scrub are checkable. Clarify generation terminology as above. |
| M5.2 | Blocked by findings 1 and 2: undefined numerical acceptance across the claimed products and missing reflection coverage. |
| M5.3 | Same blockers as M5.2. A hardware Chrome execution is the right separate proof. |
| M5.4 | Judgeable: frozen base, identical cooked input, byte comparison, repeat control, negative control and source/resource inspection reject an inert substitute. |
| M5.5 | Judgeable with the same proof on Chrome; a new checker is legitimate planned work. |
| M5.6 | Judgeable: direct-term isolation, zero/half transmission and all named receivers reject final-colour darkening and omission. Cooked-volume caveat above. |
| M5.7 | Judgeable: world drift, cross-receiver edges, retained sky light and geometric-shadow overlap are observable in the specified captures. |
| M5.8 | Judgeable: repeats the visual witnesses on Chrome rather than inferring portability. |
| M5.9 | Judgeable: costs are observations; per-pipeline counts against actual limits are concrete. Preserve both-target coverage despite the Mac command/target label. |
| M6.1 | Judgeable: named defects, held route, native-size crops and real-time motion after TAAU. The parked crossing exclusion must not erase the separate shipped Ground E terrain-lip finding or the held 17.4 h route stop. |
| M6.2 | Judgeable with the same inspection scope on Chrome. |
| M6.3 | Blocked by finding 1; “still match” cannot be inferred from the current reporting-only comparison. |
| M6.4 | Judgeable for recording costs and investigating hitches; improve comparator wording as suggested. |
| M6.5 | Judgeable: explicit known defects and separate implementation/correctness/visual/device claims prevent the close from certifying excluded work. |

## Evidence and limits

Read the requested skill, ORCHESTRATE profile, generic and project graded examples, Thrum review/performance
guidance, governing request and relevant contracts/consumers. No module-folder BRIEF or GLOSSARY and no previous
ATMOSPHERE design-review file were present. Inspected the complete current design and done-when file, the plan's
milestones and relevant historical rulings, and the source boundaries cited above. This is not a whole-module
code audit, a runtime qualification or an independent re-review of the old M1–M4 evidence.

No GPU runs, new visual captures or playback were needed to establish these document/interface findings.
Missing future scripts are not treated as implementation failures in a design review. Only this review file
was added; no plan, acceptance definition, implementation or milestone state was changed.

Validation: `./dev check-docs` passed (614 Markdown files, 2,794 local links, 30 module records, crate policy
and design hashes). All 13 links in this review resolve to existing local files; `git diff --check` passed.

DESIGN CHANGES: 2 blocking findings
