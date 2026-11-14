# CLOUDS design review

**Design approved, with two Suggestions and no Must Fix findings.** The revised design answers the human's
request for a reusable cloud system chosen for Thrum, with Broadsides available as a resource. Both Must Fix
findings from the previous review are resolved. This approves the design and its proof order, not cloud
appearance, device performance or an implementation.

Reviewed commit: `79d39ed1e247cc941502b89938c4debe48bf500e`, 14 November 2026. Previous review:
`docs/evidence/CLOUDS/design-review.md` at `f2418fb348cd19faf73df292cee640ee83d97dfe`.

## Previous findings

### shadow-only-density-mismatch — resolved

The missing resource is now present. [DESIGN.md](../../plans/CLOUDS/DESIGN.md#placed-volumes), lines 546–573,
requires the Sun's path to read the leased slab wherever a placed cell has volume-form weight. The slot and
weight depend on the camera, placed range and lease, never on whether Sky is declared. The capability table
and shadow-only definition, lines 581–600, give Shapes its lease in every rendering profile. Both profiles
use the column form when the lease is absent or the volume form has no weight. The headless query still
needs only footprints. The plan explicitly records the library memory retained in shadow-only mode.

This resolves the actual counterexample: equal vertical footprints do not imply equal oblique paths, but
the shadow evaluator no longer tries to recover those paths from footprints. M1.15 requires independent
voxel integrals to distinguish the equal-footprint pair. M6.2 requires the GPU result to follow the volume
at weight 1, the column form at 0 and their blend between, with a nonuniform shape at two Sun elevations.
Its reviewer question requires the footprint-only result to fall outside the accepted bound at the volume
probes. M6.12 then requires identical bytes with and without Sky at the same weights, including the absent
lease case. M8.9 preserves that obligation when World light is added and checks resource creation too.
Together these reject both the original missing-volume implementation and the substitute that uses the
footprint in both profiles merely to make their bytes agree.

I read and reran [the planner's revised proof](design-proof/second_revision_proof.py). Its equal-footprint
pair gives optical depths 3.959767 and 0.565681 from the two volumes, versus 1.414203 from the common column
form. [The rerun](design-review/second-revision-proof-rerun.txt) also reproduces the wider placed-shape
results. These are CPU feasibility results; neither this script nor this review proves GPU byte equality.
That remains M6.12's job. The finite-column approximation has explicit error and visual checks; see S1
below for how its proof should be described.

### sun-disc-phase-normalization — resolved

[Cloud light](../../plans/CLOUDS/DESIGN.md#cloud-light), lines 409–440, replaces denominator widening with an
average of the entire first-order phase over the Sun's disc. For each incident direction the normalized
phase integrates to one over outgoing directions. Averaging these integrals with normalized disc weights
therefore also gives one. Rotational symmetry gives the first moment as the point-source mean cosine times
`(1 + cos ρ)/2`. At the disc centre the result is the point-source phase integrated over the disc, divided
by the disc's solid angle. These are properties of the selected mechanism, not a renormalization hiding
the old angular error. The point-source mixture and fit agree with sections 2 and 3.1 of the primary
[Jendersie–d’Eon supplemental](https://research.nvidia.com/labs/rtr/approximate-mie/publications/approximate-mie-supplemental.pdf).
The finite-disc convolution is the design's additional operation.

The rerun gives unit integral to six decimal places at 5, 10, 20, 40 and 50 µm, and the predicted first
moment. M1.8 now proves the point-source model separately; M1.14 proves the disc average and the table read
in f32, including the forward peak and a blend of type tables. M5.1 reads that same table and includes 0°
and 0.25° radiance probes. The old widened lobe cannot pass the normalization requirement, and simply
returning the point-source phase cannot pass the centre-of-disc requirement.

I additionally checked the proposed 1,025-entry table with simulated f32 storage, coordinates and linear
interpolation, independently integrating that lookup with composite Simpson quadrature. At 5, 20 and
50 µm its integrals are 1.000019455, 1.000078464 and 1.000133774; the largest relative error at M1.14's
angle probes is 0.001401. A 37/63 blend of the endpoint tables integrates to 1.000091476. Doubling the
integration resolution leaves those reported integrals unchanged. The [check](design-review/phase-table-check.py)
and [output](design-review/phase-table-check.txt) show that f32 does not invalidate the proposed table.
This check reuses the inspected planner disc quadrature; it is independent of its table-integration code,
not an independent Mie solver or a GPU test.

### Previous Suggestion S1 — taken and resolved

M5.1 now scopes local-reach invariance to a fixture without detail erosion, where full and coarse density
agree. That tests double counting without requiring two deliberately different density representations
to have the same integral. The actual approximation is still checked and visually judged elsewhere.

## Fit to the human's request

I read all five tasks in `RECORD.json` under `briefs`, oldest first, and the approved project brief before
judging the design. The project brief governs the purpose, V1 exit and performance policy. The later owner
words permit reuse and require first-principles selection; the lead's interpretations remain labelled
assumptions. The water task concerns the cohesive water system and explains the transfer of the receiver
and example to WATERS; it does not implicitly commission precipitation or inland-water simulation inside
CLOUDS.

The design retains the commissioned K1–K7 experiences: recognizable weather through the horizon, cloud
lighting through the day, shade shared by all receivers, structural life and world-space drift, accepted
placed shapes and growth, temporal stability, and visitor controls on Mac and Chrome. Its common density
and optional capabilities serve procedural, shape-only and mixed skies without making Broadsides' old
assembly a constraint. Game-directed shapes remain an explicitly disclosed addition. Reuse is recorded
with provenance and does not transfer Broadsides' visual acceptance to Thrum's lighting.

The CPU weather remains authoritative; GPU products are derived. G3s still owns sky, Sun and receiver
attenuation, EFFECT-GRAPH owns execution and state commit, CONTENT-LOOP owns volume residency, and the
engine owns reconstruction. Keeping volume leases for shape shade respects this ownership and does not
require a new content interface. The V1 exit remains unchanged behind off-by-default selections. Numeric
costs remain measured baselines and tripwires, as the approved brief requires. The camera/geometry range,
formation-demo deferral and other exclusions remain disclosed assumptions, rather than hidden reductions
of an acceptance item.

## Changed-item coverage and reviewability

I read the complete current design and plan, all 116 done-when items, and the design/item diff from
`f2418fb348cd`. Every changed or added item named in this review's task is covered below. These are design
assessments; no milestone verdict was changed.

| Changed scope | Assessment with preceding milestones and declared dependencies available |
|---|---|
| M1.8 | Point-source optics are a CPU-only proof, distinct from finite-disc rendering. The expected values have independent sources. |
| M1.14 | The rendered phase/table can be judged without a renderer by reproducing its f32 read. Normalization, first moment, peak and type blending reject the prior mechanism. |
| M1.15 | CPU voxel fixtures can distinguish identical footprints with different oblique paths before any cooked product or renderer exists. |
| M5.1 | M1 supplies the disc reference/table, and M3–M4 supply rendering. Near-Sun probes and the corrected local-reach fixture are achievable. |
| M6.2 | CONTENT-LOOP M1 supplies the lease. M1's reference and M2's Sun-path machinery exist. The new M6-specific bound recognizes placed-shape filtering without permitting a footprint-only substitute. |
| M6.12 | Both profiles can hold the same volume lease after M6's placed-volume work. Resident/absent and intermediate-weight cases make byte equality meaningful. |
| M8.9 | The capabilities and World light exist by this milestone. Explicitly retaining the lease closes the old resource contradiction; creation counts still reject hidden-but-running work. |
| M11 dependency/quality bar | The dependency is now WATERS M8. The quality bar is unchanged. WATERS supplies water depth, reflection and direct-Sun shade before cloud integration, including the craft used by M11.4. |
| M11.1 | The selection now targets `examples/water` and `water-scene`, consistent with the module list's ruling 5. Off-selection equality and public-interface use remain. See S2 for one stale owner name. |
| M11.2 | The test package changes to `water-scene`; water depth, single attenuation and environment reflection obligations remain unchanged and agree with WATERS' optics. |
| M11.5 | The browser command changes to `check_water_browser.mjs`, matching WATERS' example. First-frame validation and the noon visual inspection remain required. |

The remaining items keep the independent appearance and motion inspections, CPU/GPU conformance, resource
and lifecycle checks, and later reconstruction/governor qualification. A static panorama, unrelated shadow
texture, density that only fades instead of developing structure, or heavy temporal history hiding motion
cannot satisfy the combined requirements. No additional Must Fix was established.

## Suggestions

### S1. Suggestion — state the placed-shape proof's sampling and reference scale — verified

Part H's [implementation](design-proof/second_revision_proof.py), lines 274–287, uses **four density samples
per stored slice**, not one. At lines 348–349 its reference is a seven-ray average of transmittance over a
column's width, not the point ray alone. The design's results at lines 685–692 summarize this as agreement
with the volume's own path, while the cost table at line 759 counts slices as samples (6.3 million for a
full three-level fill). Clarify these two qualifications beside the reported error and cost: which spatial
quantity the error compares, and how many integration samples produce it. Stored slices and density reads
are different counts.

This is not a build blocker: the script states its actual reference, the cost figures are estimates, and
M2/M6 already require measured work and conformance with filtering explained. The mechanism remains
feasible. The clarification prevents the builder from treating the reported accuracy as demonstrated at
one read per slice, or treating a filtered reference as a pointwise bound.

### S2. Suggestion — name the migrated water owner in M11.1 — verified

[The M11.1 reviewer question](../../plans/CLOUDS/DONE.json), line 1227, still asks whether any item or test
of `SEA` changed. The example and optics now belong to `WATERS`. Name WATERS as the affected owner here
(keeping SEA protected too if useful). This is a stale inspection prompt, not an interface gap: the plan
already says to change nothing else of WATERS, and ARCHITECTURE ruling 5 protects the owning module's items.

## Evidence, limits and validation

I performed the reading and judgement myself, with no delegates. Alongside the module files and prior
review, I checked the relevant C02/C04/C06–C09/C12 contracts, module-list seam rulings, port decision,
CONTENT-LOOP lease semantics, EFFECT-GRAPH tick/interpolation rules, and WATERS' depth/optics design and M8
items. I read the planner's revision log after assessing the changes. This is a revision review with the
prior design and findings visible, not a blind first-principles study.

Commands run for numerical evidence:

- `python3 -B docs/evidence/CLOUDS/design-proof/second_revision_proof.py`
- `python3 -B docs/evidence/CLOUDS/design-review/phase-table-check.py`

Both completed successfully. I did not run GPU, Chrome, capture or device-performance tests, inspect a
running cloud renderer, reread Broadsides assets/code, or rerun the older field sketches. No visual,
performance or implementation maturity is promoted by this review. M12's existing governor/MetalFX and
attended-display requirements remain prerequisites to closing that milestone, not to approving the design.

Validation: `./dev check-docs` passed (829 Markdown files, module records, crate policy and current design
hashes); the [log](design-review/second-revision-check-docs.txt) records it. `git diff --check` passed.
Only this review and four new evidence files in `design-review/` are changed. Planner files and existing
evidence are left intact. The owned temporary Cargo output was removed, and the changes remain uncommitted
for the review script.

DESIGN OK
