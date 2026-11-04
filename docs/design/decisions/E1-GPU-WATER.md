# GPU water: one GPU simulation with a fully supported wake

5 November 2026. **Selected direction by the user. Implementation follows G7.**

This record changes who owns water state. It amends these clauses, which now point here:
* the spectral sea profile in the [sea and cloud port decision](SEA-AND-CLOUD-PORT.md);
* the CPU-authority clause of [A04](../ARCHITECTURE-DECISIONS.md#a04--different-water-models-have-explicit-physical-boundaries);
* the CPU execution of [E1's initial operators](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river);
* the CPU authority and query rules in [WATER](../modules/WATER.md), [WATER-DYNAMICS](../modules/WATER-DYNAMICS.md) and
  [WATER-INTEGRATION](../modules/WATER-INTEGRATION.md).

The numerical methods those documents select still stand. What changes is where they run and what water promises
to replay. Optics, ownership, hardware obligations and porting policy are unchanged.

## Requirement

The user decided on 5 November 2026:
* **Water does not need exact replay.** It must behave the same way every run, but not bit for bit across devices,
  drivers or quality settings.
* **All water simulation runs on the GPU.** This covers the sea, wakes, rivers, lakes, pools and whitewater.
* **Wakes are a fully supported interactive capability.** They are not a presentation residual.

The CPU-authority design existed to give exact replay, headless servers and replicas the same water. Without the
exact-replay requirement, it cost what players see: waves under 20 m and wakes could not move bodies, and rivers had
to fit a CPU budget.

## Decision

**One GPU authority per water domain.** E1 owns one GPU state for each domain. Bodies, appearance, optics and audio
all read it; nothing keeps a CPU twin of live water.

| Domain | GPU state | Method |
|---|---|---|
| Open sea | Analytic long waves + FFT short-wave cascades | The port decision's spectral profile, evaluated once per tick into a camera-following field clipmap |
| Sea interaction | Two-level dispersive wake solver | eWave-style spectral propagation (Tessendorf, SIGGRAPH 2014 course): a near level of about 0.25 m cells around the camera focus, and a far level of about 2 m cells, 256² each |
| Rivers, lakes, pools | Finite shallow water / generalised heightfield | The methods selected in [WATER-DYNAMICS](../modules/WATER-DYNAMICS.md) and the initial HLL patch, run as GPU compute |
| Whitewater | Fresh foam, aged foam, aeration, slick | One transported field per domain, fed by one deposition pass |

The CPU keeps one analytic long-wave evaluator. It serves only as the **test oracle** for the GPU sea and as the
**no-GPU fallback** described below.

Water advances on committed tick time with bounded catch-up, never on frame `dt`. Its f32 state may differ between
devices. Tests use analytic oracles and tolerances, not bit equality.

## The wake

The wake is physical water state.
* Bodies feel it: a boat crossing another's wake pitches and rolls through the same query as any wave.
* It carries foam: wake velocity advects the whitewater field.
* It outlives its source: a stopped ship's wake keeps spreading and decaying.

**Sources.**
* Each hull writes its pressure footprint once per tick into every interaction level that covers it. The footprint
  comes from the hull geometry, using the reference-immersion surrogate in WATER-DYNAMICS.
* Impacts, falls and spray landings write into the same levels.
* Every source has a stable ID and generation. The source cap comes from the capability registry; overflow is
  counted and reported.

**Coverage.**
* The near level resolves wakes down to about 1 m wavelength around the focus.
* The far level carries wakes of at least about 4 m across about 512 m.
* Beyond the far level, each moving body emits analytic Kelvin wake packets, evaluated on the GPU. They keep distant
  fleets waking at declared lower fidelity, and distant bodies still feel each other through them.
* Inland domains carry wakes in their own shallow-water state; no separate wake solver runs there.

**Whitewater from the wake.** The deposition pass turns causes into birth rates, never binary stamps:
* wake and wave breaking, from slope or compression;
* bow and shoulder break;
* a swept turbulent stern ribbon;
* wake cusps;
* powered wash and impacts.

Foam and bubbles decay on separate timescales. Rendering samples the wake per pixel, never only at mesh vertices.

## Bodies and forces

P3 still integrates every body on the CPU, and the hull reaction still enters the water once, as the source above.
Forces now come from the GPU.

* **Batched query.** Each tick, E1 dispatches one compute query covering every hull and floater sample point. It
  returns height, normal, orbital velocity and dynamic pressure from the full GPU state: long waves, cascades, both
  interaction levels, Kelvin packets and inland domains.
* **Asynchronous readback.** Results arrive one or two ticks later, through a ring of mapped buffers. Physics never
  waits on the GPU. If a result is late, P3 uses the latest one extrapolated by its time derivative, and counts the
  miss.
* **Latency compensation.** Analytic and cascade terms are evaluated at the tick when P3 will use them. Interaction
  terms are extrapolated with their stored velocity.
* **Declared error.** For latency `Δt` the remaining error is the interaction field's change over `Δt`. For a wave
  of frequency `ω` that is a phase error of at most `ωΔt`. Report it per profile.
* **Cost.** At 30 ships × 64 points × 32 bytes the readback is about 61 KB per tick. Its budget is under 0.1 ms of
  GPU time and no stall.

**Synchronous readback stays forbidden.**

## What this changes elsewhere

* **Replay and prediction.** Water, and every body whose forces read water, leave the exact re-simulation set.
  * G2's bounded replay covers the rest of the world.
  * Replays restore water-coupled bodies from recorded state.
  * N1 already excludes water from prediction, and that does not change.
  * Corrections invalidate water histories as the [contracts](../CONTRACTS.md) already require.
* **Saves.**
  * P7 records the water forcing (seed, spectrum, tide, sources, `t_sim`) and the persistent GPU fields: interaction
    levels, shallow-water state and whitewater.
  * The fields are captured asynchronously at f16 where the error is declared.
  * A load without a field capture restarts that field calm, and says so.
* **Servers and headless play.**
  * A host that simulates water needs a GPU compute adapter. A software adapter is acceptable where its cost is
    measured.
  * Without one, E1 runs the **no-GPU fallback**: the CPU analytic long-wave sea only, with no wakes, cascades or
    inland flow. It is a declared, separate physical profile; bodies still float.
  * Replicas receive body state from the authority and run their own GPU water for presentation and local floaters.
    Their water may differ visually from the authority's.
* **Graphics off.** GPU water simulation is physics and keeps running. Turning drawing off retires only optical and
  drawing work.

## Performance targets

These are 1440p targets on an M3 Max, carried from the Broadsides work:
* water surface ≤1.5 ms;
* waves, interaction and foam ≤0.8 ms;
* caustics ≤0.3 ms;
* a visible river ≤0.5 ms;
* body query and readback ≤0.1 ms;
* CPU water work ≤1 ms per tick.

Qualify each recipe subtractively against the whole frame, as the [hardware-native decision](HARDWARE-NATIVE-ENGINE.md)
requires. No measurement or device qualification is claimed here.

## Proof order

1. A GPU sea that matches the CPU oracle within tolerance, and a body floating on it through the asynchronous query.
2. One ship's wake: visible V arms and stern trail, foam that turns to lace and fades, and the wake surviving source-off.
3. Two bodies crossing each other's wakes, each responding through the query, with stated latency error.
4. An inland domain with a floater and a wake; then save, load, graphics-off and the no-GPU fallback.
5. A fleet of 20 or more ships visibly waking within budget.

Reject any step on scheduled bobbing, a brightness-mask wake, a synchronous readback or a stall.
