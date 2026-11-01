# Capability review closure

1 November 2026. Independent closure review of CAP-R1–CAP-R4 against the revised complete assembly, the
retained commission and my earlier independent inventory/comparison. Read the complete current source-evolution,
SKY-G/air/light/ambient, update/seam, scenario, worked-case and completion/proof sections; checked the affected
capability ledger and construction distinction. No canonical edits, implementation/GPU run, image/motion inspection
or performance measurement occurred.

Inspected SHA-256 identities:

- CLOUDS.md: `1100e7cc3b7109d263b4a903796578746a8b99a51f69157b36f3ec04fd6b9bad`
- CLOUD-CAPABILITIES.md: `4ef919ceb068e1514fb86dd10b1e59544b0dd88c86a00d9e9dcce8e2ccae99be`
- retained COMMISSION.md: `02fe7b2126d0c13c412388b532340ec2b6189fac2234fe5357ad62b243f35374`

## Dispositions

| Finding | Closure verification | Disposition |
| --- | --- | --- |
| CAP-R1 P1 finite ambient producer | CLOUDS 485–503 specifies ordinary directional depth → direct-only atmospheric A0 → fixed isotropic cloud ambient → final joint main/mirror/environment field. A0 integrates only Sun-lit air/fog source under combined extinction and already-built ordinary Sun visibility. Cloud source/ambient is excluded during A0; the ground is black; external space diffuse is named; the Sun disk is excluded from A0. No preintegrated clear-air image is subsequently attenuated again. Final clouded surface light never feeds A0/direct light. This supplies the missing finite DAG and once-only segment/source accounting. | **Closed at design level.** Ground bounce and recursive cloud interreflection are explicitly omitted approximations; their visual consequences remain qualification risks. |
| CAP-R2 P1 moving/sheared anvil inverse | CLOUDS 178–198 replaces the general inverse-radius query with launch-time/sector forward trajectories under shared wind/outflow, and a normalized compact 3D-kernel integral weighted by a normalized sector distribution. Moving/tilted outlet history belongs to each launched cohort; folds/multiple launches add their actual emitted amounts. A segment optimization must preserve the integral with its arclength Jacobian. Only a separately verified restricted radial case may use an inverse. | **Closed at design level.** Actual trajectory accuracy, quadrature convergence, persistent morphology and anvil appearance remain unproven. |
| CAP-R3 P2 usable consumer seams | CLOUDS 577–589 requires a nonzero authored precipitation interval and a nonempty lightning fixture. Forward traversal has a committed cursor; pause emits nothing; accelerated traversal must enumerate events even when a step spans their whole interval; seek creates a replay generation and resets stub cursors. Active snapshot state is separate from notifications; backwards inspection is explicitly not a new live-world strike. Culling cannot remove causes, and epoch/revision replacement retires obsolete cursors. | **Closed at design level.** The required stub results still need implementation, including offscreen/multiple-view/skip and replay witnesses. Unsupported flux may remain absent in other providers; the required fixture cannot use that absence as its pass. |
| CAP-R4 P2 completion scope | The inventory's completion audit and CLOUDS 701–708 now separately name standalone family/weather/full-sky optical proof, typed stubs/source facts, actual engine bindings and per-device qualification. A mirror is the required independent reflected-ray witness here; rough-water BRDF integration stays a separate engine-consumer gate. CL20/23/24/27 retain their engine obligations without requiring this cloud task to finish the whole runtime. | **Closed.** Do not promote engine maturity from the standalone proof. |

## Cross-checks and remaining limits

The ring source has correct optical dimensions: Q_sigma m²/s × normalized kernel m^-3 × ds yields sigma m^-1.
A normalized angular mixture of individually normalized spatial kernels remains normalized after arbitrary trajectory
translation; anisotropic kernels explicitly include their determinant. The selected compact spherical K3 constant was
also checked analytically: its radial integral is `(315/16)*(1/3-3/5+3/7-1/9)=1`. This verifies a mathematical claim,
not an implemented sampler or produced image. Emitted-cohort loss/core-junction partitioning remains an explicitly
authored optical approximation, without a new conservation claim.

The updated SKY-G domain now recognizes that incident/Sun/environment rays reach blockers beyond the view-population
worksheet. A canonical spherical virtual population, deduplicated face cells, conservative cross-face support and
finite-depth nonrecursive lighting requests replace fabricated clear values outside a local cache. This is compatible
with the resolved ambient DAG; actual all-consumer domain traversal and expanded workload still require proof.

The changed optical mixture is explicitly an appearance approximation with a disclosed thin-limit loss. I do not
recertify its energy derivation or numerical cost findings from other reviewers. Required thin-ice, thick-storm,
overcast diffuse-light and native-motion gates remain open; replacing the earlier closure does not inherit its visual
acceptance. The governing text preserves those gates and a replacement seam if the new candidate fails.

No new or carrying capability/producer blocker was found in the revised sections for the four reviewed findings.
This closes these design findings only. It does not declare the entire design numerically/visually qualified, any
new family accepted, the complete sky implemented, target cost met, or the user's requested goal complete. Proceed
through the stated correctness instrumentation and isolated-family proofs; preserve the best verified paused result.
