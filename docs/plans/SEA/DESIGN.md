# Sea: the wind's waves, one field for every body of water

13 November 2026, judged again from first principles on 14 November, and narrowed the same day to the wave field
when water was designed as one system. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[brief](BRIEF.md) · [module list](../ARCHITECTURE.md) · [the water system, `WATERS`](../WATERS/DESIGN.md). C02, C04
and C06 in [CONTRACTS](../../design/CONTRACTS.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

## The request

The owner, 10 November 2026, from [the brief](BRIEF.md): "wants water and clouds finished and consolidated". On 13
November: "use the fable planner like normal but you can bring that across when it makes sense."

The owner, 14 November 2026, in order:

> You can fully use everything in broadsides. But we should make sure that it aligns with the approach we want. But
> yes, you can copy that code out and reuse as is if it's useful.

> We can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
> design.

> That's the approach we were going with in broadsides that might not be the right approach here. The agent should
> look at that work from first principles and figure out how to design the best possible atmospheric system for the
> thrum game engine. Maybe that's a flexible set of cloud capabilities built from what we did in broadsides, maybe it
> isn't, but it should start from first principles.

The last was said of clouds. The lead reads the last words as governing the sea too, which is
[an assumption in the plan](PLAN.md#open-for-the-human): the design of 13 November, [the brief](BRIEF.md) and
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) describe the Broadsides approach and are inputs to
judge, never constraints.

So this design starts from what Thrum needs of a sea, chooses each mechanism against that, and only then asks what
Broadsides has that serves it. [Judged from first principles](#judged-from-first-principles) gives the verdict on each choice about the wave field; the verdicts on the rest of the earlier design are in
[`WATERS`' design](../WATERS/DESIGN.md#judged-from-first-principles). [What Broadsides offers](#what-broadsides-offers) says what may be copied.

It is not a blind study. Read for it: the design of 13 November and its two reviews; the Broadsides water notes at
`ae8c1ebb7` (`THRUM-PORT`, `TIMINGS`, `SURFACE-DESIGN`, `SEA-PARKED`, `PROGRESS`, `DESIGN`), its sea, wake and
transform kernels at that commit, and the foam spike at `2e5f47184`; Unity's and Crest's published water systems;
and in this repository the designs of `EFFECT-GRAPH`, `ATMOSPHERE`, `CONTENT-LOOP`, `APPLE-PRESENTATION` and
`CREATOR`, and the CPU model `SEA` M1 built.

The owner, 14 November 2026, later the same day:

> Please make sure the sea work is actually thinking about water as a whole. So puddles, rivers, water falls,
> streams, lakes, and more. We want something that's built cohesively.

So water is now designed as one system, in [`WATERS`' design](../WATERS/DESIGN.md), and this module is the part of
it that was always the sea's own: the wind's waves. Everything else the earlier design held (the surface, bodies
and the query, the wake, foam, optics, controls and the close) moved to `WATERS`, generalised to every body of
water, and its milestones M3 to M10 moved with it. M1 and M2 are unchanged, word for word.

## Purpose and scope

The wind's waves are one field for the world: swell and wind sea from a glassy calm to a gale, as a function of
place and time. Nothing in the field depends on where water lies, so it is computed once, and every body of water
reads it: the open sea whole, a lake at the share its fetch allows, a beach as far as its depth allows
([gains](../WATERS/DESIGN.md#waves-on-a-body-gains)).

**Commissioned:** the CPU wave model and its f64 oracle (M1), and the same field on the GPU at every frame's
time (M2). Nothing is drawn here. Cue S1's waves and the oracle half of S2 are this module's; the surface that
shows them, and every other cue, is `WATERS`'.

**Not built:** anything that needs to know where water is. Waves do not shoal, turn or break here.

**What exists.** `thrum-sea-model` is built by M1. Broadsides has the same kernels; its frame is not used.

**After this module.** Once M2 is Done, `thrum-sea-model` and `thrum-sea` pass to `WATERS`, which reads the
spectrum's band energies for its gains, declares these passes inside its own contribution and keeps these tests
passing.

## Research and selection

The sources, and what each was read for, are in [`WATERS`' design](../WATERS/DESIGN.md#research-and-selection),
where the sea design's list now lives. The three that select this module's mechanisms:

- **Unity's HDRP water** sums three periodic FFT bands, and scripts read a second copy of the simulation on the
  CPU. An all-FFT sea under the portable 256-point transform spans at most about 1.3 km while it still resolves
  a 20 m wave, which repeats fifteen times across a 20 km vista.
- **Crest** draws wave components into camera-centred levels of detail that the surface samples, which is what
  the field cache is.
- **Broadsides**' accepted calm and trade-wind sea (W1) has this layering, and is the evidence that it reads as
  open water. Four cascades cost it about 0.5 ms an evaluation on this Mac.

### Judged from first principles

| Inherited choice | Verdict | Why |
|---|---|---|
| Analytic long waves and FFT cascades as separate layers | **Keep** | Forces and the vista come from long waves, which are few. They must be evaluated exactly at a point by a CPU, for the oracle and for an authority without a GPU, and must not repeat. Short waves are many and matter only as statistics, which is what a periodic FFT patch with moments gives. One method for both fails one of them |
| The field computed once a tick and shown by blending two tick copies | **Change** | Long waves and cascades are functions of time and hold no state. Evaluating them at the display's time is exact, needs one copy and no lifecycle, and halves the reads of the most repeated work in the frame, the water pixel |
| A field cache for the long waves | **Keep**, with no state | Vertices, pixels and foam births all want the long waves as a field, and one evaluation serves the three. Evaluated whole each frame, it needs no wrapping history |
| Cascade phase from a time in seconds | **Change** | In f32 it is 0.26 rad wrong for a 7 cm wave after 24 hours (part F). Each bin turns a whole number of times in a declared loop, and the phase is an integer product that wraps |
| A module that also draws, floats, wakes and foams the sea | **Change** | Those are water's, on any body, and are designed once in `WATERS` |

### What Broadsides offers

Broadsides code may be copied where it implements a mechanism of this design as stated here; paths are in the
Broadsides repository at `ae8c1ebb7`, and nothing there is changed. [`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff)
says how a copy is recorded; this module's copies are listed in `docs/evidence/SEA/PROVENANCE.md`.

| Mechanism here | Where Broadsides has it | Expected fit |
|---|---|---|
| Spectrum, long-wave components, the f64 evaluator | `crates/sim/bw_water/src/ocean/` | M1 wrote these here from published sources. Nothing more is needed; its golden tests may be read for cases |
| Cascade evolution to the frame's time with whole-cycle loop phases; the 256-point transform inside 16 KiB of workgroup memory with twiddles built in f64; packing; mips | `crates/render/bw_render_water/shaders/ocean_evolve.wesl`, `ocean_fft.wesl`, `ocean_assemble.wesl`, `ocean_mips.wesl` | Close. These already do what [the ambient sea](#the-ambient-sea) asks |

## Mechanisms

### One field, two layers, each wave carried once

| Layer | Carries | Representation | Periodic |
|---|---|---|---|
| Long waves | Swell and the wind sea at wavelengths of 20 m and more | At most 192 analytic components, evaluated where needed | No |
| Cascades | The wind sea below 20 m | FFT patches, one band of the same spectrum each | Yes, per patch |

One directional spectrum (a fetch-limited wind sea with frequency-dependent spreading, plus narrow swell trains)
is split at 20 m. Above the split it is sampled into components with wave vectors chosen so that no two are
commensurate. Below it, it fills the cascades. Each wave vector belongs to exactly one layer. The engine takes
parameters: wind speed and direction, fetch, and a list of swells. Named presets are the game's. The third thing
a surface sums, the waves that bodies raise, is [`WATERS`'](../WATERS/DESIGN.md#interaction-waves) and owns
no wave vector of this spectrum.

A wind or swell change moves each component's and each cascade bin's **power** toward the new spectrum over a
declared response time and keeps its phase, so the sea changes with no pop and no restart.

### Time: the field is evaluated at the display's time

**Functions of time: the long waves, the cascades and the field cache.** Given the spectrum's powers and a time,
each is fully determined. They are evaluated **every presented frame, at the display's time** `t = C − 1 + α` in
ticks, where `C` is the frame's committed tick and `α` its interpolation alpha (C02); the CPU forms `t` in f64.
They hold nothing a frame could not recompute from the model and `t`. Their passes are every-frame passes and
not state producers in the seam's sense, so they run on a capture frame too, and no rule of commitment applies
to them:

| Frame | What the ambient layers show |
|---|---|
| Any presented frame, on a new tick or the same one | The model's sea at that frame's `t` |
| A pause | The same `t`, so the same sea |
| A hitch of any length, a backward tick, a new timeline | The model's sea at the new `t`. A step of more than one tick marks the sea's history invalid, because motion is not linear across it |
| Discarded | Nothing is kept, so nothing is left behind |
| Capture | The model's sea at the capture's own `t` |
| A replaced device | The next frame, as any other |

This is exact where a blend of two tick copies was not, and it is cheaper where cost is highest. A sea pixel is
shaded about 440 million times a second at 2560×1440 and 120 Hz and reads every ambient layer; with two tick
copies it reads each twice. The price is evaluating the cascades and the field cache on the frames between ticks
as well, which is tick-rate work done twice as often on the Mac and no more often in Chrome
([cost account](#cost-account)).

### The ambient sea

Each of the three is evaluated at the frame's `t`, and each also writes its **rate of change** at `t`: the motion
vectors, the query's carry and a lowered cadence all use it.

**Long waves.** Wave phase is computed in f64 on the CPU for the frame's `t` and reduced to one period per
component before upload, relative to a snapped anchor near the evaluation region. The GPU adds `k · (x − anchor)`
in f32. The CPU evaluator in `thrum-sea-model` uses the same components and is the oracle. The cascades, the wake
and the foam are addressed from the same anchor, reduced modulo each patch length in f64 on the CPU, so no layer
loses precision far from the origin.

**Cascades.** Three patches of 256² with lengths near 160 m, 37 m and 9 m, in ratios that are not rational, each
holding the band its texel size resolves. A frame evolves each spectrum to `t`, inverse-transforms displacement,
its derivatives and its rate of change, and writes them with mips. The mips hold slope moments (LEAN: the mean
slope and its covariance) and the Jacobian's mean and variance, so a footprint larger than a texel reads
variance, never aliased detail. Waves below the finest cascade add a constant slope variance from the spectrum's
tail. Patch count, sizes and transform length come from the profile and are checked against the capability
record; these values are the first profile and fit the portable limits.

**Cascade phase is a whole number of turns in a loop.** A phase formed from seconds in f32 is 0.26 rad wrong for
a 7 cm wave after 24 hours (part F). So the profile declares a loop, 1,024 s in the first profile, and each
cascade bin's frequency is the deep-water value rounded to a whole number of cycles in that loop. The CPU sends
the frame's time within the loop as a 32-bit fraction, and the GPU forms each bin's phase as an integer product
that wraps: the error is under 10⁻⁵ rad at any session length, and the rounding changes a bin's speed by at most
0.12% (part F). The cascades therefore repeat in time after one loop. The long waves never do. The model in
`thrum-sea-model` uses the same rounded frequencies, so the oracle and the GPU agree by construction.

**The spectrum's powers** belong to the CPU model, which moves them on a wind or swell change
([one field, two layers](#one-field-two-layers-each-wave-carried-once)). What the GPU holds of them is a copy
that the model can rebuild at any time.

**The field cache.** A camera-centred clipmap of the long waves, each level snapped to its own texel grid in the
world: displacement, slopes and rate of change, in levels whose texel doubles outward, each level holding only
the components it can resolve and passing the rest on as slope variance. It exists because three readers want the long waves as a field: the surface's
vertices, its pixels, and the foam's births, which need the summed displacement's Jacobian on half a million
texels. One evaluation of about 18 million component terms a frame serves all three. Evaluating in the vertex
shader alone would cost a like amount and leave the other two without. It is written whole each time it is
evaluated, so it needs no wrapping history. It is a display cache: nothing that feeds a force reads it.

### Limits, recipes and knobs

- **One physical profile on every recipe.** Component count, cascade layout and loop are the profile's, never a
  quality setting (C04).
- **Limits are inputs.** Transform length, workgroup memory, storage formats and counts are read from the
  capability record and refused at preparation when exceeded.
- **Recipes.** Both recipes run the portable kernels first. An Apple-tile variant is promoted only by a paired
  measured win with the oracle still inside tolerance, in `WATERS`' closing milestone.
- **Knobs** are declared in `WATERS`, with the rest of water's: field-cache reach, and the cadence of the field
  cache and of cascades below the profile's smallest force wavelength. A layer evaluated less often than every
  frame is carried between evaluations by its rate of change, with its slopes held.

### Check that it can work

- **S1.** No tile: the long waves are not periodic, and the cascades' joint period is far beyond the view. The
  accepted Broadsides W1 sea, which has this layering, is the evidence that it reads as open water. Vista: at 5 km
  a 150 m swell is under a pixel, so the vista is slope statistics, which the moments carry.
- **S2 and the oracle.** `long_wave_precision.py`: 192 components, 6 m significant height, points to 4 km from
  an anchor 50 km from the origin after 24 hours. The wrapped f32 evaluation differs from f64 by 0.05 mm, and by
  0.5 mm with every cosine off by 10⁻⁴; an unwrapped f32 evaluation is off by 18 mm. **The declared tolerance is
  2 mm** in height for the long waves at any query point.
- **Time.** `first_principles_proof.py`. Part F: after 24 hours a phase formed from f32 seconds is up to 0.26 rad
  wrong, and the loop's integer phase 7 × 10⁻⁶ rad; rounding to whole cycles in 1,024 s changes a bin's speed by
  at most 0.12%. Part G: a blend of endpoints one tick apart misstates a 0.4 m wave by 0.53% of its amplitude and
  evaluation at the display's time by nothing.

The scripts and their output are in [the evidence folder](../../evidence/SEA/design-proof/).

## The system as it runs

| State | Owner |
|---|---|
| Spectrum, components, the f64 evaluators, the conformance harness | `thrum-sea-model` (this module), no GPU dependency |
| The field's GPU passes and resources: long-wave phases, cascades and their mips, the field cache | `thrum-sea` (this module), `simulate`-stage passes on `EFFECT-GRAPH`'s seam that keep nothing between frames |
| Where water lies, the surface, and every reader of the field | `WATERS` |
| Presets | The game |

A frame: `upload` takes the wind and swell parameters and the long-wave phases and loop time for the frame's `t`;
`simulate`, every frame, evaluates the cascades and the field cache at `t`.

## Cost account

Estimates, unmeasured. M2 records them. The surface that reads the field is `WATERS`'.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Long-wave phases | Frame, CPU | 192 components | Microseconds | |
| Cascades | Frame | 3 × 256², packed transforms for displacement, derivatives and rate of change, two dispatches each way | About 0.4 ms | About 0.5 ms for four, as a whole-frame difference |
| Field cache | Frame | 8 levels × 128², about 18 M evaluations | 0.2–0.3 ms | |
| Memory | | Cascades, working sets, cache | About 15 MiB | |

**Why the field runs every frame.** Evaluating it on the tick and blending two copies would save about 0.7 ms on
every second frame at 120 Hz and nothing in Chrome, and would add a second read of every layer to every water
pixel of every frame. Both figures are estimates; `WATERS`' first cost record has the surface's real cost, and
the cadence knobs lower the field's rate if a record favours that. No timing figure decides an item
([brief](../BRIEF.md#goal-and-exit-rule), decision 3).

## Risks

1. **A 160 m repeat shows from 40 m up.** Early sign: `WATERS`' first vista capture. Fallback: a larger first
   patch under a profile the portable limits still admit, designed here first.
2. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M2 runs under WebGPU's default
   limits on the Mac; `WATERS` M3 is the first real Chrome run.
3. **The field costs more every frame than the reads it saves.** Early sign: `WATERS`' first record on repeat
   frames. The designed remedy is the cadence knobs. The wrong fix is a second copy blended in the surface pass.

## Proof and builder handoff

- **Target.** `mac-metal`. Every GPU check holds the `gpu` lock. Scratch goes in `tmp/sea-<milestone>/`, selected
  evidence in `docs/evidence/SEA/`.
- **Exact baselines.** The sea adds crates beside the existing scenes and moves no retained image. Every
  milestone carries the verifier as its claim-time check ([rules](../ARCHITECTURE.md#exact-baselines)).
- **Broadsides, dependencies and provenance** are as [`WATERS`' handoff](../WATERS/DESIGN.md#proof-and-builder-handoff)
  says, with this module's provenance file.
- **For `WATERS`.** Keep the passes declarable from another crate's contribution, and the spectrum's band
  energies readable for a fetch, since `WATERS` builds on both.

## Open questions

| Question | Settled by |
|---|---|
| What tolerance do the cascades meet against their f64 reference on Metal? | M2 measures first, then asserts the recorded bound |
| Is evaluating the field every frame cheaper than a second read of it in every water pixel? | `WATERS` M2's record, on tick frames and repeat frames |
| Is an Apple-tile kernel variant a measured win? | `WATERS` M11 |
