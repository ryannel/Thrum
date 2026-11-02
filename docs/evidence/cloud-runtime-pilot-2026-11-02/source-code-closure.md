# P0/P1a source code correction review

2 November 2026. Focused verification of the six findings in `source-code-review.md` against updated `src/weather.js`, `src/source.js`, `src/scene.js`, `src/bench.js`, `shaders/source.wgsl` and the added CPU/GPU reference checks. Scope remains the dense 96×80×80 source pilot; no sparse/full-family, native beauty or M2 claim. I made no implementation changes.

## Prior findings: implementation disposition

| Earlier finding | Current code finding | Evidence level |
| --- | --- | --- |
| CPU weather tied to optical render | `WorldClock.step` in `bench.js::frame` now advances `Weather` before the scene/GPU demand branch. Optical-off, provider switch and GPU backlog do not themselves stop CPU weather. `Weather` has a bounded frame history and source samples named times. | **Code path corrected;** `node tests/weather.test.mjs` exercises a renderer-free 10 s clock. Actual UI/off/backlog receipt is still needed. |
| Dry front side reversed | `source.wgsl::localSource` now uses `smoothstep(frontX−400,frontX+400,x.x)`. With a front at +3800 moving left, x=0 starts dry=0 and points to the right dry first. No-front `frontX=1e6` is neutral over the pilot bounds. | **Code math corrected.** GPU front/reaction reference check is added but no result receipt inspected here. |
| Reset/capture false submission and epoch mismatch | `bench.js::reset` drains the GPU before replacing world/weather and creating a new optical epoch; `Scene.reset` throws unless drained. Capture drains, requires `render` success, clones `lastSubmission`, waits and names the image with the submitted time/epoch. Provider configuration retires the old scene before selecting the new one. | **Code path corrected.** Rapid UI/queue capture evidence remains unrun. |
| Source ignores lift / periodic fixed sites | `SourceDriver` uses bounded seeded finite pulses, expires records, continues serial births at hazard rate `0.15×moist×lift`, and throws on 32-pulse capacity. `source.wgsl` samples pulse storage and additionally gates S by lift; zero lift yields no new source. | **Producer implemented.** CPU test runs 1200 s and zero-lift case. Local mature renewal/irregular 3D appearance remains visually unqualified. |
| MacCormack invalid donor fallback | Stage 3 now tracks invalid donor corners and selects `fwd.xy` rather than corrected/clamped `adv` when any required donor is outside the dense domain. | **Code branch corrected.** The new GPU linear-translation check exercises an interior path, not a crossing/unknown-donor case; boundary fallback still needs an independent observed GPU case. |
| Weather command range/replay | `command` validates finite per-control ranges before enqueue; `sendFront` enqueues a front command, which is committed with tick/revision into the bounded command record. Capacity is explicit. | **Code path corrected.** CPU tests cover invalid values and front logging. A complete visual checkpoint/recovery proof remains outside P0/P1a. |

The new curl field and coarse/detail remap provide an actual runtime morphological input to source transport and the common coefficient query. They are **candidate visual corrections**, not accepted Cu/Cb/Sc/St/Ci quality. The shader still initializes Cb from a tall field, so a clear/low initial state is needed later to prove continuous Cu→Cb growth. The source/consumer optical GPU checks in `src/gpu-reference.js` are present but were not run or witnessed in this review; no paper implementation, CPU test or screenshot promotes them to a passed GPU receipt.

## Remaining consequential source-time defect: non-causal fractional weather sampling

`Weather.sample(t)` interpolates between history frames **only if the later committed frame is already present**. Before tick 1 commits, `sample(0.75)` returns tick-0 state; after tick 1 commits, the same request interpolates toward tick-1 state. I reproduced this with the current module:

```text
const w = new Weather(); w.command({wind:12});
early = w.sample(.75).wind;      // 3
w.advanceTo(1);
late = w.sample(.75).wind;       // 4.493094714268017
```

`Scene.render` samples weather at each source substep. Thus two runs with the same seed, committed commands and fixed source tick count can apply different wind/forcing depending on whether source work happened before tick 1 or caught up after it. A command scheduled for tick 1 also influences optical state at t<1 only in the catch-up run. This violates the selected deterministic source/replay contract and means GPU backlog can change cloud evolution without changing recorded WeatherFrames. **Fix the sampling law, not its tolerance:** use a causal fixed interval value for all t within a committed tick, or analytically evaluate a known target from the previous committed state without letting a future command leak backward. A regression test should compare `sample(.75)` before and after tick 1 commits, including a queued tick-1 command and front; then compare source stepping live versus delayed catch-up from the same committed sequence. The result must match at each named source tick.

## Verification and remaining limits

`node tests/weather.test.mjs` and `node tests/reference.test.mjs` both passed in this review. These are CPU checks. The added `runGpuReferenceChecks` code compares constant-rate reaction, front loss, interior linear advection, zero-lift/no-birth, birth from empty q, analytic Beer, air transmittance and phase normalization. Its **execution/result is uninspected** here; even a pass would not establish boundary fallback, 3D mature morphology, coupled moving shadows/reflections or M2 cost. Actual browser receipts and personally inspected normal/accelerated motion remain required. The dense pilot is allowed as a bounded first source; missing P3 sparse pages/halo/mips are not attributed as defects in this review.

**Verdict:** six earlier code findings are corrected at implementation/CPU-test level. Fractional weather sampling is a remaining deterministic-source defect to fix before claiming P1a replay/source-time correctness. Native cloud quality and GPU/device qualification remain open.
