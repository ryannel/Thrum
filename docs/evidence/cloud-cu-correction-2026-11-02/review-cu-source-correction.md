# Independent review — proposed Cu source correction

2 November 2026. Reviewed proposed-cu-source-correction.md, SHA-256
dc97c1fcb51cf840a3c549479e0369c1393155536f038cb9bf949dc1344cf6ae, against the actual failed Cu producer,
CF01/source/group/motion contracts and the already-inspected native candidate/comparator. No code, GPU, canonical
edit or commit. The proposal's author supplied new mechanisms; this is a different reader's challenge, not inherited
approval. Production-style authored geometry is allowed; neither fluid simulation nor mathematical cloud perfection
is required.

**Recommendation:** the surface hierarchy is a concrete, plausible replacement candidate, not a relabelled ten-ball
grammar. Its medium/fine exposed caps, spacing and physical-distance density operator can create geometry the old
producer did not contain. Before declaring it build-ready, close the terminal erosion-field and field-build/index
choices below. Keep the first witness on spheres and a rigid local wind frame unless the optional general cases are
separately defined. Native appearance remains an indispensable unrun gate: algebra does not rule out broccoli/foam.

## Findings

### SOURCE-R1 — P2: a bounded terminal delay is not a specified spatial erosion field or a fixed-width boundary

The proposed terminal operator is E_j=(r_final+w)*smoothstep(t_stop+delta_j(q), ..., t). Bounded delta and positive
T_erase do prove eventual extinction, but the proposal only describes delta/exposure verbally. An implementer still
must choose the spatial front that is supposed to create local, irregular breakup. Boundedness alone does not imply
continuity, a resolvable length scale or the stated 12m boundary thickness.

In particular d_j is a metre signed distance before erosion, whereas d_j-E_j(q,t) generally has gradient magnitude
other than one. Its smoothstep width is 12m in **field-value units**, not automatically 12m along the resulting surface
normal. A sharp spatial delay can create an unresolved front even when the original sphere ramp spans several cells.
This matters to the proposal's explicit resolution/optical visibility reasoning, not to a demand for exact physics.

**Correction:** select a concrete material-space delay/exposure field (a bounded plane/sector front with a stated
length scale is enough), its seed/pose/age inputs and valid ranges. State that the 12m true-distance interpretation
holds for the un-eroded source, then either bound the terminal front's spatial gradient/width or qualify a separate
minimum drying-front width against the same source cells. Do not introduce an unnecessary signed-distance solver.
For example, a declared bound |grad E|<1 would bound local widths between w/(1+|grad E|) and w/(1-|grad E|);
other explicit resolvable erosion regimes are acceptable. Ensure CPU and uploaded pose use the same chosen front.

**Witness:** fixed-world source samples across a mature boundary and across the moving terminal front show a finite,
resolved transition; every centre and detached remnant reaches zero by the recorded retirement without whole-source
alpha multiplication. The breakup image/motion still must look irregular rather than like clean shrinking buttons.

### SOURCE-R2 — P2: the selected 256-grid producer has no concrete conservative index/build path yet

The actual current canonicalSigma scans every cohort. The proposal correctly rejects that as an unexplained baseline
at 256live nodes x256³cells (~4.3billion primitive visits), but replaces it only with “cook hierarchy/bounds”, a target
of <=64local contributors and a backing-field shortcut. The brick/index producer, update rule and full-density proof
are not selected. Retaining the old shader loop and merely promising an index later would leave the proposal's cost
control unimplemented at the point where it matters.

**Correction:** choose the first finite field path before extending the renderer: brick dimensions, current-pose
sphere/box candidate construction, bounded list storage/overflow behavior and the exact predicate for a brick that is
already full backing density despite erosion. Bounds must include current attachment/roll trajectories and newly born
or vanished support; count candidates/visits and index rebuild work. A deliberately slow, clearly bounded offline
source-only reference is also legitimate, but label it as such rather than a viable recurring source build. No GPU
benchmark or optimal layout is required to settle this ownership/mechanism choice.

**Related primitive choice:** use the specified exact spherical distance first, or select a named calibrated coarse
primitive with an explicit error/cost bound. “Genuine ellipsoid distance” is not one multiply: the generic closest-point
problem needs an admitted algorithm/iterations and interior/degenerate handling. It must not appear unnoticed inside
billions of field samples. Spheres avoid this unnecessary uncertainty for the initial hierarchy witness.

**Witness:** candidate lists are conservative against a brute-force source oracle, including moving/eroding parents,
new caps and overlap; forced overflow fails visibly rather than dropping density. Count actual primitive visits and
show the full-interior shortcut cannot hide a cleared parent or unsupported child. Appearance and runtime cost remain
separate subsequent gates.

## Analytic checks that support retaining the candidate

