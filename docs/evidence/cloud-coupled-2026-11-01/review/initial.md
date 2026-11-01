# Independent Q2/Q3 coupled-proof challenge — initial

2026-11-01. Reviewer is a separately commissioned agent. Read Thrum AGENTS, module-design implementation/review/performance, CLOUDS lighting/transport/reflection/profile/proof sections, capability rows CL07–12/14/15/21/22/26, C07/C08/C12, and Q1 selected-source summary. No implementation, image, moving output, readback or performance result has yet been reviewed. This is a model/fixture challenge, not acceptance or engine qualification. The source is the actual prepared Q1 scalar density, not invented procedural substitute content. Exposure to the saved design precedes this report; this is an independent derived challenge, not a blind blue-sky selection.

## Required observable coupling

1. One selected source ID, revision, age, transform and density field serve both camera views, direct lighting, the ground, matte sphere, mirror rays and optional environment rays. Looking away must not remove a source needed by another consumer. Neither a view nor a mirror advances source age.
2. A translated/developing density feature produces a geometrically corresponding transmitted-light feature on receivers, with displacement fixed by the declared Sun direction. Brightness need not match cloud alpha, and illumination must not use a camera projection as its geometry.
3. Every direct incident-light path contains the same geometry visibility and the same sum of air, both fog volumes and cloud extinction. Fog in front of the cloud in the Sun direction can darken the cloud even when disjoint on the camera ray; the matte sphere can block illumination of cloud/air/fog as well as the receiver. View visibility and light visibility are separate queries.
4. On an overlapping camera segment, extinction adds before exponentiation and each medium's source uses joint incident light. A cloud-only final overlay with separately shaded fog is not the stated closure.
5. A perfect planar mirror traces a second scene segment from the interface in the reflected direction. Foreground camera-to-mirror medium is applied once after reflection. Mirror rays can see sources/occluders outside the main frustum, and must terminate at reflected opaque geometry and ground where applicable.
6. Opaque geometry clips the actual ray, including cloud entry, inside support, exit and horizon cases. No sky/ground classification can discard foreground medium or leak a background cloud through the sphere.
7. Direction/intensity/source/sky/fog/geometry changes affect every relevant consumer at the same declared age. All direct, ambient and emissive inputs zero give zero radiance while view extinction remains. Sun-off alone is not darkness if sky ambient or Moon remain enabled.
8. Ordinary camera/source/object motion must work with spatial-only integration. No history is a valid baseline; it cannot claim temporal reuse or combined-layer motion qualification.

## Plausible-wrong counterexample

A convincing cloud renderer, a cloud-shadow ground texture, a screen-reflected cloud image and separately composited fog can all animate from the same playbar and pass a pretty still. It still fails this commission: the screen image loses an off-camera cloud; fog between Sun and cloud does not affect the cloud source; the sphere only casts ground shadows; two overlapping fog alpha layers depend on arbitrary order; camera-to-mirror haze may appear twice; cloud light may ignore ground below the horizon. The proof must distinguish this assembly from actual shared optical transport, not merely show correlated motion.

## Minimal discriminating model

Use one analytical scene query for opaque ground/sphere/mirror intersections and one coefficient query `coeff(x, age)` which returns air + fog A + fog B + actual cloud, each added once. Geometry and support are in metres, with an explicit external boundary and declared below-horizon source visibility. A light query at any volume or surface point uses the same geometry and finite supported-medium path, with its source raw intensity and normalized phase convention. If `toLight` is sample-to-light and `ray` is camera-to-sample, phase cosine is `dot(toLight, ray)`; describe the convention so a reversed HG lobe can be rejected.

For direct single scattering, integrate `sigma_t=sum(sigma_t_i)` and `j=sum(sigma_s_i*p_i*E_raw*V_geom*T_light)` jointly, then the exact constant-step `(T,S)` update. Retain declared cloud-only higher-order/ambient approximations if implementing the saved CLOUDS recipe; state when a direct-only diagnostic instead exercises only C08 algebra. A cloud-only cached tau is legitimate only as one term of the combined light path, never the complete visibility result. A cumulative cache needs inside-support endpoints and finite differences, not the ground's total column value.

Use a common bounded `traceSegment(origin, direction, terminal)` for main and reflected transport. At the mirror terminal, obtain `L_reflected = traceSegment(mirror + epsilon*r, r, reflected opaque/sky terminal)` and then compose main `(S_main,T_main)` as `S_main + T_main*L_reflected`. The reflected segment excludes the camera-to-interface segment. The epsilon must not skip a material-scale foreground interval or self-hit.

