# Independent fitted-light appearance and sampled lifecycle review

2026-11-02. Scope: actual native 1280x720 stills and sampled frames from the retained Q1 comparator, with JSON identity/scope inspection. No GPU execution, timing admission, code edits or new physical-certification requirement. This review accompanies `review-fit-implementation.md`; that report records the inspected implementation, closed staged-clock finding and remaining cache-approximation qualification.

## Evidence actually inspected

Direct `view_image` inspection, without compositing or enhancement:

- `docs/evidence/cloud-cu-correction-2026-11-02/images/{passive-front,primary-front,fit-front}.png` and corresponding JSON. These hold source time90, prepared source, camera, Sun30 and exposure while selecting the three light modes. Source bounds are `[0,900,0]` plus512m cube.
- The same directory's `{fit-coupled,fit-low-sun,main-hidden,all-off,all-off-extinction}.png` and JSON. These are the deliberately low-altitude local fixture, not SKY-G or the full engine atmosphere/environment graph.
- `tmp/cloud-sky-2026-11-01/captures/motion-1793579723098-growth-{0,4,8}.png`: normal-rate source ages40,49.994,59.999s.
- `motion-1793579800488-breakup-{0,4,8}.png`: normal-rate ages200,209.982,219.980s.
- `motion-1793579885974-full16-{0,1,3,4,5,6,7,8}.png`: ages0,29.891,89.842,119.842,149.965,179.962,209.688,239.946s.
- `motion-1793579938289-full64-{0,1,4,6,7,8}.png`: ages0,29.619,119.238,179.008,208.877,238.733s.

All four motion JSONs identify build `e0425a51c36f46855c9dedfd79465915805c4ad910c0e556679de4cb7c9db748`; they retain829/780/631/158 rendered sample records respectively and the pinned prepared-Q1/table identities. I did **not** play the WebM clips or inspect every rendered frame. Sample counts and sparse snapshots are not a smoothness/flicker certificate. The camera follows the translated cloud bounds; receiver/sphere relative movement is visible, but this route is not an independent fixed-camera wind/material-correspondence test.

## Held light correction: clear useful improvement

The fitted image gives this same detailed source a much more neutral grey-white body than either passive or exact-primary-only images. Both controls remain conspicuously dark blue-grey; primary alone does not solve the visual problem. The fit retains an identifiable darker right flank, shaded lower body and useful internal transitions. The source's irregular outline and medium relief remain legible. It has not become a uniform white cutout, and the strong improvement cannot reasonably be attributed to a geometry/camera change in this held comparison.

This is a positive native-art result for the selected liquid-Cu lighting correction on the retained Q1 shape. It does not validate arbitrary geometry, overcast, ice, extreme optical thickness, all solar directions or the off-grid/cached-profile error over their domains. The cook's reported0.020958 off-grid error and unproven uniform energy certificate remain empirical qualifiers, not reasons to discard this appearance comparison. The extra interpolation-of-A approximation noted in the implementation review is also not measured by these images.

## Ordinary shadow and mirror consumers: qualitative coherence

`fit-coupled` shows the lit body, an independently positioned ground shadow and an inverted reflected cloud with compatible form/shading. `main-hidden` removes the main-view cloud and reveals the orange receiver sphere while retaining the cloud's ground shadow and reflected body. This is a useful native witness that those two consumers do not disappear with the main-view visibility switch.

The Sun5 image is darker/greyer with a longer displaced shadow and a correspondingly altered reflection. This supports local response to low solar elevation; the simple fixture does not establish sunset colour, real air transport or the complete sky/ground feedback graph. `all-off` is visibly black, while `all-off-extinction` retains the white cloud silhouette. That distinguishes zero illumination from removal of the source/extinction.

I see no concrete missing-binding, reversed-mirror or main-hidden shadow-loss defect in these images. These are qualitative observations: image inspection does not establish exact receiver transmittance, reflection equivalence, shared-air once-only accounting or temporal cache coherence at every frame.

## Lifecycle observations and remaining Q1 limits

Normal-rate growth samples show an initially faint, partly prefigured cloud scaffold becoming a filled detailed body between40 and60s. The sphere becomes progressively occluded, and the cloud shadow/reflection become stronger with it. The early30s accelerated samples are especially faint, with thin blue-white patches over the sphere and little body mass. This is visible local formation, but the early impression is wisps gradually filling a predetermined formation rather than strong rising/rolling convective heads. That remains a content/producer limit of this retained master; the light correction does not repair it.

The90/120/150s samples retain substantially the same outline and inner arrangement. Translation relative to the sphere/ground is clear, but mature structural renewal/head replacement is not. This agrees with the source's explicit static-mature declaration and prevents a new CF01 lifecycle/mature-motion pass.

By180s the body has lost material and split into distinct lobes; by200/210s there are separated, thinner remnants; near220s it is almost absent. Shadow and reflection diminish and fragment alongside the main cloud. The terminal full-cycle samples are clear of visible cloud. Thus the sampled breakup is not merely an unchanged silhouette under one uniform final fade. Some intermediate late remnants are soft and round, and loss of fine/medium structure can read as rounded puffs before final wisps. These images do not justify a claim of accepted natural detrainment or a newly demonstrated evolving turbulence field.

At similar source ages the16x and64x snapshots show the same broad lifecycle states and compatible consumer states. I found no gross sampled stale shadow/reflection, unrelated shape jump or leftover terminal cloud. I cannot infer absence of popping, shimmer, delayed updates or history artifacts between the inspected snapshots. The native videos are retained for that stricter temporal viewing.

## Scoped disposition

Keep this fitted-light result as a successful held appearance improvement and local multi-consumer diagnostic on the pinned detailed master. No new concrete implementation defect is established by the inspected images. Do not transfer this result to whole-family/four-variant acceptance, mature renewal, the hierarchy attempt, other cloud families, weather transitions, full SKY-G, engine integration or device performance. The known faint birth/static mature/rounded late-content limits remain explicit, as required by `CLOUD-CU-SOURCE.md`; they are not hidden by the brighter body. Full playback and the missing content/operating-scope witnesses remain separate work, not a reason to restart the bounded rejected hierarchy tuning attempt.
