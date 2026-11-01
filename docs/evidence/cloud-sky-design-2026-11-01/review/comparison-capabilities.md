# Independent capability comparison

1 November 2026. Compared the revised canonical inventory/design with the exact retained commission,
my prior independent-capabilities.md, Thrum architecture/C4/construction requirements and the repo-local
engine-module-design review workflow. Canonical files were read only; no implementation, GPU, motion or
benchmark run occurred. This review therefore concerns coverage and build-ready causal closure, not achieved quality.

Reviewed identities (SHA-256):

- CLOUD-CAPABILITIES.md: `8c093f3f606216102f23fcbdf30c2177d561ccd34cd33ca5a42ec75ab5e5477a`
- CLOUDS.md: `b2b7c7a38aed08bc305b9af87a9a74fb3f1affc7e57adc477ece9198fcaef459`
- retained COMMISSION.md: `02fe7b2126d0c13c412388b532340ec2b6189fac2234fe5357ad62b243f35374`

The five families, all four required weather sequences, full-sky distribution/horizon, independent consumer
bindings and progressive cost accounting are now explicit. The earlier optional-storm conflict is resolved in
these documents and the currently read BUILD-PLAN handoff. That is coverage, not proof. Two consequential source
links still need explicit closure before the draft should be treated as fully build-ready. Two further acceptance/
scope clarifications prevent vacuous seam passes and accidental enlargement into a whole-engine implementation task.

## Findings

### CAP-R1 — P1: external ambient has no unambiguous finite producer or transport domain

**Location:** CLOUDS.md 328–338 and 375–380, with extended-air producer 302–313.

The cloud source includes `L_external`; the later text says this samples the pristine sky through the combined
cloud/air/fog field and avoids feeding a clouded probe into itself. It does not define whether the pristine sky
is a terminal radiance, an already-integrated clear-air sky at the query point, or a source-term field. Nor does
it state which cloud source terms are included/excluded during the ambient quadrature. These interpretations
are not equivalent: attenuating an already integrated sky by a second air traversal counts air differently from
integrating atmospheric sources once; evaluating the full cloud source while computing its own ambient is circular.
The warning against feedback does not choose the finite computation that supplies `L_external`.

**Consequence:** another implementer still has to select a core illumination closure for thin ice, overlapping
layers and overcast. A visually plausible direct-light cloud proof cannot qualify the missing broad-light producer.
This is a design ambiguity, not evidence that a rendered artifact has already occurred.

**Concrete correction:** choose and document a finite dependency DAG for pristine sky, direct-only/higher-order
cloud source, external incident radiance and clouded surface environment. State ray origins/terminals, units,
medium/source terms and exclusions for every pass, and explain how air is included once. One possible solution is
a declared direct-source-only ambient quadrature followed by the final source evaluation; that is an example, not
a required algorithm. A deliberately attenuated-clear-sky approximation is also admissible if its omitted in-scatter,
path and validity are explicit and its required witnesses pass. Reject a hidden recursive full-source query.

**Closure witness:** no-cloud/no-fog reproduces the selected clear-air incident field; inserting an absorbing
cloud changes only the appropriate paths; two cloud layers/fog use one stated incident-light closure; overcast
surface/cloud ambient is computed without a read-before-write or iterative fixed point invented during coding.

### CAP-R2 — P1: anvil sampling inverse does not yet account for the selected emitter motion/shear

**Location:** CLOUDS.md 170–175, shared wind 101–109 and sky-frame mapping 296–298.

The radial producer recovers emission time as `t_emit=t-(r-r_0)/v_out` while also prescribing a moving storm,
upper-level drift/shear, tilted/asymmetric feeding and spherical source charts. The formula works in a declared
radial material frame with monotone constant-speed outflow. The draft does not define how a world query point
is pulled into that frame, how the moving outlet's history is accounted for, or how the asserted spreading
Jacobian changes after shear/chart deformation. Measuring Euclidean radius from the current moving head generally
does not recover the emission time of material emitted from earlier head positions under different velocity.
The phrase “within the admitted monotone branch” bounds intent but does not supply this missing map.

**Consequence:** the required cap-fed, persistent, sheared anvil can acquire a different age/amount in visible,
shadow and reflection queries, or the implementer must invent the necessary inverse/forward-cohort evaluation.
No claim is made that the cohort approach is unsuitable; this concerns the exact production link.

**Concrete correction:** define either (a) a precise admissible co-moving radial mapping with its inverse and
Jacobian, constraining winds/feeding where necessary, or (b) forward-emitted outflow cohorts with explicit support,
world transforms/amount and summation/partition query rules. Explain folds or multiple contributing cohorts if the
mapping ceases to be one-to-one. Keep the cap inlet nonsingular. This can remain an authored approximation.