A standalone WebGPU analytic scene is appropriate because Thrum has no renderer. It establishes bounded source/transport/consumer semantics only. It must not be relabelled V1/V2/V3/V4 integration, public binding, full C07, complete Q3 or target admission. Analytic full-image tracing can be a correctness instrument, but a native full-resolution analytic renderer does not measure the proposed half-resolution service/upscaler cost.

## Small fixture and rejection observations

- Main camera sees a low cloud, ground and sphere; an independently positioned secondary camera/mirror sees a source feature outside the main frustum. Keep at least one mirror region with pure sky beyond and another with opaque reflected sphere/ground. Verify world-space matching with explicit ray probe coordinates, not visual resemblance alone.
- Fix density age and translate the cloud orthogonally to Sun, then fix transform and vary age. Capture corresponding main/secondary/mirror/receiver changes and record all source keys. This separates trajectory, source development and view authority.
- Move the sphere through a direct path to an in-cloud or fog sample. Compare raw-light, geometry visibility, air tau, each fog tau, cloud tau and final source term. Receiver-only sphere shadows fail this case.
- Place fog A before the cloud along a Sun path while it misses a chosen camera ray; cloud radiance must decrease. Place fog B crossing cloud in the camera ray and overlapping A elsewhere. Compare joint result to a fine-step independent reference and to a deliberately independent-overlay counterexample.
- Use homogeneous absorber-only slabs before, inside and behind a luminous scattering/cloud region. Check finite endpoints and disjoint composition analytically; reordering separated absorbers has predictable view/light effects, while splitting a homogeneous region into overlapping constituent coefficients must leave the joint result unchanged.
- Add absorbing fog only between camera and mirror. With reflected radiance held fixed, the measured result must follow `S_fg + T_fg*L_reflected` exactly once. Also place fog only on the reflected segment to distinguish two legs. Sphere occlusion on the reflected leg must work.
- Sweep opaque depth across before-entry, inside-support and beyond-exit at an ordinary horizon. Compare native ray tracing against reconstructed output with actual depth/interval acceptance. No rejected coarse taps may silently return unmodified background. A short native crop of the horizon and sphere edges is essential.
- Toggle each light independently and then all sources/ambient off. Move light direction at fixed age. Clear cloud support and restore the current age; stale directional/ambient data rejects reactivation. Optional Moon is either fully exercised/costed or explicitly unavailable.

## Readbacks and numerical evidence

Read back untonemapped scene-linear `(S,T)`, terminal radiance and diagnostic light terms for chosen direct/main/reflected/surface paths, with source age/transform and exact endpoints. Use a CPU/fine independent evaluator of the same actual source or analytic slab cases; rerunning the shader formula in another code wrapper alone is a weak oracle. Set tolerances before accepting results; distinguish source quantization, light-cache interpolation, primary quadrature and half-resolution reconstruction. Report errors near thin/fog/cloud/sphere boundaries, not only global means. Require finite nonnegative coefficients/T in [0,1], nonzero-to-zero source transitions, and max/percentile errors with known path identity.

## Cost and evidence boundaries

At 720p, half-linear main media has 230400 rays; a half-linear 640x360 reflection adds 57600. With one 128^3 field and a 256-step support cap, those imply up to 73.728 million primary cloud positions before skipping, plus joint air/fog and direct light queries. Per-sample direct shadow marching adds another ray-length factor, so fully nested analytic transport can readily dominate; do not infer viability from a fast source kernel. A full-resolution main doubles each dimension relative to half-linear and quadruples media ray count.

A budget-relevant implementation needs actual 1280x720 displayed output, half-resolution media, depth/interval-aware upscale or clearly attributed full-resolution fallback, reflected work and all source/light producers in one timed schedule. If the bounded proof keeps a brute-force reference path instead, report its integrated actual cost and failed/missing budget gate; do not treat its timing as the selected renderer's cost or quietly claim low-resolution upscale qualification.

Retain loaded JS/WGSL hashes, selected clip hash, actual page/runtime identity, viewport/DPR, device/backend, exact options, source-age progression, normal-speed and accelerated motion, and raw consumer readbacks. Separate GPU timestamps, CPU submission, completion wait and callback cadence. Readback/capture instrumentation should be excluded from the timed loop and its exclusion stated. No concurrent GPU benchmark from this reviewer. C12 target claims still require complete-frame sustained cadence/tails, real peak memory including browser overhead, startup and per-device qualification; standalone outputs do not establish those.

## Current disposition

No design-invalidating contradiction found in joint-coefficient/direct-path/reflected-segment model. Highest risks to reject early: ground-only visibility reused in cloud; independently lit overlapping media; mirror foreground applied twice; support/culling tied to main view; coarse reconstruction ignoring opaque/air/fog path lengths. Implementation/output/cost remain unreviewed. No code changes, benchmark, commit or acceptance performed by this reviewer.
