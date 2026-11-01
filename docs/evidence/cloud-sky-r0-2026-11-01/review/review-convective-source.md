# Independent Cu source and CPU-test review

1 November 2026. Read `convective.js`, `test_convective.mjs` and the current preview JS/WGSL/cache integration. Ran only `node scripts/cloud_sky_proof/test_convective.mjs` and small independent CPU constructor/frame probes. No GPU execution, canonical edits, implementation edits or commits. Broadsides was untouched.

Producer SHA-256 reviewed: `8ebb8b2f44036c61f7be78c2f1440def9902ad508392175bd4eb7d86a7e810d1`; tests `fc6eec46d98a18830662d234cb0222211dad49b4d7a5b6bcc8813947b3083787`. Preview was actively being built; findings below identify the inspected behavior rather than certifying a stable preview release.

## CS01 — P2: finite accepted birth time can publish infinite GPU parameters

**Location:** `convective.js` pose at64–70 and `cuFrame` at178–195; birth-edge coverage in `test_convective.mjs`.

`cuFrame` admits every finite source time. Radius is finalRadius ×smooth(birth,birth+grow,time), then its reciprocal is written directly into Float32Array. The root radius approaches zero quadratically at birth. A valid finite time therefore overflows the packed reciprocal while the root remains active.

Confirmed CPU reproduction:

```js
const f = cuFrame(createCuRecipe(), 1e-20);
// header source time ≈9.99999968e-21; root active=1
// f.slice(8,11) => [Infinity, Infinity, Infinity]
```

The canonical CPU query remains finite, so the claimed valid packed source is no longer equivalent. At a point rounding to its centre, WGSL `delta*invR` can become0×Infinity/NaN. Field-grid samples far outside the tiny root may skip it; that does not make nonfinite admitted source parameters safe. The current tests use0 and much later samples, so they miss this boundary.

**Correction:** enforce a stated numerical birth policy before upload. Either reject unrepresentable packed frames with an explicit unsupported/admission error, or omit unresolved tiny support only under a conservative optical error bound. Do not silently clamp inverse radii to change geometry. Validate finiteness of the complete packed frame. Add positive near-zero and near-cohort-birth probes, including points at the rounded centre; verify that packed/CPU paths agree within the declared policy or both report unsupported. This is a numerical edge correction, not a request to remove genuine formation.

## CS02 — P2: changing playback rate can rewind the shared source clock

**Location:** inspected `cu-preview.js` `currentAge()` and rate-change handler, approximately20 and145.

`currentAge` computes source time from the wall-time anchor, while `age` stores the most recently rendered source sample. On a rate change, the handler resets `anchorAge=age`, discarding elapsed source time since that rendered sample. During accelerated playback the discontinuity can be seconds. Example: anchor650 at64×, current wall time1s gives714; a last rendered sample at0.96s is711.44. The handler anchors to711.44, rewinding2.56 source seconds.

Reading `currentAge()` inside the existing onchange handler without another change is insufficient: the DOM select already contains the **new** rate, so that function would retroactively apply the new rate to the old interval.

**Correction:** retain the committed playback rate independently of the DOM. On change, evaluate the old clock at one captured wall timestamp using the old rate, then update source anchor/wall anchor/stored rate together. Add a pure CPU clock-continuity test with a delayed rendered frame and1×/16×/64× changes, pause/resume and seek. Preserve one latched source age for field, light caches, receiver and mirror.

## Actual checked scope and remaining qualification

The existing source tests pass. Their observed maximum CPU canonical versus independently decoded Float32-parameter/double-arithmetic reference discrepancy is `5.367832468620992e-8 m^-1` over the sampled fixed-seed cases. The BigInt hash oracle usefully checks wrapping lattice arithmetic. This is **not** actual WGSL arithmetic or field/ray/image qualification.

Within that tested domain, no further concrete contradiction was found in finite retirement, base/support, local mature birth/loss, world-unit conversion or the inspected source-only mixture:

- Root/cohorts have finite compact retirement; root retires at1730s, before the1800s envelope. Mature intervals contain new cohorts and retiring old ones; sampled local births and losses are present.
- Children start inside a filled surviving parent for the tested recipe/seed. Local metre bounds conservatively enclose interpolated centres and bounded final radii; the y=0 base mask and12m feather match CPU/WGSL intent. max-union represents one condensate without summing its cohort overlaps.
- Preview field coordinates are source-local; scene placement adds the same explicit world trajectory to view/light/reflection queries. Pose generation and shader drying must continue to share the same packed-header time; `canonicalSigma` currently states this precondition rather than rejecting mismatched query ages.
- The inspected preview coefficients `.7,.1,.0175,.00375` equal the selected w*b branches; ordinary extinction is unchanged. HG cosine `dot(viewRay,sunDirection)` has the expected forward-looking sign. Surface direct attenuation occurs once. Reflected far leg is integrated from the interface, then the near pair is applied once. The constant terminal and bounded vacuum fixture do not contain the Sun disk, so its stated1.92e6 source exit bound is conservatively larger than these fixture inputs.

The preview's64³ tau interpolation,8³/five-axis ambient and constant diffuse receiver term remain explicitly approximate bounded-fixture products. No spherical-air/horizon, actual water, full clouded surface-environment, ordinary/accelerated native motion or CF01 appearance pass follows from this review. In particular the CPU packed reference does not prove the actual field preserves early small structure, parent/daughter silhouette quality or temporal continuity. Correct the two concrete issues and retain the existing explicit unproven scope for subsequent GPU/native witnesses.
