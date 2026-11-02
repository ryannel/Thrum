# Independent review — proposed cloud-light correction

2 November 2026. Reviewed proposed-cloud-light-correction.md (SHA-256
7c675a4b23a2d9f5b693e1c144ce030d4b62fc5dfbdbb427342fd16a8899f7c3), actual Cu source/cache/transport assembly,
canonical optical/source/group/low-layer contracts and the retained five-family commission. Root supplied the exact
earlier user wording permitting reduced physical requirements and choosing production style/coherent motion; the
later commission makes weather and five families required again. That relaxes fidelity, not consumer coherence or
appearance/motion. No code/canonical edit, GPU run or new numeric experiment was performed.

**Recommendation:** this is a reasonable *proposed liquid-Cu comparison profile*. Its source is concrete, finite and
camera-independent, with an explicit table cook and sensible falsifying images. Two boundary statements need correction
before broader adoption. The slab fit, source grouping and brightness prediction do not establish any family pass.
Do not require a converged scattering solver or exact arbitrary-geometry conservation to attempt the scoped comparison.

## Concrete findings

### LIGHT-R1 — P2: reference-column M cannot directly supply the declared world-vertical body thickness

The proposal defines T_body as an integral over an owner's vertical column, then says Sc/St can supply it directly
from M after the declared shear/frame transform. Canonical CLOUDS explicitly says M is a **reference material-column**
optical amount. Under height-dependent horizontal shear, a fixed world-vertical ray crosses different material columns.
A simple frame transform or scalar Jacobian therefore cannot turn M at one material column into the required integral.
Likewise, “vertical” must mean the same declared axis in the T_body map and mu0=abs(sun dot owner_up); a rotated source
or spherical patch cannot silently use world Y for one and local radial up for the other.

**Correction:** for the first unrotated Cu fixture, explicitly pin the common column/up axis and integrate the unioned
owner field with world-length weights. For sheared Sc/St, integrate the transformed coefficient query along the declared
owner-up column, or explicitly change the descriptor to a reference-column approximation and qualify/recalibrate it as
such. Reuse M only in the named no-shear/aligned regime where the equality holds. Keep all views/consumers on the same
column definition, support and source generation. This does not require atmospheric dynamics or a new solver.

**Witness:** a localized column patch under two height-dependent translations yields a different world-vertical
integral from an unsheared material-column M. Compare the actual map with direct integration of the same admitted
sigma field; require invariance under common rigid rotation when owner-up and light transform together.

**Scope:** not a fatal defect in the proposed unrotated single-Cu experiment; it is a concrete compatibility fault
in the broader “direct M reuse” claim, which should not be carried into canonical layer integration.

### LIGHT-R2 — P2: the promised ice/thin witness needs explicit phase/profile dispatch

The selected primary and table use albedo1/HG(.86), and the text correctly requires recooking after phase/albedo changes.
The later witness nevertheless promises exact thin-liquid/ice primary without specifying what a mixed or ice consumer
actually selects. Thrum's coefficient contract retains distinct liquid/ice scattering weights/phases. Applying the
liquid primary/table unconditionally to an ice filament would violate that contract; shared lineage is not permission
to replace ice phase with liquid phase or duplicate extinction.

**Correction:** label the initial fitted bank liquid-Cu-only. Define non-liquid dispatch now: retain the admitted ice
primary/old profile or disable the new bank for ice until a separately keyed table is qualified. If mixed-family use is
selected, specify per-constituent phase/albedo/table selection and source summation under the existing condensate
partition, while ordinary all-source tau is shared. Do not call a liquid-only fit a qualified ice closure.

**Witness:** a thin isolated ice ribbon and a liquid/ice overlap keep their declared primary phases as the extra bank
vanishes. Changing a non-overlapping thick liquid owner cannot change the local ice A descriptor, except through the
shared incident-light path. No arbitrary phase library or ice microphysics is necessary.

## What checks analytically and structurally

**Generated versus escaped flux:** the proposal is right to distinguish them. Integrating all scattering-event source
counts repeated redistribution, so exceeding one incident-beam unit there is not alone an energy error. Final escaped
power in a nonemissive reference experiment is a different observable. The previous passive generated-source constraint
was sufficient but unnecessarily restrictive for a production cloud approximation. Removing it does not itself justify
an unbounded escaped result; the proposal preserves that distinction rather than claiming a physical solver.

