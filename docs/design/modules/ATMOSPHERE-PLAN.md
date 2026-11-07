# Atmosphere milestones

Design [ATMOSPHERE.md](ATMOSPHERE.md) · Updated 2026-11-07
Milestone: M1 moonless midnight
Now: the packed 24 KiB profile passes the scalar reference.
Next: bind it in the atmosphere graph and qualify storage and scene lighting on GPU.
Blocked: none. G3s stays open.

## M1 The valley remains legible at moonless midnight: in progress

Done when

- [ ] On Metal and Chrome, the actual cooked valley at 0 h has a dark nonzero sky and a legible terrain silhouette, with diffuse rock and metal both responding to the same weak environment. Judge fixed ground and upslope cameras at native scale.
- [ ] The raw sky zenith and horizontal integral are on the intended dark-site scale. The packed field matches the converged scalar reference within 5% for horizontal irradiance and 10% per sampled zenith, peak, and split-horizon ray at 0, 1.8 and 15 km. Interpolated night in-scatter at 1, 10 and 30 km agrees within 10%. The tolerances qualify the implementation of this optical approximation. Switching off nightglow affects sky, meter, diffuse and reflection together, and restoring it changes no direct sunlight.
- [ ] A continuous sunset-to-night sweep has no exposure freeze while light still falls, storage-scale step, or broken shadow transition. Alternating bright and dark submissions across captures, drops, slot reuse and device replacement preserve the correct scale and pixels. A 1% Lambert control survives both devices' real HDR attachments and display paths. Dawn, noon and sunset retain their checked images.
- [ ] Pair whole-frame and pass cost with the old run and investigate tripwires. Workspace check and real window and browser motion runs pass independent visual review. Evidence goes in the existing G3s page.

Work

- [x] Reproduced a completely black midnight frame on the actual cooked valley at a91cdd0e. The scalar range check confirms the fixed fp16 scale loses a 10% Lambert surface.
- [x] The independently stepped scalar direct-plus-scatter probe resolves the positive-elevation peak and both limits of the curved horizon at 0, 1.8 and 15 km. The denser quadrature changes the sampled radiance by at most 0.063%.
- [x] A second scalar reference includes the finite shell, both components, the engine phase function, ground irradiance and finite paths. Raw source and rows are in `tmp/g3s-showcase/night-design/`.
- [x] The scratch RG16Float profile matches the dense reference on 29 distinct sky/ground rays, three irradiance rows and three finite paths. Maximum errors are 0.213%, 0.028% and 0.060% respectively. These are packed CPU reads. The GPU sampling and scene checks remain open.
- [ ] Wire the night channel into sky, diffuse, reflection and finite aerial transport. Prove alpha decoding, including interpolated paths and cooked material blending, on Metal and Chrome. Verify the solar-only outputs are unaffected.
- [ ] Connect GPU exposure to all forward colour writers and reconcile the policy with the render spec. Prove the lower EV clamp is reachable, daylight behavior stays intact and a recycled slot uses its own scale.
- [ ] Capture and inspect actual valley sequences. Review the raw data and record results.

## M2 Stars emerge as the observer turns and time advances: pending

Done when

- [ ] From the same valley views, star appearance follows the dimming sky; star positions match the chosen observer rotation, stay stable under slow pans and are occluded by terrain. Hold bright day, late twilight and moonless night; watch motion on Mac and Chrome.
- [ ] Integrated test-star flux stays within 5% across subpixel position and 720p/1080p/2160p. At matched night exposure, magnitude 6 is dimmer than magnitude 3 by a visible margin in the resolved footprints. Verify the rotation with known directions, including the time-slider wrap. A star centred behind the skyline leaves no sprite leak. Validate at image centre and edge.
- [ ] The catalogue provenance is complete, paired runs record frame cost and attachment traffic, and the workspace and independent in-game review pass.

Work

- [ ] Verify data rights and derive the compact immutable star records.
- [ ] Add the background draw with conservative footprint and qualify extent, depth and exposure changes.
- [ ] Run the in-game digest and close with evidence. Lunar illumination and the broader G3s calibration and shadow checks remain separately tracked.

## Review

Independent optical and graph reviews of the draft over `a91cdd0e` found
the EV floor, fp16 precision, shared scale selection, graph timing order,
finite aerial in-scatter and star occlusion/flux gaps. The revised design
specifies those mechanisms and the checks that still need a game run. Both
reviewers rechecked the finite-shell design (reviewed design blob `f9d0c12a`,
plan `c3cc5748`) and found no implementation blocker.

## Log

- 2026-11-07 — started the moonless-night milestones after the cubic LUT probe produced no appreciable SDR improvement.