- **Attachment/growth:** at g=0 the centre is d_attach inside the nominal parent surface; with r=0 it contributes
  no support. As g increases, protrusion is beta*r-d_attach*(1-g), so a child emerges from the existing body continuously.
  At full growth the outward cap is beta*r and the centre remains inward by (1-beta)*r. The condition d_attach>=w
  makes an un-eroded parent's birth site filled. For late births, admission must check the *actual eroded union*, not
  only nominal R, as the proposal's “filled surviving parent” rule implies.
- **Relief:** for equal caps on a planar backing surface, the stated midpoint height and peak-minus-valley formula
  follow directly from a sphere cross-section. s=1.6r gives .4r relief for beta=.55. Curvature changes the result;
  requiring actual curved-surface checks and r<=.45local curvature radius correctly prevents treating the planar
  equation as a universal guarantee. These are meaningful geometric controls absent from the failed flat grammar.
- **Union/optics:** max of positive-inside sphere distances followed by one density ramp gives one same-condensate
  source and no phantom bridges outside sphere supports. Uniform sigma0 avoids accidentally summing overlapping
  caps as extra extinction. A base mask still gives the condensation boundary. The local planar segment estimates
  ~.15tau for h14m/w12m and ~.018tau for w50m are consistent with integrating the smoothstep ramp; they do not predict
  full-body contrast or justify a lighting gain.
- **Finite lifetime:** r(t)<=r_final and terminal E=r_final+w eventually make every d_j-E negative, including the
  cohort centre. Zero-radius policy and float32-defined current poses preserve the existing finite CPU/upload seam.
  Pose rounding and birth-support omission remain separately stated approximations. No inverse growth map is needed.

## Scope/operating cases that must remain explicit

**Orphans:** a parent may cease feeding while existing children survive. That is allowed authored detrainment, not an
inherent bug. Keep the retired parent's cooked reference pose available for a child's declared remaining trajectory,
or cook the detached trajectory at the transition; never dereference deleted source state or snap to a new parent.
Admission during growth must test attachment against eroded density. Later detached beads are an appearance rejection,
not something to repair by deleting all descendants together. The retained1024-node finite history can support this
without making every retired node a live field contributor.

**Wind and local metrics:** the current preview has a rigid constant-wind event translation, not an implemented
height-shear map. For that rigid first case, metre distances and n(t) rolling tracks are coherent. A future nonrigid
height-wind map deforms distances: transporting a local12m ramp does not guarantee a world12m ramp. Keep that broader
metric/Jacobian validity explicit. Integrating angular speed U_tangent/R_parent(t), with child admission only after a
positive adequate parent radius, is preferable to phase=instantaneous frequency x total age. Cooked pose/basis tracks
avoid an inverse or undefined normal frame; shared wind must be applied once, not baked into both parent and child.

**CPU/field mapping:** evaluate the same current float32-defined centres/radii/orientations and metre-valued erosion
inputs in CPU and GPU queries. Per-level world pitch/extent must use the already-reviewed packed-field convention.
A finer texture cannot fix missing canonical medium caps. Conversely, once canonical relief exists, source filtering,
ray step and native image footprint must retain it before concluding the geometry failed.

**Cost/content:** the node and byte ceilings are admission proposals, not measurements. r60–130L1 and r28–40L2
cannot all be combined arbitrarily under the .45curvature rule; the cook must choose compatible parent/child scales.
Four gross variants must alter exposed plume topology, not only shuffle small caps. Do not pass morphology from a
node-count quota. No cheapness claim follows from compact graph bytes or a single selected frame.

**Source/light separation:** the body-column/light correction must consume this same unioned sigma and generation;
ordinary receiver shadows/reflected rays keep that source. New source geometry, ramp and uniform sigma0 change optical
depth and need recorded recipe identity. A support diagnostic can isolate geometry; a brighter rendered cloud is not
proof that this source produces missing relief. Preserve the prior prepared master and its honest lifecycle limits as
a comparator/replacement seam if the hierarchy still reads as primitive construction.

## Review evidence and remaining qualification

I independently checked the attachment, planar cross-section and optical-ramp reasoning above. No native hierarchy
image or motion exists in this review, so exposed coarse/medium/fine relief, natural shape, absence of popping,
mature replacement, local breakup, consumer consistency and cost remain unproven. The proposal's stop condition—one
controlled hierarchy attempt followed by the prepared-master seam if defining relief still fails—is reasonable.

The [Bouthors/Neyret author page](https://evasion.inrialpes.fr/Publications/2004/BN04/) currently exposes bibliographic
metadata but broken method/image includes; I did not independently inspect its full paper or figures. The proposal's
disclosed extract-only research support must remain labelled as such. Its specific dynamic/optical operators are
reviewed as authored synthesis, not a recovered production algorithm or a paper-established appearance guarantee.

Resolve SOURCE-R1/R2's concrete initial producer choices, then a bounded source-only/native comparison is justified.
No full fluid solver, exact all-cloud distance field, universal wind deformation or new numerical perfection target
is required. The family still cannot pass without the commissioned actual normal/accelerated motion and native views.
