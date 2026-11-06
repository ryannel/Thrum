# G3s first light — open

6 November, M3 Max Metal / Chrome 156. Partial visual/function checkpoint; no imported spectra or speed claim.

| Check | Result | Scope |
| --- | --- | --- |
| Storage/camera/SDR | LUTs, πL, meter/history, shoulder f64 within 1 byte | Pass |
| SH9 diffuse | Projection 0.0645% M0; reference refinement ≤0.0483% M0 | Sampling pass |
| SH approximation | Direct error 5.225% M0; up to 331% on faint directions | Accuracy open |
| GGX cube/DFG | Actual Metal constants, axes/edges/corners, rotated Sun, independent f64 hemisphere integral | Pass |
| Material/energy | 120 GPU consumer cases: metal/roughness/view, solar 0/1/2, furnace and back hemisphere | Pass |
| Cache/lifecycle | Cold/warm cancellation, Sun/profile/altitude changes, amplitude/view reuse; browser replacement | Pass |
| Look | Noon/sunrise/twilight subjects; 24 camera/Sun samples independently reviewed | Partial |

![Six selected Chrome views](g3s-first-light/solar-contact.png) · [sampled camera/Sun clip](g3s-first-light/solar-sweep.mp4) · [earlier native Sun](g3s-first-light/sun-facing-1080p.png)

Served Wasm `5d5988bcbe8a2b0e5f2b886eee030c09982e57565c3feabfce19fff9acfa3940`, Chrome 156.0.8078.81,
ANGLE Metal M3 Max, 1252×704. `node scripts/check_v1_browser.mjs --case sky-reflections` captures seven held phases
and 24 UI-driven camera/Sun samples in one healthy generation; world paused (final run tick2). Contact selects six;
clip is 24 samples at 5 fps, **not live cadence**. Source PNGs and all decoded frames were inspected independently.
Metallic foreground uses the ONE disk-free Sky; nearby 18% rough neutral stays outside local-light spheres.
Sunrise coverage was added after review; first occluded fixture was rejected. Twilight colours read on both materials.
No broad clipping/banding, blackouts or cube seams observed; grid moiré, regular shadow stipples/stairs and pastel
contrast remain. Motion PNGs have zero black/clipped pixels; H.264 damages the dense grid, so source PNGs govern.
Three sky-only 280-row comparisons against prior SH9 captures peak at 1 SDR byte. Transport/spectral accuracy,
physical display motion and actual tab/mobile lifecycle remain open; the scoped reflection slice is retainable.

**Recorded cost, not a win:** 10k/1080p threaded Metal, 200 callbacks/run, same harness; baseline `614156e` and current
fixture. ABBA ran after owned Chrome exited, other sessions active (load ~12–14). Each range is two run p95s:

| Hot path | Before | With reflections |
| --- | ---: | ---: |
| Render callback incl. GPU poll | 0.732–0.863 ms | 0.921–1.120 ms |
| Scene GPU pass sum | 1.763–1.914 ms | 3.278–3.310 ms |
| Atmosphere compute incl. aerial | 0.343–0.344 ms | 0.958–0.978 ms |

New work increases cost: ~4.19M sky fetches/changed sky, ~0.30MiB products, device-prepared 896 GGX directions.
Forward and unchanged tone buckets also rise; fixture/load confound isolation. No whole-frame attribution solely to
prefiltering. Earlier overlapping-Chrome block had 14–17ms GPU tails and remains raw; no clean-device timing claim.
Retain this feature baseline for the later whole-frame profile; no numeric admission target or allocation claim.

[Design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains): GGX split-sum,
64²×6/seven mips, 65² endpoint DFG, 128/1024 samples, correlated Smith; Sun/aerial/pre-exposure apply once.
SH9 retains independent horizontal metering; dark-direction and transport errors are separate. Selected transport
stays `e061671`, gains65536/1024; phase seed rejected (Sun−3° median14.34→17.24%), scalar/moment prototypes mixed.
Continuous closure refinement passes four cells but exposes inherited vacuum self-feedback; no whole-sky pass.
Raw refs/prototypes remain `tmp/g3s-{source-quality,spatial-prototype,rayleigh-moment}/`; no GPL-derived inputs.
Final captures/oracles, both timing blocks and rejected fixture metadata: `tmp/g3s-environment-reflections/`.
`./dev check`, actual Metal referees and all11 Chrome cases pass; no iPhone result. C250 has no readback;
NeutralHighlights preserves calibration but is not standard PBR Neutral. G3s remains open.
