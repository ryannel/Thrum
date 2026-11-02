# Cu source-correction design closure

2 November 2026. Read the complete actual revised proposed-cu-source-correction.md, SHA-256
0048c0f5087c32f8091d5769b5c9aae9cf9c0a7c3ba32b40e508195bf0c5a2a6, against SOURCE-R1/R2 and the
previous independent review. No implementation, GPU, canonical edit or commit. This is different-reader
verification of the added operators, not acceptance based on the author's disposition message.

**SOURCE-R1 and SOURCE-R2 are closed at design level for the stated initial spherical/rigid-wind profile.**
No new structural blocker was found. Native appearance, motion, numerical admission and runtime cost remain open.

## SOURCE-R1 — selected local loss field and width

The proposal now supplies an explicit deterministic material-plane delay: a normalized seeded direction, final-radius
metric, clamped affine delay and per-level positive erase times. It no longer leaves breakup to an unspecified
“exposure field”. Its chosen span=0.32*T*R/(R+w) gives

`|grad E| <= (R+w)*(1.5/T)*span/(2R) = 0.24`.

The uploaded rounded direction/parameters must separately satisfy the stated <=0.25 admission inequality. Since
sphere distance has unit gradient at a regular point, the resulting F=d-E has gradient magnitude in[0.75,1.25]
there. The revised text correctly distinguishes field-value width from signed distance, limits the9.6–16m width
interpretation to regular branches and treats centres/union contacts/base-mask intersections through the direct
oracle. It no longer claims a universal12m final signed-distance boundary. Max union still contributes one condensate.

The bounded delay and E=R+w at final departure erase the centre and all support, rather than protecting a residual
core. Child birth checks now explicitly require actual eroded parent density; retired parents retain their immutable
reference trajectories/radii for existing children. New feeds stop without deleting every descendant. Rigid event
translation/yaw preserves metre metrics and applies wind once; height shear and anisotropic transforms are explicitly
excluded from this first profile. These close the earlier source/lifetime/metric ambiguity.

The .25 gradient admission, finite float32 pose agreement, <=4.8m largest grid pitch and .01m numeric guard are
**requirements to verify**, not results already measured. The selected radius/beta/pitch ranges are joint constraints:
for example r28m/beta0.45 at4.8m pitch would not meet a three-cell protrusion witness, so the cook must choose compatible
values or reject/refine that combination. This is an authored-profile admission issue, not a demand for more physics.

## SOURCE-R2 — concrete finite field/index producer

The proposal selects exact spheres throughout, removing the unresolved generic ellipsoid closest-point cost. It
chooses16³-voxel bricks over256³, giving4096bricks, and specifies current-pose AABB enumeration, conservative sphere/brick
intersection, deterministic count/prefix/fill passes, stable source-ID ordering, finite buffers and visible overflow.
Old positions/generations are not reused, and all field bricks are rewritten before publication. Needed historical
parent poses are distinct from live contributing density nodes.

The full-material shortcut is sound for its declared geometry: maximum sphere-centre distance over an AABB occurs
at a corner. Minimum material-plane delay bounds maximum erosion because erosion decreases as delay increases.
Thus a *single current live sphere* satisfying r-D_far-E_max>=w+margin certifies its material factor1 throughout the
brick; max union cannot lower it. The base mask is still evaluated per voxel. This avoids the invalid corner-by-corner
or un-eroded-parent shortcut. Conservative numeric padding must be applied consistently to candidate brick ranges,
intersection and predicates during implementation; a narrower unguarded broadphase cannot simply assume the guarded
fine test repairs omitted candidates.

Work and bytes are now actual selected admissions: <=64raw candidates/brick, <=262144IDs, <=524288intersection tests
across the two passes and <=134217728nonfull scalar visits. The latter is an independent stricter cap, not the claim
that4096bricks x64candidates always fits it; the stated4096*sum(nonfull_counts) predispatch check must enforce it.
The node/index byte accounting is coherent:1MiB IDs+64KiB metadata+16KiB current nodes is about1.08MiB per GPU input
generation, or2.16MiB doubled, before other counted staging/mirrors. The1024-node/8-knot raw-history example is640KiB.
No timing follows from these ceilings; brute-force evaluation is explicitly an offline oracle, not the hidden runtime.

## Remaining witnesses and allowed next step

The numerical guard requires actual packed-pose/field interval/reference qualification across the finite coordinate/
time ranges. Index equivalence must cover every sphere/brick boundary, moving/rolling caps, births, retirement,
eroded-parent counterexamples, parent-reference survival and forced capacity rejection. Those are meaningful
implementation checks now that their producer and rejection conditions are selected.

The hierarchy still can look like foam, broccoli or clean beads. The plane-loss field can still produce mechanical
breakup. A256³ field and bounded index can still miss desired cost. None is passed by this closure. Retain the planned
held source-only L0/L1/L2 and128/256/native comparisons, then normal/accelerated lifecycle and independent optical
consumers. Preserve the prepared master as the replacement seam if the defining geometry fails. The source-only
attempt is now justified within this profile without adding a fluid solver, universal signed-distance reconstruction
or a new perfection gate. Lighting, other families, full sky and complete-engine admission remain separate.
