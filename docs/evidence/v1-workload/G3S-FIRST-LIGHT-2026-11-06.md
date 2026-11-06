# G3s first light — open

6 November 2026, Apple M3 Max Metal. Exact game build after `04d0d7b`; `./dev check` passed. G3s is **not qualified**: the spectral oracle, an isolated sky/LUT/aerial GPU budget, and scene lighting at twilight remain open. The 120 Hz display path belongs to G3b.

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [Sun-facing 1080p still](g3s-first-light/sun-facing-1080p.png) · [96-sample solar sweep](g3s-first-light/solar-sweep.mp4)

The game selects Sun time. The engine's Hillaire-style LUT path lights the sky and surfaces; the tone pass shares the live view record. A bounded view exposure makes the 0.77 twilight sky pixel sum 100/765, versus 20/765 at fixed exposure. A fine 17-step Metal dawn/dusk test and independent review of the 96-frame sequence found no brightness reversal at six sky points. The reviewer found the scene itself still too dark at twilight and pre-existing grid moiré. Recommendation: keep this exposure as an incremental sky improvement, then improve physically sourced twilight ambient light and compare sky radiance with the spectral oracle before closing G3s.

| Row | Target | Current observation | Status |
| --- | ---: | --- | --- |
| Sky + LUT + aerial GPU, 1080p | p95 ≤0.3 ms | Separate complete row unavailable; atmosphere compute p50/p95 0.257/2.510 to 0.403/3.960 ms in two loaded runs | Open |
| Whole game render-thread CPU, 1080p | G3b p95 ≤0.3 ms | Candidate A/B/A p95 1.219/1.361/1.206 ms; fixed-exposure control is B | Open; offscreen, shared load |
| Twilight and sweep | visible, no banding/flicker | Visible sky gradient; no sampled flash; dark scene remains | Partial agent review |
| Spectral sky radiance | declared oracle tolerance | Oracle not yet built/qualified | Open |

Three GPU agents and Blender shared the Mac. Candidate/control CPU runs were interleaved; the control was a temporary fixed-exposure build with the same engine frame path. Their GPU pass sums varied by multiples, so no GPU performance claim follows. Raw local logs: `tmp/g3s-exposure-{a1,b1,a2}.log`, `tmp/g3s-exposure-dev-check.log`. Selected stills and clip above are retained; raw PPM sweeps are disposable.
