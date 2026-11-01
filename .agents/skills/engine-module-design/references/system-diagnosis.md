# Diagnose a fault at its system boundary

Use when an artifact, mismatch or unstable interaction might come from inconsistent representations, ownership or
integration. Seek the smallest sound correction to the general system; a local bug can still have a local fix.

## Isolate cause without confusing isolation with repair

Reproduce with known inputs and build identity. Hold state, camera, lighting or presentation constant as useful;
separate a local appearance jump from an application-wide hitch. Compare intermediate state and output, not only the
final image. Controlled ablations can isolate a contribution without proving that deleting it satisfies the design.

Trace the suspect path from source through state, transforms, reconstruction and final composition to all important
consumers. Ask who owns the quantity, which assumptions it needs, and whether another path derives a conflicting
version. A visible shader fault can originate in upstream support, timing, encoding, sampling or resource lifecycle.

Classify the failure: implementation mistake, unsupported model/combination, missing input/content, calibration,
representation error or presentation fault. Choose a correction at that responsibility. Turning a term off, changing
axes, adding randomness or damping a response may isolate or conceal a defect; retain such a change only when it
restores the intended contract or legitimately removes an unneeded contribution.

Ask whether the present model can produce the missing behaviour at all before tuning its parameters. Separate a
missing mechanism from an unresolved scale, inadequate driving content and an optical/presentation mismatch. Compare
them under held conditions; increasing spatial detail cannot supply absent time evolution or correct a different
quantity's calibration. After a negative result, choose a bounded experiment that can change the responsible design
decision rather than repeating a more expensive version of the same test.

## Make the representation contract explicit

For shared quantities, identify source/band identity, units and coordinate frame, spatial/temporal support, physical
time, transfer encoding, composition rule and consumer fidelity. Use one authority where these quantities must agree;
different consumers may approximate it with a stated bound. Do not force all appearance detail into simulation.

Derived quantities must follow the represented object and its transformations. For example, a geometric displacement
and its shading differential need compatible derivatives; localization of a field needs product-rule terms, and
changing frequency needs an integrated phase rather than applying the new frequency to all elapsed time. Specify
Jacobian/normal transformations for vector charts instead of assuming every surface is a heightfield. These are
examples of consistency requirements, not a prescribed representation for every module.

Check retained and unresolved detail across pixel, mesh, solver and query scales. Filtering one representation can
require compensating statistical response in another; do not count the same contribution twice or amplify unresolved
detail until it resembles the target. Legitimate independent authored/statistical detail should declare its role
rather than masquerade as a physically simulated quantity. Shared resources belong at the scope of reuse—such as a
view/frame service—rather than being allocated independently by every feature instance.

## Verify the correction across consumers

Select tests that expose the broken relationship, with an analytic or independent reference when meaningful. Exercise
relevant rotations/transforms, physical time, source tails, mixed contributions, resolution changes and fallback paths.
Compare ordinary output and motion as well as differential/transfer correctness. Distinguish bounded sampling error
from exact evaluator agreement; two agreeing evaluators can share the same mistake.

Reinspect adjacent consumers and measure resulting cost. Record what the ablation established, why the final repair
preserves the design, and which broader integration remains unproved. A generalized contract does not itself deliver
every regime or justify an unnecessary rewrite.
