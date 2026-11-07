# G3s first light — open

Updated 7 November, M3 Max Metal / Chrome156. Partial function/visual checkpoints; no spectra or speed claim.

| Check | Result | Scope |
| --- | --- | --- |
| Storage/camera/SDR | Scaled LUTs, πL, meter/history, shoulder f64 within1byte | Pass |
| SH9 diffuse | Projection0.0645% M0; refinement≤0.0483% M0; approximation5.225% M0 and331% on faint directions | Accuracy open |
| GGX cube/DFG | Metal constants, seams, rotated Sun, independent f64 hemisphere integral;120 energy/hemisphere cases | Pass |
| Normal repair | GPU regression fails before/passes after; front raster corrected, real backfaces retained | Pass |
| Aerial storage | Actual producer vs fp32 storage, Sun+2° through−9°; real consumer S/T units | Pass, same-model oracle |
| Cache/lifecycle | Cold/warm cancellation, Sun/profile/altitude changes, amplitude/view reuse; browser replacement | Pass |
| Look | Held phases and sampled camera/Sun routes independently reviewed | Partial |

![Initial reflection checkpoint: six Chrome views](g3s-first-light/solar-contact.png) · [sampled clip](g3s-first-light/solar-sweep.mp4)

That retained digest is Wasm `5d5988bcbe8a2b0e5f2b886eee030c09982e57565c3feabfce19fff9acfa3940`,
Chrome156.0.8078.81/ANGLE Metal M3 Max,1252×704. Seven held phases and24 UI-driven samples, paused authority,
one healthy generation; clip5fps is **not live cadence**. Metal foreground receives the ONE disk-free Sky.
Independent reviews find coherent reflections/gradients but grid moiré, shadow stairs/stipples and pastel contrast.
Transport/spectral accuracy, physical display motion and tab/mobile lifecycle remain open.

7 November normal repair: full `./dev check` and all11 actual Chrome cases pass at Wasm
`11308dde593463a392cde4bddc86228fdd1ca3aef0a580cf88527320ee511d3a`. Private cooked-valley images lose their
black contour seams (noon near-black terrain pixels3958→0). Independent code/visual review retains the repair;
landscape bands, grounding/contact shadows, flat foreground and twilight contrast remain unqualified.

Aerial RGB now stores ×1024 and decodes before Sun; T/alpha and transport stay unchanged. At Sun−9°, raw half
loses35852/93184 sampled faint RGB values to zero; the cited2.78e−9 sample changes100%→1.45% relative storage
error. This is quantization, not a physical-model tolerance. Actual consumer checks preserve S·Sun+T·surface.
Five fixed valley views change by at most1SDR byte; **the visible terraces remain**. An entirely black night
has no before match. Independent review retains the scoped correction. Full check and actual Chrome sky route
pass at Wasm `b30ad323c506866ea91bbb58d6a1a592e572829cdad89039283fbb8ce7c62cd4` (seven phases+24 samples).

**Cost history; no measured speed win.** Each range is two runs,10k entities/1080p/threaded Metal/200callbacks:

| Slice and recipe | Callback incl. GPU poll, p95 before→after | Scene GPU before→after |
| --- | --- | --- |
| GGX reflections, diagnostic fixture, shared load12–14 |0.732–0.863→0.921–1.120ms | p95 1.763–1.914→3.278–3.310ms |
| Normal repair, private61312-triangle valley, shared load3–11 |0.741–0.772→0.734–0.738ms | p50 1.561–1.566→1.570–1.571ms |
| Aerial encoding, same private valley, shared Mac |0.851–0.931→0.793–0.880ms | p50 1.588–1.595→1.585–1.593ms |

ABBA order; no allocation or isolated-GPU claim. The normal block's first before may overlap owned capture startup;
aerial block has no owned GPU overlap. Other sessions/load vary. Reflections add~4.19M fetches/changed Sky and~0.30MiB
products; prepared896 GGX directions remove repeated trigonometry. Earlier reflection block with overlapping owned
Chrome has14–17ms GPU tails and is retained raw. No whole-frame attribution solely to prefiltering.

[Design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains): GGX split-sum,
64²×6/seven mips,65² endpoint DFG,128/1024 samples. Sun/aerial/pre-exposure apply once. Selected transport stays
`e061671`; phase seed rejected, scalar/moment prototypes mixed. **Open daily review:** twilight M interpolation,
near-ground Sun lookup, meter/SH disagreement and spectral error. No whole-sky accuracy pass or iPhone result.
Raw inputs, images, GPU oracles and timings: `tmp/g3s-environment-reflections/`, `tmp/g3s-showcase/`;
scientific rejected variants remain in `tmp/g3s-{source-quality,spatial-prototype,rayleigh-moment}/`. No GPL inputs.
