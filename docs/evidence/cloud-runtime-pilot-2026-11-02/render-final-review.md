# Render checkpoint delta review

2 November 2026. Read-only review of proof build `0ab4062170f6d491`. The current `view.wgsl`, `optics.wgsl`, `scene.js`, and `gpu-reference.js` SHA-256 hashes match `captures/gpu-reference-checks.json`. I inspected the source and pinned `cloud-0-2063-e1` image/receipt; I did not run a new GPU session.

## Corrected path

- The primary ray now computes `near = integrate(camera, direction, terminal, ...)` once. For a ground hit, the terminal is a ground surface response or `mix(ground, skyRay(reflectionOrigin, reflectedDirection), Fresnel)`. Final color is `near.s + near.t * terminal`. The reflected `skyRay` starts just above the ground and integrates its own upward path. This avoids the former duplicate camera-to-ground air prefix and keeps the reflected cloud/air ray independent of the main view. The camera is clamped above y=0 in `Scene.cameraPosition`; the allowed reflected direction is upward at a ground hit.
- Ground direct Sun uses `solar * airSun * receiverT * fogSun * max(sun.y,0) / PI` against the Lambert base. This fixes the missing Lambert `1/PI` factor in the earlier pilot. The ambient ground term remains an authored approximation.
- `coefficients(...,1)` is now used by the view, local Sun prefix, direct full-box ray, point light cache, ambient rays, and receiver map. The current pilot look is carried by one uniform tuple: second-octave amplitude `p.sun.w=1`, optical-depth factor `p.domain.y=.25`, phase eccentricity factor `p.domain.z=0`. Cache T1, direct T1, and local-prefix T1 use the same `.25` factor; the view uses the same amplitude/phase setting. This is internally coherent. It differs from the selected baseline default `(.5,.5,.5)` and should be labelled as a pilot look setting in proof records, not silently presented as that default.
- The current graph orders source update, receiver Beer field, point light cache, joint main/reflected view, and display. The image receipt names the same submitted frame/source epoch and shows an optical time within one source step of world time. No stale or out-of-order optical read is apparent from that graph and receipt.

## Pinned numerical evidence and exact limit

`gpu-reference-checks.json` records pass and no GPU errors on an Apple Metal 3 adapter. The checks actually exercise interior reaction, an ideal linear advection case, no-birth/birth controls, constant-coefficient optical depth, clear-air transmittance against a JS reference, and primary phase normalization. Their reported errors fit the stated tolerances. This is a useful finite *interior* GPU oracle, not proof of full source morphology, all-voxel finite state, local-plus-cache Sun accuracy, reflection equivalence, family art, or frame cost.

**Proof-label correction:** the check named `analyticBeer` only compares `tauRay` optical depth (`0.0090000024` versus `0.009`). Although its shader computes `exp(-tau)` into the result buffer, the receipt does not compare that channel, and it never invokes the view's `beer` source integral. Rename the check to `constantOpticalDepth` or add assertions for transmittance and a constant-source Beer integral before citing it as a Beer transport test.

## Remaining render qualification

- The dense single-box point cache and receiver field use fixed uniform 32/64-step full-box rays. No occupancy interval traversal, thin-support miss test, cache-versus-direct same-point error or offscreen multi-source completeness is shown. The point-cache/local-prefix composition is structurally disjoint, but numerical error remains unqualified. Current visuals do not pass morphology acceptance.
- `rayAir` now includes fog extinction/source on pre-box, post-box and empty-box paths, but its clear-air solar source outside the cloud box is not attenuated by cloud/geometry shadows. Inside-box air uses the point cache. A moving shadow through clear air and complete coupled fog remain P4 checks.
- Reflected sky uses its own current ray, but no matched mirror-on/off, main-hidden, same-point source/light or reflection-error capture has been pinned. The image demonstrates the path visually, not quantitative reflection acceptance.
- The fixed sky mean, direct single-scatter air plus transmittance LUT, dense source, no SH9/current environment capture, no TAA, no sparse occupancy, and unqualified timestamp totals remain declared pilot limits. The build must not be reported as meeting the 5 ms cloud, 16 ms frame, memory, or final family/atmosphere targets.

No canonical or proof source files were changed in this review.
