# Sea: an open sea that bodies float on, with wake and foam

13 November 2026, judged again from first principles on 14 November. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md). C02, C04, C06, C07, C08, C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[the water decision](../../design/decisions/E1-GPU-WATER.md), [the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md)
and [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

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
Broadsides has that serves it. [Judged from first principles](#judged-from-first-principles) gives the verdict on
every choice the earlier design inherited: two changed, the rest are kept for reasons that do not rest on Broadsides
having built them. [What Broadsides offers](#what-broadsides-offers) says what may be copied.

It is not a blind study. Read for it: the design of 13 November and its two reviews; the Broadsides water notes at
`ae8c1ebb7` (`THRUM-PORT`, `TIMINGS`, `SURFACE-DESIGN`, `SEA-PARKED`, `PROGRESS`, `DESIGN`), its sea, wake and
transform kernels at that commit, and the foam spike at `2e5f47184`; Unity's and Crest's published water systems;
and in this repository the designs of `EFFECT-GRAPH`, `ATMOSPHERE`, `CONTENT-LOOP`, `APPLE-PRESENTATION` and
`CREATOR`, and the CPU model `SEA` M1 built.

## Purpose and scope

A reusable engine sea. Thrum's first outlet is teaching demos on the owner's website, then games
([the brief](../BRIEF.md#purpose)), so the sea has three kinds of user, and each sets a requirement:

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a sunset over water, how waves disperse, why a wake is a wedge | A sea that is right as well as good-looking, at the display's refresh, with only the parts the demo shows | Mechanisms with an oracle; every part beyond the open sea optional, at no cost when left out |
| A game | Bodies that float, wakes they feel, the same forces whatever the camera or the device does | One physical profile on every recipe; forces from the sea's own state |
| `CLOUDS` and `ATMOSPHERE` | A surface lit by the one sky and shaded once | No private sky, exposure or reconstruction |

Consumers built now are the sea example scene and `CLOUDS`' sea milestone.

**Commissioned:** cues S1 to S7, on `mac-metal` and `chrome-webgpu`, lit only by `ATMOSPHERE`'s products.

**Not built**, from the brief: spray release at chase distance, the attached sheet at the hull and a visibly
rising gas cloud (parked spikes); inland water, coasts, shoaling, surf and tides; planing trim calibrated to
tow-tank data, fleets, ray-traced reflection; multiplayer water. Also not built: Kelvin wake packets for far
bodies, a second wake window set, saving the wake and foam fields, and a camera below the surface. The seams the
water decision asks for are kept: causes are plain records with stable IDs, forces never depend on the camera, and
no water state would cross a network.

**What exists.** `thrum-sea-model`, the CPU model, is built by M1. Thrum has no sea on the GPU. Broadsides has a
whole sea whose surface pass takes 21 ms of GPU time at 1440p ([brief](BRIEF.md#where-it-is-finished)): its
kernels are a resource, and the way its frame is assembled is not.

## Cues

The cues and ranges are the brief's. Judged again, each is something one of the users above needs, so none is
dropped: S1, S5, S6 and S7 are the demo's sea, S2 is the game's, and S3 and S4 are asked for by the owner's
[water decision](../../design/decisions/E1-GPU-WATER.md) ("Wakes are fully supported interactive water. Bodies feel
them, they carry foam, and they outlive their source") and are the two subjects a water demo most wants to show. The
bar for every inspected cue is the sea and wakes of Sea of Thieves, in words, at the display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **S1** Open sea from the waterline to the horizon: wind sea over swell, no repeating tile, crests that sharpen with wind | Glassy calm to gale presets; camera 1–40 m above the water and a 1–20 km vista; noon, low Sun and twilight | One tiled FFT patch. Rejected by a 40 m view of a kilometre of sea, and by the long waves matching the CPU model at points a patch length apart |
| **S2** Bodies float: a small float and one hull rise, pitch and roll with the waves they sit on | One craft and eight floats; rest to 15 m/s; the GPU field read through one batched asynchronous query, one to two ticks late, never synchronously | Scheduled bobbing, or a CPU copy of the waves. Rejected by a float riding a wake crest that only the GPU holds, and by a count of blocking reads that stays zero |
| **S3** A wake that is water: it spreads behind a moving hull, persists after the hull stops or turns, and lifts a float it crosses | Displacement speeds to planing; turn, coast and restart; wedge angle and crest travel compared with theory at two speeds | A wake decal, or a constant-speed wave equation. Rejected by the transverse wavelength at two speeds and by a crest that keeps travelling after the source is removed |
| **S4** Foam that is material: born where waves break and where the hull meets water, carried by the water, thinning and vanishing with age | No foam on a calm sea or at rest; whitecaps scaling from fresh breeze to gale; a trail that outlives its source; no stamped or tiled pattern | Foam drawn from the present wave shape. Rejected by a trail that stays where the water carried it after the hull has gone, and by two views a tile length apart |
| **S5** Sea optics: the sky and Sun reflect, colour deepens with depth, the Sun glitters, cloud shade dims it once | G3s sky from dawn to night; screen-space reflection with the environment product as fallback; `CLOUDS` shade when present | A private sky or exposure, or a frame that only looks right at one exposure. Rejected by a seam test against terrain-lit geometry at the waterline at four times of day, and by a body's reflection and submerged part held to the optics equation at two exposures |
| **S6** Stable in motion: no shimmer, ghosting or history smear under reconstruction; cuts and rebuilds reset cleanly | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract | Zero motion on a moving surface. Rejected by motion read from submitted frames against the surface's known displacement |
| **S7** Controls a visitor can use: wind, swell, time of day and a field overlay, on Mac and in Chrome | The typed controls of `CREATOR` | A control that restarts the sea. Rejected by a wind change in motion with no pop |

Finished also means, from the brief: the CPU f64 long-wave evaluator is the test oracle and the GPU matches it
within a declared tolerance; persistent state advances from tick-derived time; capacity overflow is a counted
refusal; every cadence is a governor knob with a declared history invalidation; and the cost account holds at full
refresh on this Mac at the showcase extent, with paired records and tripwires, not a budget gate.

## Research and selection

What each source was read for, and how far.

- **Sea of Thieves** ([SIGGRAPH 2018 talk abstract](https://history.siggraph.org/wp-content/uploads/2022/09/2018-Talks-Ang_The-Technical-Art-of-Sea-of-Thieves.pdf),
  both pages read). Its ocean is Tessendorf's FFT. Foam is born at wave peaks and around objects that cut the
  surface, found by depth comparison in a camera-centred window, and the foam buffer is blurred with feedback so
  it disperses. Colour blends deep and sub-surface colours by view, Sun and a wave-peak mask. It has no dispersive
  wake solver. So the bar's wake is a foam trail, and S3 asks for more than the bar; S4's hull foam and dispersal
  are the bar's own.
- **eWave** ([Tessendorf 2014](https://jtessen.people.clemson.edu/reports/papers_files/ewavealgorithm.pdf), pages
  1 to 6 read). Height `h` and surface potential `φ` advance per wavenumber by an exact rotation, equations 19 and
  20: `h' = cos(ωΔt) h + (k/ω) sin(ωΔt) φ`, `φ' = cos(ωΔt) φ − (g/ω) sin(ωΔt) h`, with `ω = √(gk)`. For free propagation
  it is exact for any `Δt` and unconditionally stable, and the FFT form needs a periodic domain. The paper's
  exactness does not cover what this design adds to it, a source and an absorbing border; [the wake](#the-wake)
  says how far those hold.
- **Whitecaps** ([Dupuy and Bruneton 2012](https://liris.cnrs.fr/Documents/Liris-5812.pdf), read whole). Breaking
  is where the Jacobian of the horizontal displacement falls below a threshold. Coverage inside a pixel is
  `½ + ½ erf((ε − μ)/(√2 σ))` from the Jacobian's filtered mean and variance, so distant whitecaps average
  correctly from mipmaps. The authors name its limit: no persistence after a wave has broken.
- **Ship wakes** (Rabaud and Moisy 2013, [abstract only](https://arxiv.org/abs/1304.2653)). The wake's bright ridge
  sits at Kelvin's 19.47° at low hull Froude number and narrows roughly as 1/Fr above about 0.5.
- **Unity's HDRP water system** ([simulation](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-water-system-simulation.html)
  and [quality](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-quality-and-performance-decisions.html)
  pages, as summarised by search; not read in full). An ocean is the sum of three periodic FFT bands, two of swell
  and one of ripples. Scripts that float things read a second copy of the simulation run on the CPU, at half or
  full resolution. It simulates the deformed plane only, with no wake solver.
- **Crest** ([ocean simulation page, 4.11](https://crest.readthedocs.io/en/4.11/user/ocean-simulation.html), read
  whole). Wave components are drawn into camera-centred levels of detail that the surface samples. Its dynamic
  waves run a separate simulation in each level, whose wave speed comes from a gravity multiplier and a Courant
  number. Foam is deposited where waves pinch and in shallows, and fades at a set rate.
- **Broadsides**, as what was tried. Its accepted calm and trade-wind sea (W1), its wake and query milestones M1
  and M2, and the foam spike's five changes with blind verdicts are evidence that mechanisms of this kind make
  their cues. Its timings are evidence of where cost went: a fused full-rate surface shader at about 2.35 ns a
  pixel, a whole-scene copy, a full-screen reflection trace of about 3 ms, and world work regrouped per draw. Four
  cascades cost it about 0.5 ms an evaluation on this Mac.
- **Not read for this design:** the Atlas GDC 2019 slides (over the fetch limit, twice), Crest's source, and
  Horvath 2015 on directional spectra. A search for ocean or wake work published in 2024 to 2026 found none that
  supersedes the sources above. Nothing below rests on what was not read.

Four reproductions were run for this design and are kept with their output in
[the evidence folder](../../evidence/SEA/design-proof/): `wake_proof.py`, `long_wave_precision.py`,
`revision_proof.py`, whose five parts A to E are cited below by letter, and `first_principles_proof.py`, parts F
and G. [Check that it can work](#check-that-it-can-work) reports them.

### Judged from first principles

Every choice the design of 13 November took from the brief, the port decision or the Broadsides notes, judged
against the users and cues above. "Keep" means it would be chosen again with Broadsides out of the room.

| Inherited choice | Verdict | Why |
|---|---|---|
| Three layers: analytic long waves, FFT cascades, a wake | **Keep** | Forces and the vista come from long waves, which are few. They must be evaluated exactly at a point by a CPU, for the oracle and for an authority without a GPU, and must not repeat. Short waves are many and matter only as statistics, which is what a periodic FFT patch with moments gives. One method for both fails one of them: an all-FFT sea (Unity's) under the portable 256-point transform spans at most about 1.3 km while it still resolves a 20 m wave, and that repeats fifteen times across a 20 km vista; an all-analytic sea cannot hold the short waves |
| The ambient sea computed once a tick and shown by blending two tick copies in the surface pass | **Change** | Long waves and cascades are functions of time and hold no state. Evaluating them at the display's time is exact, needs one copy and no lifecycle, and halves the ambient texture reads of the most repeated work in the frame, the sea pixel. A tick copy is the Broadsides frame, and the blend was a patch for showing it at 120 Hz. See [time](#time-functions-are-evaluated-at-the-displays-time-state-steps-on-the-tick) |
| A field cache for the long waves | **Keep**, with no state | Vertices, pixels and foam births all want the long waves as a field, and one evaluation serves the three. Drawing components into camera-centred levels is also what Crest does. Evaluated whole each frame, it needs no wrapping history |
| Cascade phase from a time in seconds | **Change** | In f32 it is 0.26 rad wrong for a 7 cm wave after 24 hours (part F), which a demo left open would show as stutter. Each bin turns a whole number of times in a declared loop, and the phase is an integer product that wraps |
| Bodies read the GPU through one batched query, one to two ticks late | **Keep** | The owner's water decision puts all water on the GPU, and the wake exists nowhere else. A CPU copy of the waves (Unity's) cannot feel a wake |
| The wake: eWave on two body-following windows with complementary bands | **Keep** | It was selected against five alternatives by reproduction, below, and nothing published since changes the table. Crest's per-level wave equation is the candidate that fails |
| A wake source that is the body's own reaction force | **Keep** | The amplitude is physical and has no gain to tune |
| Foam as four transported stocks | **Keep** | Each stock is one thing a viewer sees: white water, lace, pale water under it, and the smooth ribbon a hull leaves. All four live in one texture, so four cost what one does. The bar's foam, accumulated and blurred, has no ageing and no pale water |
| Reflection and refraction of registered bodies only, with no scene copy | **Keep** | An open sea is optically thick and empty. A whole-scene copy ends a render pass on a tile GPU for pixels that reflect only sky |
| Rigid bodies integrated by the game | **Keep** | Thrum has no physics owner, and the engine must hold no hull |
| One sea, all of it or none | **Change** | A sunset demo needs neither wake, foam nor query. Each is a [capability](#capabilities-recipes-limits-and-knobs) a scene leaves out at no cost |
| Nothing copied from Broadsides; assets made here | **Change** | The owner's words of 14 November. See below |

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each milestone judges fit file by file; this table says where to look. Paths are in
the Broadsides repository at `ae8c1ebb7` unless a commit is given, and nothing there is ever changed.

| Mechanism here | Where Broadsides has it | Expected fit |
|---|---|---|
| Spectrum, long-wave components, the f64 evaluator | `crates/sim/bw_water/src/ocean/` | M1 wrote these here from published sources. Nothing more is needed; its golden tests may be read for cases |
| Cascade evolution to the frame's time with whole-cycle loop phases; the 256-point transform inside 16 KiB of workgroup memory with twiddles built in f64; packing; mips | `crates/render/bw_render_water/shaders/ocean_evolve.wesl`, `ocean_fft.wesl`, `ocean_assemble.wesl`, `ocean_mips.wesl` | Close. These already do what [the ambient sea](#the-ambient-sea) asks |
| The eWave step: height and potential as one complex field, the exact turn, a source carried through the step, a damped border | `shaders/ocean_wake.wesl` and its CPU twin `crates/sim/bw_water/src/wake/grid.rs` | Close for one level. The two banded levels and the step of several ticks are this design's |
| The batched query and its ring | `shaders/body_query.wesl`, `body_sea_query.wesl`; `crates/sim/bw_water/src/ocean/query.rs` | The kernels may fit; the mailbox and packing are written for Thrum's tick page |
| Foam stocks, deposition, transport and the stochastic pattern sampler | At `2e5f47184`: `crates/gpu/bw_gpu/shaders/lib/sea_foam.wesl`, `foam_pattern.wesl`, `whitewater_medium.wesl`; `crates/render/bw_render_water/shaders/ocean_foam.wesl`, `ocean_deposition.wesl` | The two changes the spike passed are here. Check each rule against [whitewater](#whitewater) before copying |
| The ranked foam pattern | At `2e5f47184`: `assets/textures/water-foam/ranked.png` (Git LFS) and its recipe in `factory/materials/water-foam/` | Usable as the example's pattern. It derives from Crest's foam texture under the MIT licence, and that notice comes with it |
| Hydrostatics, hull stations, a hull | `crates/sim/bw_water/src/hull/`, `body.rs`; Broadsides' ship assets | Game-side code and content for `examples/sea` |
| Water optics: Fresnel, in-scatter, the microfacet Sun | Functions in `shaders/ocean_surface.wesl`, `ocean_volume.wesl` | Take functions. The pass they sit in is where the 21 ms went and is not copied |
| Sky, exposure, temporal AA, the Bevy host, `RenderModule`, `pass_list`, `TargetPool`, frame packets, lake and coast code | | Not used. Thrum has one owner for each |

Broadsides shaders are WESL and Thrum's are WGSL, so a copied shader has its imports resolved into Thrum's
modules; its algorithm is left as it is. [The handoff](#proof-and-builder-handoff) says how a copy is recorded.

### The wake operator, decided against the cues

| Candidate | Verdict |
|---|---|
| A wake decal or foam trail only (the bar's own) | Fails S3: nothing lifts a float |
| Constant-speed wave equation on a grid, as in Crest's dynamic waves | Fails S3. In the reproduction its ridge follows the Mach angle, 42° at 3 m/s and 17° at 6 m/s, and it has no transverse wave train |
| Steady Kelvin pattern evaluated per body | Fails S3: it has no memory, so it cannot persist through a stop or a turn |
| Wave packets or wavelets (Jeschke and Wojtan 2017, 2018) | Correct dispersion, but cost grows with packets and a rasterised field is still needed for the surface and the query. Not selected |
| Generalised shallow water with dispersive waves (Jeschke and Wojtan 2023) | The inland choice in [WATER](../../design/modules/WATER.md), for beds and shores. An open deep sea has neither, and pays its cost for nothing |
| **eWave on body-following periodic windows, two levels with complementary bands** | **Selected.** Exact deep-water dispersion at any speed, a few FFTs a tick, and stable at any step |

Two levels, because one 256² window cannot both resolve a 0.5 m ripple at the hull and hold a 25 m transverse
wave: a 64 m window of 0.25 m cells does the first and a 512 m window of 2 m cells the second. Two overlapping
periodic levels can only avoid carrying one wave twice if each owns a band of wavenumbers, and the reproduction
shows what happens otherwise: two full-band levels summed are wrong by 69% of the field's RMS height, against 2.2%
with the split.

**What would change these selections.** A sea that must meet a shore reopens the operator, under WATER. A
first-frame record that shows the surface's representation to be wrong, and not merely untuned, reopens it through
a proposal in the log ([cost account](#cost-account)). It does not stop the milestones after it.

## Mechanisms

### One sea, three layers, each wave carried once

The sea surface is the sum of three layers. Each wave vector belongs to exactly one.

| Layer | Carries | Representation | Periodic |
|---|---|---|---|
| Long waves | Swell and the wind sea at wavelengths of 20 m and more | At most 192 analytic components, evaluated where needed | No |
| Cascades | The wind sea below 20 m | FFT patches, one band of the same spectrum each | Yes, per patch |
| Wake | What bodies and impacts raise | Two eWave levels in windows that follow the craft | No |

One directional spectrum (a fetch-limited wind sea with frequency-dependent spreading, plus narrow swell trains)
is split at 20 m. Above the split it is sampled into components with wave vectors chosen so that no two are
commensurate. Below it, it fills the cascades. The engine takes parameters: wind speed and direction, fetch, and a
list of swells. Named presets are the game's.

A wind or swell change moves each component's and each cascade bin's **power** toward the new spectrum over a
declared response time and keeps its phase, so the sea changes with no pop and no restart.

### Time: functions are evaluated at the display's time, state steps on the tick

The sea holds two kinds of thing, and time reaches them differently.

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

**State: the wake and the foam.** Each is the integral of what bodies and breaking waves did, and advances on the
committed tick through `EFFECT-GRAPH`'s
[state table](../EFFECT-GRAPH/DESIGN.md#state-time-and-the-tentative-frame-m2). The seam runs a producer once in
a frame, so each operator takes the number of ticks to advance as a parameter:

| Layer | A frame *n* ticks ahead | A full frame: the first of an epoch, a backward tick, a new timeline, a replaced device |
|---|---|---|
| Wake | One step of `min(n, 8)` ticks ([the wake](#the-wake)) | Reset to calm, and the scene's status says so |
| Foam | One step of `min(n, 8)` ticks, each operator written for a step of any length ([whitewater](#whitewater)) | Reset to none |

**The catch-up bound is 8 ticks**, 133 ms at a 60 Hz tick. Ticks beyond it are counted as dropped and reported:
the wake's waves and the foam's ageing are then that many ticks late, and nothing else changes. The bound is
where the wake's one-step error is still small (part A: 0.16% of the field's RMS height after one 8-tick step,
1.4% after one of 16).

**Two endpoints, for state only.** State cannot be evaluated between ticks, and a wake that stepped at 60 Hz
under a hull drawn at 120 Hz would judder against it. So the wake and the foam are each kept in two banks, the
endpoints: the state at the committed tick `C`, and the state at the tick committed before it, `P`. Each bank
carries the tick it holds, and the ticks commit with the frame. The surface draws the blend of the two at weight
`w = clamp((t − P) / (C − P), 0, 1)`.

| Frame | The banks once it is submitted | What the surface draws of the wake and foam |
|---|---|---|
| The same tick: a repeat frame, a pause | Unchanged | The blend at `w` |
| *n* ticks ahead, *n* ≥ 1 | The bank that held `P` now holds the new tick: `(C, C + n)` | The blend at `w`. For *n* = 1 the endpoints are one tick apart and `w = α`. For *n* > 1 the older endpoint is *n* ticks old and `w` runs from `(n − 1)/n` to 1 |
| A full frame | Both hold the frame's tick: the wake calm, no foam | That state, unblended, until the next tick. History is marked invalid |
| Discarded | Unchanged: nothing commits, under the seam's tentative rule | Nothing |
| Capture | Unchanged: the seam runs no producer and keeps the state, whatever the capture's tick | The committed endpoints at the capture's own `w`, under the ambient sea at the capture's `t`. A capture at another tick shows the committed wake and foam, not that tick's |

A blend is right for these two. Under a moving hull the wake is forced, and a blend of two dents a few
centimetres apart is the dent at the hull's interpolated place to second order; the free waves it misstates are
small. For a wave of amplitude `A` and frequency `ω`, endpoints one tick apart misstate it by at most
`A(ωΔt)²/8`: 0.5% for a 0.4 m wave at a 60 Hz tick (part G). Endpoints `s` ticks apart, in the one tick that
follows a hitch, misstate it by at most `A(ωΔt)²(s − 1)/2`, and never by more than `A(ωΔt + 2/s)`: at `s = 8`,
0.3% for a 20 m wave, 3% for a 2 m wave and 15% for a 0.4 m ripple (part B). Both are declared approximations of
the wake alone, on frames whose history is already invalid.

A history pair's bindings do not offer the older side on a frame whose producer was skipped, so the endpoints are
two persistent banks, each written by its own skippable producer. On a frame that advances, the producer of the
bank that holds `P` runs; on a full frame both run. `EFFECT-GRAPH` needs no change for this.

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
([one sea, three layers](#one-sea-three-layers-each-wave-carried-once)). What the GPU holds of them is a copy
that the model can rebuild at any time.

**The field cache.** A camera-centred clipmap of the long waves, each level snapped to its own texel grid in the
world: displacement, slopes and rate of change, in levels whose texel doubles outward, each level holding only
the components it can resolve and passing the rest on as slope variance. It exists because three readers want the long waves as a field: the surface's
vertices, its pixels, and the foam's births, which need the summed displacement's Jacobian on half a million
texels. One evaluation of about 18 million component terms a frame serves all three. Evaluating in the vertex
shader alone would cost a like amount and leave the other two without. It is written whole each time it is
evaluated, so it needs no wrapping history. It is a display cache: nothing that feeds a force reads it.

### The surface and the frame contract

The surface is a camera-centred mesh whose spacing doubles outward, snapped to the world so vertices do not
swim, closed to the horizon by a far skirt. Earth's curvature is not modelled; the sea covers everything below
the level horizon. A vertex is displaced by the long waves (field cache), the cascades its spacing resolves, and
the wake. A pixel takes its normal from the same layers down to its footprint and its roughness from the slope
covariance of everything smaller (Bruneton's geometry-to-BRDF transition, through the LEAN moments).

The sea draws in `EFFECT-GRAPH`'s `after-opaque` stage, once, and writes the frame contract for its pixels:

- **Depth.** The sea writes scene depth. Reconstruction's depth test and any later medium, such as a cloud march
  that must stop at the water, then see the sea as the nearest surface. This needs
  [a ruling](#what-the-sea-asks-of-other-modules).
- **Motion.** From unjittered positions: where this surface point was at the previous presentation. For the
  ambient layers that is their rate of change at `t` times the interval since the previous presentation; for the
  wake it comes from its two endpoints.
- **Reactive.** Foam, glitter and the reflection of moving bodies write reactive coverage.
- **Reset.** A sea rebuild, a profile change, a full frame, an advance of more than one tick and a foam pattern
  change mark the sea's history invalid. A quality knob applies the invalidation it declares.

The camera stays above the local surface. Below it the sea still draws its surface and nothing more.

### Bodies and the query

**The query.** Once per tick the game posts sample points for every body: position, a footprint radius, the tick
the answer will be used at, and the body's ID and generation. One compute dispatch answers them all from the
sea's own state, never from the field cache or the camera:

- long waves, evaluated analytically at each point for the tick it will be used at;
- cascades, read from the frame's evaluation at the mip that matches the point's footprint, so a 6 m hull does
  not shake on ripples;
- the wake, read from its windows, with **the body's own static dent added back** (below).

Crests sharpen because displacement is horizontal as well as vertical, so the surface above a world point is
found by fixed-point iteration on the horizontal displacement; the oracle iterates to convergence.

Each answer is height, slope, the surface's velocity and the time derivative of height, with one support flag
per layer, and it is **valid at the use tick** its point asked for. The dispatch brings every layer to that tick
before it sums them: the long waves are evaluated there, the height of the cascades is carried there from the
frame's `t` by its rate of change, and the wake's from its state tick. The answer carries the use tick and the
times it was made from. A game that uses an answer at another tick moves the whole answer once, by its time
derivative from the use tick, so no layer is extrapolated twice. For a wave of frequency `ω` carried `Δt` the
remaining error is at most `½(ωΔt)²` of its amplitude, reported per profile. The cascades are carried at most
three ticks: 0.4% for a 20 m wave and 3.8% for a 2 m wave, whose height is centimetres (part G).

Results come back through a ring of mapped buffers one to two ticks later. A result older than the declared
maximum is unavailable, and the game falls back to the conformance model in `thrum-sea-model` (long waves only)
and counts it. A hitch ages answers the same way. No path waits on the GPU: a blocking map or poll is a defect,
and a counter proves there is none.

Inputs travel with the tick page: the game puts its body records and sea parameters in its page, and the scene
hands them to the sea in `upload`. Results return through a mailbox the game drains at the start of a tick.
Query capacity comes from the profile and the capability record. A request over capacity is refused whole, by
body in ID order, counted and reported.

**Where bodies are integrated.** In the game. The engine owns the query and the wake source; the example scene
owns hydrostatics, drag, thrust and the rigid-body step, as game code, until Thrum has a physics owner.

### The wake

**State.** Two levels of 256², with cells of 0.25 m (a 64 m window) and 2 m (512 m), each holding `h` and `φ`.
The far level owns wavelengths above about 12 m and the near level the rest, through complementary filters on
the source. A level never holds a wave from the other's band.

**A step of *n* ticks.** Transform, add half the step's impulse, apply equations 19 and 20 for `nΔt`, add the
other half, transform back, and damp a border one eighth of the window wide by its per-tick factor to the power
*n*. The border absorbs what reaches the edge, so nothing wraps. The impulse is the body's pressure summed along
its path over the step, placed in Fourier space at exact positions, so a wake neither stair-steps with the cells
nor bunches where a hitch ended.

Free propagation is exact for any step. The two additions are approximations that grow with it: the impulse split
around the rotation is second order in the step, and the border treats a wave that crosses it during the step as
if it had been there throughout. Part A measures them on a hull at 6 m/s against the same run stepped tick by
tick: one 8-tick step differs by 0.16% of the field's RMS height, a hitch of 8 ticks every second by 0.6%, and the
same with the pressure held at the body's last place by 3.4%, which is why the impulse follows the path. At 16
ticks the figures are 1.4% and 4.9%. Hence the catch-up bound of 8.

**Windows follow the craft, not the camera.** Each level is world-anchored and shifts by whole cells, with the
craft held ahead of centre so most of the window lies behind it. Two players looking in different directions
would get the same forces. The finished scope has one window set, around the craft the game names.

**The source is the body's own reaction.** Each tick a body record gives its pose, velocity, a small map of its
wetted footprint in hull coordinates and its bounding box (both registered once), and the vertical force the
water put on it in the game's last step. The engine spreads that force over the footprint as pressure. The
amplitude is therefore the body's weight at rest and its load in motion: a physical input, with no gain to tune,
the same at any grid size (C09's reaction, returned once). Body records over the profile's source capacity are
refused in ID order, counted and reported.

**The static dent.** A pressure patch at rest settles to a dent of depth `p/ρg` under the hull: the hull's own
displacement. A body that read that dent as the water level would sink into its own hole. So the query adds
`p/ρg` of the asking body's own patch back at its points. At rest the body then reads the undisturbed level, and
in motion it still feels its own bow and stern waves, and every other body's wake unchanged.

**What the reach means.** Waves shorter than about 12 m exist only in the near window, some 30 m behind the
craft; longer ones reach about 250 m. Beyond that they fade through the border. This is the water decision's
sizing, stated as a limit ([plan](PLAN.md#open-for-the-human)).

### Whitewater

Foam is transported material, never a function of the present wave shape. It is held as four stocks, because a
viewer sees four things: white water, the lace it ages into, pale water where air is still under the surface,
and the smooth ribbon a hull leaves. The four are channels of one texture, so they cost what one would. Broadsides
arrived at the same four, and its spike passed the two rules marked below.

**Stocks**, per square metre, on two camera-following wrapping levels (0.25 m and 2 m cells): submerged air,
fresh film, aged film, and slick. Fresh and aged film are shares of the surface, and their sum is the cover,
between 0 and 1. Slick damps the short-wave roughness under it.

**Transfers and sinks.** Material moves between stocks only along one chain, each link on its own lifetime:

| From | To | Rule |
|---|---|---|
| Submerged air | Fresh film | Air that reaches the surface whitens it once, at the water type's yield, bounded as a birth is. Air that surfaces under full cover is lost |
| Fresh film | Aged film | Rupture: everything fresh film loses, aged film gains |
| Aged film | Nothing | The chain's sink |
| Slick | Nothing | Slow decay. Slick is fed only by births |

So a recipient rises with no cause running: aged film grows out of fresh film, and fresh film out of submerged
air, before either falls (part D: from fresh film alone, with lifetimes of 6 s and 20 s, aged film reaches 0.53
after 6 s). What cannot rise without a birth is the inventory, fresh plus aged film plus the yield times the air.
With the source off every stock ends at nothing. The chain is linear between births, so a step of any number of
ticks has a closed form, and one long step equals many short ones to rounding.

**Births come only from booked causes**, as bounded rates `dc/dt = r(1 − c)` integrated exactly over the tick:

- *breaking waves*: where the Jacobian of the summed displacement falls below an onset. The onset is biased short
  of the Stokes limit (the spike found that the physical limit almost never fires on a height field), and the
  rate follows the swept-front rule, so one passage of a breaking crest leaves cover behind it;
- *wake breaking*: the same criterion on the wake's displacement;
- *hull contact*: along the footprint's edge, from the body's speed relative to the water;
- *the stern ribbon*: behind the footprint, from the power the body lost to the water, which the game's own drag
  supplies in the body record.

A cause that loses no power makes no foam. That is what keeps a calm sea and a hull at rest clean.

**Transport.** Stocks move with the surface drift, gather where it converges, and spread down their own gradient
with a diffusivity that dies with the turbulence that made them. This is the dispersal the bar gets from a
feedback blur, as a physical term. Drift is traced back along the flow and spreading is a kernel as wide as the
step makes it, so a step of up to the catch-up bound stays stable. Beyond the far level the surface uses the erf
coverage estimator from the cascades' Jacobian moments, so whitecaps continue to the vista without state.

**Appearance.** Coverage thresholds a ranked pattern, so thin foam is lace and thick foam closes. The pattern is
sampled by stochastic patches, the change that removed the stamped repeat. The pattern is a cooked 2D product
the game supplies through a `CONTENT-LOOP` lease.

**How much of the spike's open look work is inside S4.** Judged against the bar: caps with a bright core and a
feathered edge are inside, because flat white cutouts do not read as whitecaps. Pale water over submerged air
under and behind foam is inside, because it is most of how the bar's wake reads. A trail that widens and thins is
inside, as the dispersion term. Wind streaks are outside. The parked spikes stay parked.

### Optics

The sea composes C08's ordered segments itself, in one pass, in physical units:
`L = T_air · (R · L_reflected + (1 − R) · (S_water + T_water · L_behind)) + S_air`.

- **Air.** `ATMOSPHERE`'s aerial `(T, S)` from the camera to the surface point, applied once.
- **Reflection.** A provider slot with two providers built. The environment product is always present: the
  filtered sky around the reflected direction at the roughness the slope covariance gives. Screen-space
  reflection of registered bodies is first where a ray finds one. Ray query is a later provider and is not built.
- **The Sun.** The true Sun through the air, times the cloud factor once, into a microfacet lobe whose width is
  the slope covariance. Glitter, and its path at low Sun, come from the sea state and not from a texture.
- **Water.** In-scatter from the water type's absorption and backscatter (a game table), lit by the same Sun and
  sky. A thin crest seen against the Sun passes light through. A body under water is seen through
  `T_water = exp(−c·d)` per channel along the refracted path, so it tints and fades with depth.
- **Exposure.** The lent scale multiplies `L` once, as the sea writes its colour, as the forward pass does.

**Bodies in the water: one copy, and where it is read.** An open sea is optically thick. `L_behind` is zero
except where a body is under the surface, and a reflected ray meets something other than sky only at a body. So
the sea never copies the whole scene, on any recipe, which settles the port decision's Apple-tile obligation by
removing the work, and it does not request `EFFECT-GRAPH`'s shared full-screen copy. It works from the bodies a
scene registers, each with a world box, which the craft and floats carry in their body records. Opaque geometry
that is not registered is neither reflected nor seen through the water, and takes the environment.

*The source.* Before the surface pass, a gather pass copies opaque colour and linear depth inside each on-screen
body's **source rectangle**: the screen bounds of its box, padded by the filter's reach. Nothing else is copied,
and with no body on screen the pass does not run. Scene depth cannot be sampled in the pass that writes it, which
is the second reason the gather is its own pass.

*The receiving region.* The sea pixels that can show a body are not the body's own rectangle: a reflection lies
below the waterline, outside the silhouette. Two world volumes bound them, with the mean sea level at height 0,
`a` the largest surface height the sea state gives this tick, `θ` the slope allowance and `ℓ` the reach (below):

- *reflection:* the body's box, extended down to `−(top + 2a + 2ℓ sin θ)` and sideways by
  `2(top + a + ℓ sin θ) sin θ`. A pixel shows a reflection exactly when its view ray, continued through the
  surface, passes through the body's mirror image in that pixel's tangent plane, and this volume holds every such
  image for a surface point within `a` of the mean level and a normal within `θ` of vertical;
- *refraction:* the slab of surface heights over the box, grown sideways by `d · tan(48.8° + θ)`, where `d` is
  how far the box reaches below the highest surface and 48.8° is the largest angle a ray can take into water.

The screen bounds of both volumes are the body's **receiving rectangle**; a volume that reaches the camera's
plane takes the whole screen. The rectangles are computed on the CPU each frame and mark a coarse screen mask of
which bodies each part of the screen may show. On a calm sea a receiving rectangle is the body and its mirror
image, two to three times the body's own; in a gale it can be most of the screen. What stays bounded there is the
work in it, not its area.

*The mapping.* Where the mask marks a body, a sea pixel tests its reflected ray and its refracted ray against
that body's box. A ray that misses the box does no more. A ray that meets it is marched across that segment
only, in screen space against the gathered depth, for a bounded number of steps. A hit reads the gathered colour
at the hit's own texel, which lies inside the source rectangle because the hit is on the body. A miss, a hit off
the screen, and a sample whose depth is not inside that body's box take the environment for a reflected ray and
the water alone for a refracted one.

*The bounded approximation.* A reflected ray is followed for at most the reach, and a pixel whose normal tilts
past the slope allowance takes the environment; each fades over its last quarter, so neither leaves an edge. The
first profile's reach is 64 m and its allowance 20°, and both are knobs. On a calm sea a body is whole in
reflection while the ray from the water to its top is no longer than the reach: a craft 3 m tall is whole out to
about 85 m from a camera 1 m up, 190 m from 6 m up, and 900 m from 40 m up. Only what the camera sees of a body
can be reflected or seen through the water, so a body's underside never shows; that is screen-space reflection's
own limit.

**What the gather holds.** The scene target does not hold radiance. The forward pass wrote
`(T_c · L_body + S_c) · E` there, where `(T_c, S_c)` is its aerial transport from the camera to the body and `E`
the exposure scale. Used as `L_behind` or `L_reflected` as it stands, that colour would be exposed twice and
would carry the camera's air into a second air segment. The gather undoes both as it copies, with the lent
operations that wrote them:

`L_body = max(colour / E − S_c, 0) / T_c`,

where `(T_c, S_c)` is `ATMOSPHERE`'s aerial lookup at the texel's screen position and its gathered distance. It
stores the result in the scene target's pre-exposed range, and the surface pass divides the scale out as it
reads. What a ray then reads is radiance leaving the body toward the camera, unexposed and with no air on it. No
new product is asked of `ATMOSPHERE`. The rays use it so:

- *Refracted:* `L_behind = L_body · exp(−c·z)`, with `z` the hit's depth below the surface. The forward pass lit
  the body as if it stood in air, and this is the water its light crossed on the way down, taken vertically. The
  view path's `T_water` and the water's in-scatter are the equation's own, applied once.
- *Reflected:* `L_reflected = L_body`. The equation's `T_air` and `S_air` carry the air from the camera to the
  surface point, once. The air between the water and the body is left out, because `ATMOSPHERE` lends transport
  from the camera only; that segment is no longer than the reach, over which its air passes more than 99.7% at
  every wavelength (part E).

The declared approximations, and where they hold:

- Radiance toward the camera stands in for radiance toward the water. It is exact for a matte surface and
  misplaces a highlight.
- The light's path down to a submerged body is taken vertically. The refracted Sun is never more than 48.8° from
  vertical, so the true path is at most 1.5 times as long.
- A body further than 4 km from the camera is not gathered and takes the environment. It is under a pixel tall
  there, and inside that range `T_c` stays far from zero.
- Where a scene's forward pass shadows its aerial in-scatter, the lent `S_c` is too large by at most the
  camera-to-body in-scatter; the clamp keeps the result from going negative. A texel the forward pass clamped at
  the half-float ceiling stays clamped.

### Capabilities, recipes, limits and knobs

- **Capabilities.** A scene's sea profile names the capabilities it wants. One it leaves out declares no pass and
  no resource, and the surface's shader is prepared without its reads, so it costs nothing. Changing the set is a
  profile change.

  | Capability | What it adds | Needs |
  |---|---|---|
  | Open sea, always present | Long waves, cascades, the field cache, the surface, and the optics of sky, Sun and water | |
  | Body query | The query dispatch, its ring and mailbox | |
  | Wake | The two wake levels and their sources | Body records |
  | Whitewater | The foam stocks, their births and transport, and the whitecap estimator beyond them | A foam pattern; hull births need body records, wake breaking needs the wake |
  | Body optics | The gather and the body rays | Body records |

  A sea with only the open sea shows no foam and floats nothing. It is the sea of a sunset demo.
- **One physical profile on every recipe.** Component count, cascade layout and loop, wake levels and query
  semantics are the profile's, never a quality setting (C04). Chrome and the Mac float the same body on the same
  sea.
- **Limits are inputs.** Transform length, workgroup memory, storage formats and counts, and sampled textures per
  stage are read from the capability record and refused at preparation when exceeded. Levels and endpoints are
  array layers so the surface pass stays inside sixteen sampled textures.
- **Recipes.** Both recipes run the portable kernels first. An Apple-tile variant (f16 working precision, a
  different transform layout) is promoted only by a paired measured win with the oracle still inside tolerance.
- **Knobs.** Each is a named runtime value in `EFFECT-GRAPH`'s list with a declared invalidation: surface mesh
  density; field-cache reach; the cadence of the field cache and of cascades below the profile's smallest force
  wavelength; foam cadence and level size; reflection steps, reach and slope allowance. A layer evaluated less
  often than every frame is carried between evaluations by its rate of change, with its slopes held. None changes
  what a query returns.

### Check that it can work

- **S1.** No tile: the long waves are not periodic, and the cascades' joint period is far beyond the view. The
  accepted Broadsides W1 sea, which has this layering, is the evidence that it reads as open water. Vista: at 5 km a 150 m swell is under a pixel, so the vista is
  slope statistics, which the moments carry. Risk 2 covers a visible 160 m repeat.
- **S2 and the oracle.** `long_wave_precision.py`: 192 components, 6 m significant height, points to 4 km from
  an anchor 50 km from the origin after 24 hours. The wrapped f32 evaluation differs from f64 by 0.05 mm, and by
  0.5 mm with every cosine off by 10⁻⁴; an unwrapped f32 evaluation is off by 18 mm. **The declared tolerance is
  2 mm** in height for the long waves at any query point.
- **S3.** `wake_proof.py`, with a Gaussian stand-in for a 6 m hull. Transverse wavelength at 3 and 6 m/s: 5.72
  and 23.44 m against `2πU²/g` of 5.77 and 23.07 (−0.8%, +1.6%). Ridge half-angle at 3 m/s (Fr 0.39): 20.9°
  against Kelvin's 19.47°; at 6 m/s (Fr 0.78): 9.6°, narrower as Rabaud and Moisy report. With the source removed
  a tracked crest moved 6.00 m in 2 s, its phase speed, and after 10 s waves had left through the border with no
  wrap. Two banded levels against one fine level through a 60° turn: 2.2% RMS.
- **S4.** The foam spike's blind verdicts: the stochastic sampler removed the repeat; biased onset with swept
  birth gave caps at gale, fewer at fresh breeze and none on a glassy sea. Its open defects are the look work
  named above.
- **S5.** Every light term reads a product `ATMOSPHERE` M5 lends in physical units, so a seam with terrain can
  only come from the sea's own shading. The one input that is not physical, the copied scene colour, is turned
  back with the lookup and scale that wrote it: in part E the converted gather gives the true stored colour,
  1.142, where the copy used as radiance gives 0.354 and the copy with only exposure divided out 1.001. The
  receiving rectangles are tested in part C against rays reflected and refracted on the CPU, for cameras 1 to
  40 m up, bodies 12 to 100 m off, and seas from flat to 3 m of amplitude with a 20° allowance: in 36 cases, no
  ray that met the body came from outside its receiving rectangle, and in every case some came from outside the
  body's own rectangle, 92 to 2,010 of those sampled. The design review's case, a 2 m body 20 m from a camera 2 m up, is among them.
- **S6.** Unrun. `EFFECT-GRAPH` M3 proves motion and reactive reach reconstruction; how the sea looks after it is
  first seen in M9.
- **Time.** `first_principles_proof.py`. Part F: after 24 hours a phase formed from f32 seconds is up to 0.26 rad
  wrong, and the loop's integer phase 7 × 10⁻⁶ rad; rounding to whole cycles in 1,024 s changes a bin's speed by
  at most 0.12%. Part G: a blend of endpoints one tick apart misstates a 0.4 m wave by 0.53% of its amplitude and
  evaluation at the display's time by nothing; a cascade height carried three ticks by its rate of change is off
  by 0.39% for a 20 m wave, 0.86% for 9 m and 3.8% for 2 m.
- **Hitches.** Parts A, B and D, reported under
  [time](#time-functions-are-evaluated-at-the-displays-time-state-steps-on-the-tick), [the wake](#the-wake) and
  [whitewater](#whitewater).
- **Cost.** Unrun. M3 is the first measurement. Until the showcase's own extent exists, records are taken at
  2560×1440, the extent Broadsides was measured at.

## The system as it runs

| State | Owner |
|---|---|
| Spectrum, components, the f64 evaluators, the conformance model | `thrum-sea-model` (this module), no GPU dependency |
| The sea's GPU fields and state, passes, the query ring, the surface, optics | `thrum-sea` (this module), an effect on `EFFECT-GRAPH`'s seam |
| Presets, water types, the craft and floats, hull footprints, foam pattern, body integration, controls | The example scene `examples/sea` (package `sea-scene`); never in engine crates or their tests |
| Stages, state commit, cadence, knob list | `EFFECT-GRAPH` |
| Sky, Sun, aerial, reflection product, cloud factor | `ATMOSPHERE` |
| Motion and reactive targets, history, reconstruction, the governor | `APPLE-PRESENTATION` |
| The foam pattern's residency | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's body records, query points and sea parameters, and the
long-wave phases and loop time for the frame's `t`. `simulate`, every frame: the cascades and the field cache at
`t`. `simulate`, only on a frame that advances the tick: the wake step with its sources; foam births and
transport; the query dispatch and its copy to the ring. Forward, sky and stars draw. `after-opaque`: the gather,
if a body is on screen; the surface, which follows body rays where the mask marks them. Reconstruction, tone and
present follow. On submission the sea commits its tick and each endpoint's tick. A scene without a capability
runs none of its steps.

**The example scene** is a `Game` on `thrum-app`'s shell with a ticking world of its own, as `examples/v1-game`
is, because the sky kit's world never ticks. It uses `thrum-controls`, a parameter file and
`./dev scene examples/sea [--web]`. Its craft, floats and foam pattern are the example's content: made here, or
copied from Broadsides where [that serves](#what-broadsides-offers).

**Creator journey.** A scene adds the sea contribution with a profile that names its capabilities. A scene that
floats things puts sea parameters and body records in its tick page and reads answers from the mailbox; a scene
that only shows the sea sets wind, swell and the Sun. Changing wind or swell needs no rebuild. Changing the
profile rebuilds the sea and resets wake and foam.

## What the sea asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-sea), built by this module's builder after
`EFFECT-GRAPH:M3` is Done, with `EFFECT-GRAPH`'s items unchanged and its tests kept passing:

1. An `after-opaque` pass may write scene depth (M3).
2. An effect's own pass may read scene colour and scene depth as textures and write only the effect's targets
   (M8).

3. Whatever the shell, the page, the scene packaging or the capture tap assume of the V1 game or of the sky
   kit's still world is made a property of the game (M3 on the Mac, M4 in Chrome).

If `EFFECT-GRAPH` M3 as built already allows 1 or 2, nothing is added. Everything else is used as built: the
lease of one linear 2D texture, the lent environment products and cloud factor, the temporal contribution, the
typed controls and parameter files.

## Cost account

Estimates, unmeasured, at a 60 Hz tick. Broadsides' comparators are measured there, not here.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Long-wave phases | Frame, CPU | 192 components | Microseconds | |
| Cascades | Frame | 3 × 256², packed transforms for displacement, derivatives and rate of change, two dispatches each way | About 0.4 ms | About 0.5 ms for four, as a whole-frame difference |
| Field cache | Frame | 8 levels × 128², about 18 M evaluations | 0.2–0.3 ms | |
| Wake | Tick | 2 × 256², one complex transform each way per level | Under 0.2 ms | About 3.8 ms a tick in its lake, as an interval |
| Foam | Tick | 2 × 512² × 4 stocks | Under 0.2 ms | About 2.2 ms for first deposition |
| Query | Tick | Up to 2,048 points, 32 B each | Under 0.1 ms; 64 KiB back | 0.03 ms |
| Gather | Frame | The source rectangles of bodies on screen | Proportional to the bodies' own pixels; zero with none | A whole-scene copy and a 3 ms trace |
| Body rays | Frame, inside the surface pass | Sea pixels the mask marks: two ray-against-box tests per marked body, and a short march only for a ray that meets a box | Unmeasured; M8 records it calm and in a gale with zero, one and nine bodies | |
| Surface | Frame | Sea pixels at the render extent, about 200 k vertices; one read set for each ambient layer, two for the wake and foam | The largest term; M3 measures it | 10.5 ms at 1440p after the copy |
| Memory | | Cascades, working sets, wake, foam, cache | About 35 MiB, and the gather target, whose layout is the builder's: addressed by screen position at 2560×1440 it is about 44 MiB more, packed by rectangle far less | |

Wake, foam and query work lands on alternate frames at 120 Hz, so both frame kinds are recorded.

**Why the ambient layers run every frame.** Evaluating them on the tick and blending two copies would save about
0.7 ms on every second frame at 120 Hz and nothing in Chrome, and would add a second read of every ambient layer
to every sea pixel of every frame. At the 2.35 ns a pixel Broadsides measured for its surface, a full screen of
sea at 2560×1440 costs 8.7 ms, so second reads that added a tenth to it would cost 0.9 ms on every frame. Both
figures are estimates. M3's record has the surface's real cost, and the cadence knobs lower the ambient rate if a
record favours that.

**Records and tripwires, never gates** ([brief](../BRIEF.md#goal-and-exit-rule), decision 3). `scripts/sea_cost.py`,
created in M3, takes a paired record of one scene at one view, capture off: the scene with the sea and with it
absent, on tick frames and on repeat frames, giving the whole frame, each sea pass's GPU time by name, the bytes
of each resource and the load on the machine. A record trips a wire when the whole frame is over the display's
refresh interval, when a pass is about 25% or more over its last comparable record, or when a cost grows faster
than the work that was added. A tripped wire is investigated in the milestone that tripped it, and the record
says what was measured to find the cause and what was fixed or left as a follow-up in the log. Visible stutter is
judged in the `realtime` inspections.

The ambition is the display's full refresh, and whole-frame tuning is the later performance phase. **No timing
figure decides whether an item passes, whether a milestone is Done or whether the next one starts.**
`sea_cost.py check` measures the scene again and fails only when the scene cannot be measured, when the committed
record lacks part of what a paired record holds, or when a wire the record trips has no investigation beside it.
Presented cadence on the built-in panel is measured once, attended, in M10.

## Risks

1. **The surface pass costs too much.** Early sign: M3's record. The plausible wrong fix is dropping layers from
   the pixel shader. The designed ones: fewer texture reads through packing, cascades fading to moments sooner,
   and a coarser mesh. M3 tries the ones its record points at and records what each did. Building goes on either
   way; a representation the record shows to be wrong goes to the log as a proposal for the planner.
2. **A 160 m repeat shows from 40 m up.** Early sign: M3's vista capture. Fallback: a larger first patch under a
   profile the portable limits still admit, designed here first.
3. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M2 runs under WebGPU's default
   limits on the Mac, and M4 is the first real Chrome run.
4. **Self-interaction through the query.** A body bouncing on its own wake. Early sign: M6's craft at rest must
   settle. The dent correction is the design's answer; the wrong fix is filtering the craft out of the wake.
5. **Foam reads as grit or cutouts**, the spike's open defects. Early sign: M7's first captures. The mechanism is
   settled; the look is tuned against the inspection list, and three failed looks on one cue go to the log.
6. **The sea writes depth and a later effect disagrees.** `CLOUDS` is planned after this; the ruling fixes the
   order.
7. **The example's shell glue is larger than the sea.** Early sign: M3. A ticking entry in the sky kit is a
   proposal to `CREATOR` in the log, not built here.
8. **Body rays cost too much where receiving rectangles fill the screen.** Early sign: M8's record in a gale with
   nine bodies. The designed remedies are a shorter reach and a smaller allowance as the sea's slope variance
   rises, since a rough sea blurs a body's reflection away. The wrong fix is a march over every sea pixel.
9. **The ambient layers cost more every frame than the reads they save.** Early sign: M3's record on repeat
   frames. The designed remedy is the cadence knobs, which carry a layer by its rate of change. The wrong fix is
   a second copy blended in the surface pass.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. `ios-sim` is not a target. Chrome checks need a
  browser launch outside the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
  Never substitute a Mac result.
- **Capture.** Inspected items use T2 engine capture in the mode they name, at 2560×1440 until the showcase's
  extent exists. An item gives the cue, the views and the range. The route, the framing and the length that show
  them are the builder's and the reviewer's to choose; frames are judged whole and at native size, and motion as
  consecutive frames at 1x. The example wires the tap as `examples/v1-game` does, under ruling 3 where the tap or
  the shell assumes the V1 game.
- **Cost records.** Each milestone that adds frame work takes [the paired record](#cost-account) again, beside
  the last record for the same view. A tripped wire is investigated and written up there. It never ends a
  session `BLOCKED`.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/sea-<milestone>/`, selected evidence in
  `docs/evidence/SEA/`.
- **Exact baselines.** The sea adds crates and an example beside the existing scenes and moves no retained
  image. Every milestone carries the verifier as its claim-time check
  ([rules](../ARCHITECTURE.md#exact-baselines)).
- **Broadsides.** A resource, read at the commits in [what Broadsides offers](#what-broadsides-offers) and never
  changed. A builder may copy its code, shaders and assets as they are wherever they implement a mechanism of this
  design, and writes from this design and the published sources wherever they do not: the design decides, and a
  file is never a reason to change a mechanism. What a milestone has already written here is not replaced by a
  copy without a reason an item gives. Every copied or translated file is listed in
  `docs/evidence/SEA/PROVENANCE.md` with its source commit and path. The owner's words of 14 November cover what
  Broadsides' authors wrote. Material in Broadsides that came from elsewhere keeps its own licence, which is
  recorded with it, and none that is GPL or share-alike is copied. A copied kernel is held to this module's
  oracles and items like any other code; Broadsides' tests do not stand in for them. Scheduling, allocation and
  host glue are written for Thrum's seam.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** A ledger row for `E1.water` is added in M3 and kept current, with implementation, correctness,
  visuals and device apart.

## Open questions

| Question | Settled by |
|---|---|
| What does the surface pass cost against the refresh interval? | M3 records it; M10 records it at the showcase extent on both recipes. Tuning is the later performance phase |
| Do the wake's and the foam's two endpoints serve `EFFECT-GRAPH`'s cadence rules as built? | M6; a failure is a proposal there |
| Is evaluating the ambient layers every frame cheaper than a second read of them in every sea pixel? | M3's record, on tick frames and repeat frames |
| What do the body rays cost in a gale, where receiving rectangles are large? | M8 records it |
| What tolerance do the cascades and the wake meet against their f64 references on Metal and in Chrome? | M2 and M6 measure first, then assert the recorded bound |
| How many sampled textures does the lent environment group use? | M3 counts against the record |
| Is an Apple-tile kernel variant a measured win? | M10 |