For a homogeneous reference slab, the written U and D factors follow from integrating ordinary view extinction against
the reduced-depth source. Multiplying emergent radiance by outgoing cosine and dividing by incident horizontal flux
produces the stated factors. The isotropic semi-infinite result also follows from one half times the integral of
mu/(mu0+kappa*mu). Thus F0+AF1 is a linear escaped-flux account for that named slab, and the finite cap is meaningful.
This verifies the relationships, not the proposal's numerical table samples or an arbitrary-cloud energy certificate.

**Consumer ownership:** ordinary sigma, primary direct shadows and all-cloud Sun tau remain unchanged. The new term is
source-side, isotropic and shared by main, independent reflected and final environment rays; it does not alter receiver
Beer shadows or apply a private display gain. The existing finite A0 DAG remains acyclic. Same-owner Cu lobes must be
unioned before column generation, and mixed-source contributions must add once. Table/column generations and keys are
explicit, as are main-hidden retained consumers. No missing core producer was found for the single liquid-Cu path.

**Bounds and costs:** the proposed A<3.724, phase peak<=8, E<=120000 and external ambient<=960000 imply a source ratio
below 120000*(8+3.724/(4*pi))+960000, which is below2e6 in the stated units. Updating the existing source exit bound is
necessary and identified. The proposed LUT sizes are consistent with R32F storage; the 128² column map and ~128³ visits
are real additional per-unique-owner/age work, already acknowledged. No runtime performance claim follows. Source/group
support changes, column/table replacement overlap and reflection-only preparation must remain counted. A whole-source
column recipe is not automatically shareable merely because two events share a master.

## Legitimate unrun qualification, not missing design or new physical requirements

- A finite puff, deep underside, isolated thin ribbon, low-Sun rim, thick deck and mixed layers can defeat a vertically
  calibrated slab proxy. The proposal says so. Slab scalar correctness cannot close those appearance gates.
- A thin detached remnant can share an owner-up column with a thick remaining parent, especially during Cu breakup.
  T_body may then brighten the remnant like a thick body. Include that existing lifecycle case in visual qualification;
  a local chord/component descriptor is a possible correction if it actually fails, not a required new mechanism now.
- The 35x source-factor prediction is about a specified tau/angle/column query, not a measured image improvement.
  Keep density, Sun/sky and global display fixed for comparison. A brighter white balloon remains a shape failure.
- An empirical table can be a useful appearance experiment without interval certification. The proposal explicitly
  distinguishes off-grid/doubled-quadrature checks from a uniform error certificate. Preserve that label; do not make
  interval enclosures or exact arbitrary-cloud conservation an unsolicited gate for a production-style comparison.
- T_body quantization and A interpolation near the min/cap kink need a measured error account. R16F columns are a
  candidate, with refine/direct/R32 fallback already stated. All-light-off, grazing/equal-slope and source/time changes
  remain necessary finite correctness cases. Below-horizon abs(mu0) never bypasses actual planet/light visibility.
- Overcast surface diffuse light must eventually use the final clouded field; the current constant receiver term
  cannot qualify it. The proposal discloses that seam and does not commission water, lightning rendering or climate.

## Primary-source check and review limits

Independently inspected the text of [Wrenninge2015, equation1 and Method](https://history.siggraph.org/wp-content/uploads/2022/10/2015-Talks-Wrenninge_Art-Directable-Multiple-Volumetric-Scattering.pdf):
it distinguishes contrast control from spatial diffusion and uses amplitude/phase/reduced-extinction banks. It does
not disclose this fitted one-bank LUT. Also inspected [Epic4.27, Light Multiple Scattering and Directional Light Interactions](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-clouds?application_version=4.27):
its art controls and source compensation support the proposal's bounded production-approximation rationale, not these
coefficients or an energy certificate. No new conference recording, Patry simulation or full Frostbite formula was
independently verified here.

After correcting LIGHT-R1/R2's scope/dispatch statements, no other structural blocker was found for the narrow liquid-Cu
comparison. Treat its LUT error/certification, finite-cloud behaviour, actual brightness/contrast, independent consumers,
motion and complete workload as separate unrun witnesses. Do not promote the proposal by association with its author's
other accepted work, and do not discard the source morphology rejection when lighting becomes more attractive.