**Closure witness:** translate the whole storm/wind together and obtain the translated same source; then change
wind direction during feeding and compare one tracked outflow cohort's world position/age/optical amount through
main, interior-light and reflected queries. Show the same source persists when core feeding stops. If the chosen
admitted profile excludes a counterexample, make that exclusion explicit rather than asserting a general inverse.

### CAP-R3 — P2: precipitation/event seams lack discriminating nonempty fixtures and seek semantics

**Location:** CLOUDS.md 401–409, 441–449 and CLOUD-CAPABILITIES.md CL29.

Precipitation flux is optional “if authored/supplied”; a typed stub records the cause. Lightning must prove
replay/seek/skip delivery without duplicates, but its interval-query/cursor semantics are unspecified. As written,
a storm containing no supplied flux or no lightning events can exercise the type while failing to show a usable
weather-consumer handoff. “No duplicates” also has different meanings after a backwards diagnostic seek, a new
replay epoch, a skipped presentation frame and authoritative forward progression. Existing F1/C11 event policy
must govern, not a local seen-ID set invented by the inspector.

**Concrete correction:** require at least one authored nonzero rain footprint and one nonempty lightning-event
fixture, plus absent/zero controls. Define the sampling interval/cursor, ordering, event identity lifetime,
forward-skip enumeration and backwards-seek/reset behaviour, distinguishing source inspection from gameplay/audio
side effects. This does not require rain, flash or thunder rendering.

**Closure witness:** the stub observes the same moving nonzero flux and complete ordered event set with the cloud
draw omitted, with two/skipped views, at normal/accelerated playback and after the explicitly defined seek/replay.
Flux absence remains valid for genuinely unsupported providers, but cannot alone close the required usable seam.

### CAP-R4 — P2: the completion audit conflates cloud-proof obligations with unbuilt engine rollout

**Location:** CLOUD-CAPABILITIES.md CL11, CL20/23/24/27 and 105–109; CLOUDS.md 315–318 and proof queue 558–565.

The design correctly says the commission builds standalone cloud proofs while actual engine interfaces follow
I1/I2/I3a/I3b. The inventory then says the full goal requires every shared CL obligation, several of which currently
mean actual engine provider/device/save/network bindings. CL11's status also groups full-sky/probe/rough-water
reflection as unproven, although CLOUDS explicitly selects a matte/mirror proof fixture and says this does not
qualify rough ocean reflection. The current audit can therefore be read as requiring the cloud commission to finish
the engine or production water integration before any completed cloud-proof delivery is possible.

**Concrete correction:** split requirement semantics and achieved proof-host evidence from actual engine rollout/
platform/consumer qualification. Name precisely which shared obligations are gates of this commission and which
are designed contracts with explicitly unbuilt bindings. Mark rough-water reflection as an affected future consumer
qualification unless it is separately selected as a required witness; keep independent reflected-ray appearance
mandatory. Do not label a mirror as rough-water proof, and do not silently turn water into a new mandatory build.

**Closure witness:** the final audit can separately say “required standalone five-family/weather/full-sky proof
passed/failed/unproven”, “actual engine integration unbuilt” and “target performance unqualified”, without hiding
any of those facts or making an unrelated engine milestone an accidental cloud acceptance criterion.

## Coverage conclusions and limits

No missing required family or named weather transition was found in the revised inventory. The proposed proof
order now follows the commission and keeps performance failures explicit. The convective/low-layer/filament source
separation supplies meaningful proposed mechanisms instead of claiming noise advection as lifecycle. These are
reasons to retain the qualified model portions while resolving CAP-R1/R2, not reasons to mark their appearance
accepted. Shape grammar failure remains a real unrun family-specific risk that only native/motion inspection can close.

The large workload counts are proposals, not user-imposed capacity promises. Four morphology variants, specific
altitude bands, spherical SKY-G and Moon paths must remain declared profile/inspection choices rather than new
universal product requirements. Current text mostly does that correctly. Camera-inside-cloud unrestricted flight,
scientific microphysics, all WMO types and rain/lightning effect internals remain optional; none is requested here.

I did not inspect prior proof source/code or rerun its evidence, so partial-pass statements were reviewed for scope,
not independently recertified. This report does not require a particular source algorithm or a positive design verdict.
Resolve the concrete producer ambiguities, then compare their revised worked cases before D0 closure. All actual
family, sky, motion, consumer and budget gates remain unproven/partial/failing exactly as their real evidence warrants.
