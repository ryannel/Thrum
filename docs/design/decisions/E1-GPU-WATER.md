# Water: GPU simulation, interactive wakes and shared multiplayer water

5 November 2026. **Selected direction by the user. Implementation follows G7.**

This record owns:
* where water runs;
* how bodies feel it;
* what water promises to replay;
* how players share it in multiplayer.

**14 November 2026.** Water is now designed as one system for every body of water in
[WATERS' design](../../plans/WATERS/DESIGN.md), which selects the methods, with the wave field in
[SEA's](../../plans/SEA/DESIGN.md). It keeps this record's requirements and seams with one departure, which is
[open for the owner](../../plans/WATERS/PLAN.md#open-for-the-human): a river's bulk flow is worked out once and
stored, not simulated on the GPU while the game runs. Wakes, rings and foam on a river still run on the GPU.
Where the method tables below differ from that design, the design holds.

Before that date these documents selected the numerical methods: the [sea and cloud port decision](SEA-AND-CLOUD-PORT.md),
[E1's initial operators](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river), and
[WATER-DYNAMICS](../modules/WATER-DYNAMICS.md), [WATER-APPEARANCE](../modules/WATER-APPEARANCE.md) and
[WATER-OPTICS](../modules/WATER-OPTICS.md).

## Requirements

* **Single player gets the best water the device can run.** Multiplayer machinery costs it nothing.
* **All water simulation runs on the GPU:** sea, wakes, rivers, lakes, pools and whitewater.
* **Wakes are fully supported interactive water.** Bodies feel them, they carry foam, and they outlive their source.
* **Water needs no exact replay.** It behaves the same way every run, but not bit for bit across devices, drivers or
  quality settings.
* **Multiplayer players share the water that matters.** Everyone meets the same huge wave at the same time, place and
  height. Everyone feels the same bump crossing a wake on a lake, and the same river current. Foam, spray, fine chop
  and caustics may differ between machines.

## Principle: share causes, not water state

Water state is large, megabytes of grids changing every tick. The causes that drive it are small:
* a sea preset and seed;
* the tick time;
* a few scheduled events;
* the bodies moving through it;
* the impacts landing in it.

The water that bodies feel is also well behaved:
* wave and wake propagation is linear;
* shallow water and wakes are damped.

So two machines fed the same causes stay together instead of drifting apart. **Every machine runs full-quality GPU
water locally, and the network carries only causes.** Single player is the case where every cause is local. It runs
the same code with nothing to send.

## GPU water

Each water domain has one GPU state. Bodies, appearance, optics and audio all read it, and nothing keeps a CPU copy
of live water.

| Domain | GPU state | Method |
|---|---|---|
| Waves on any body | Analytic long waves + FFT short-wave cascades, at each body's gains | [SEA's design](../../plans/SEA/DESIGN.md): functions of time, evaluated every presented frame at the display's time; [gains](../../plans/WATERS/DESIGN.md#waves-on-a-body-gains) by fetch and depth |
| Interaction on any body | Two-level dispersive solver | eWave-style spectral propagation (Tessendorf, SIGGRAPH 2014 course): a near level of about 0.25 m cells, a far level of about 2 m cells, 256² each, computed at a few depths and blended ([WATERS](../../plans/WATERS/DESIGN.md#interaction-waves)) |
| Lakes, pools, rivers | [The water map](../../plans/WATERS/DESIGN.md#the-water-map): a level, a bed and a current, prepared and not stepped | A lake is a level in a hollow; a river is a chart `RIVERS`' solver prepares once |
| Whitewater | Fresh foam, aged foam, aeration, slick | One transported field per domain, fed by one deposition pass |

The CPU keeps a f64 evaluator of the analytic long waves:
* as the **test oracle** for the GPU sea;
* as part of the **conformance model** for authorities without a GPU (see Multiplayer).

Water state (wakes, inland flow, whitewater) advances on committed tick time with bounded catch-up, never on frame
`dt`. The open sea's waves hold no state: they are evaluated at the display's time, which is the committed tick
and the frame's interpolation alpha. Tests use analytic oracles and
tolerances, not bit equality.

## The wake

**Sources.**
* Each hull writes its pressure footprint once per tick into every interaction level that covers it. The footprint
  comes from the hull geometry, using the reference-immersion surrogate in WATER-DYNAMICS.
* Impacts, falls and spray landings write into the same levels.
* Every source has a stable ID and generation. The source cap comes from the capability registry; overflow is
  counted and reported.

**Coverage.**
* The near level resolves wakes down to about 1 m wavelength.
* The far level carries wakes of at least about 4 m across about 512 m.
* Beyond the far level, each moving body emits analytic Kelvin wake packets, evaluated on the GPU. Distant fleets
  keep waking at declared lower fidelity, and distant bodies still feel each other.
* Inland domains carry wakes in their own shallow-water state.

**Coverage that feeds forces follows bodies, not the camera.** Two players looking at different places must get the
same physics.
* In single player the near level centres on the player's vessel, which the camera follows.
* A multiplayer authority runs a near level around each player vessel.
* The camera may add detail that is only drawn.

**Whitewater from the wake.** The deposition pass turns causes into birth rates, never binary stamps:
* wake and wave breaking, from slope or compression;
* bow and shoulder break;
* a swept turbulent stern ribbon;
* wake cusps;
* powered wash and impacts.

Foam and bubbles decay on separate timescales. Rendering samples the wake per pixel, never only at mesh vertices.

## Bodies and forces

P3 integrates every body on the CPU, and each hull's reaction enters the water once, as its wake source. Forces come
from a **force provider** chosen per world:

| Provider | Used by | What it reads |
|---|---|---|
| GPU query | Single player, a player-hosted session's host, a server with a GPU | The full GPU state |
| Conformance model | An authority with no GPU | CPU evaluation at hull points only (see Multiplayer) |

**The GPU query:**
* **Batched query.** Each tick, one compute dispatch covers every hull and floater sample point. It returns height,
  normal, orbital velocity and dynamic pressure from every water layer.
* **Asynchronous readback.** Results arrive one or two ticks later, through a ring of mapped buffers. Physics never
  waits. A late result is extrapolated by its time derivative, and the miss is counted.
* **Latency compensation.** Analytic and cascade terms are evaluated at the tick when P3 uses them. Interaction terms
  are extrapolated with their stored velocity.
* **Declared error.** For latency `Δt` the remaining error is the interaction field's change over `Δt`. For a wave
  of frequency `ω` that is a phase error of at most `ωΔt`. Report it per profile.
* **Cost.** At 30 ships × 64 points × 32 bytes the readback is about 61 KB per tick. Its budget is under 0.1 ms of
  GPU time and no stall.

**Synchronous readback is forbidden.**

## Multiplayer

### Three kinds of water

| Kind | Examples | How players agree | Network cost |
|---|---|---|---|
| **Same by construction** | Sea state, swell, tide, weather-driven sea, huge waves | A function of the preset, the seed and the shared tick time. A huge wave is a scheduled event, evaluated identically everywhere | Event records only |
| **Same from shared causes** | Wakes, impact rings, river and lake flow | Every machine feeds the same causes into its own solver: replicated body state, impact events and inflow changes | Already paid by body state |
| **Local only** | Foam, bubbles, spray, fine chop beyond the force band, caustics, decorative floaters | Not shared | None |

### Causes

Water causes are records with:
* a stable ID and generation;
* a tick;
* their parameters.

The same record type serves local play and the network.

| Cause | Carried by |
|---|---|
| Sea preset, seed, tide | Session content, pinned at join |
| Tick time | N1's shared tick |
| Wave event: origin, direction, height, period, shape, start tick | A reliable event record |
| Hull source | Derived on every machine from replicated body state; never sent separately |
| Impact: point, energy, tick | The authoritative event that caused it, such as a shot landing |
| Inflow, level or gate change | A reliable event record |

**Wave events are scheduled ahead in simulation time,** never triggered "now" from a frame. The lead time must exceed
the largest admitted client latency plus the interpolation buffer, at least one second. A huge wave is naturally
spawned far away, so the lead costs nothing. Every machine then evaluates it from the same start tick.

**Wave phase is computed in f64 on the CPU** and wrapped to one period per component before upload. Every GPU then
draws the same wave to within millimetres, however long the session runs.

Remote bodies are presented 100 ms behind the authority. A client therefore feeds their hull sources at each sample's
own tick, extrapolated to the present tick within N1's extrapolation envelope. The remaining offset of a remote wake
is about body speed × residual delay. It is declared, and corrected through the authority's forces.

### Who decides forces

**The authority computes every force,** from its own copy of the same water.

* A **player-hosted session or a GPU server** uses the GPU query, exactly as single player does.
* An **authority with no GPU** uses the **conformance model**, evaluated only at hull sample points:
  * the analytic swell and every scheduled wave event, exactly;
  * wakes as Kelvin packets from the replicated body tracks;
  * impact rings as analytic packets;
  * inland flow from a coarse CPU shallow-water state at hull scale.

  A conformance test requires clients' GPU water to match this model within tolerance for every wavelength at least
  as long as the smallest admitted hull. Shorter detail is local.

**Remote vessels** show the authority's interpolated pose. The client's water nearly matches the authority's, so they
sit on the waves it draws. A drawn-only waterline settle hides the residual, bounded by the conformance tolerance.

**A player's own vessel** may be predicted against the local water copy, which shares every cause with the authority.
Corrections are small and are smoothed under N1's correction policy. Prediction state is:
* the vessel's P3 state;
* the cause records up to the predicted tick;
* the local water's tick.

The local water is not rolled back. It differs from the authority's only within the conformance tolerance.

### Keeping copies together

* **Divergence test.** For each provider, run two copies with a tiny perturbation in their inputs and require them
  to stay within the conformance tolerance over a long session. Linear wake propagation and damped shallow water are
  expected to pass.
* **Correction stream.** A finite domain that fails the test, for example through chaotic wetting and drying, gets a
  coarse correction stream from the authority: about 32² heights and flows at about 1 Hz, around 1–2 KB/s. Clients
  nudge towards it. Only domains that need it pay for it.
* **No grid replication.** Clients never receive full water grids, and client pixels never supply authority forces
  or saved state.

## Saves, servers and graphics off

* **Replay.** Water, and every body whose forces read water, leave G2's exact re-simulation set. Replays restore
  water-coupled bodies from recorded state.
* **Saves.**
  * P7 records the causes: preset, seed, tide, `t_sim`, pending wave events and source state.
  * It also records the persistent GPU fields: interaction levels, shallow-water state and whitewater. These are
    captured asynchronously at f16 where the error is declared.
  * A load without a field capture restarts that field calm, and says so.
* **Headless.** A headless authority uses the GPU query when it has a GPU adapter, a software adapter included where
  its cost is measured. Otherwise it uses the conformance model.
* **Graphics off.** GPU water simulation keeps running. Only optical and drawing work retires.

## Performance

These are 1440p targets on an M3 Max, carried from the Broadsides work:
* water surface ≤1.5 ms;
* waves, interaction and foam ≤0.8 ms;
* caustics ≤0.3 ms;
* a visible river ≤0.5 ms;
* body query and readback ≤0.1 ms;
* CPU water work ≤1 ms per tick.

Multiplayer adds these costs:
* no water bandwidth beyond body state and occasional event records, plus correction streams only where the
  divergence test fails;
* an estimated 0.2 ms of GPU time per extra player vessel on the authority, for its near level;
* the conformance model's CPU cost on an authority without a GPU.

Qualify each recipe subtractively against the whole frame, as the [hardware-native decision](HARDWARE-NATIVE-ENGINE.md)
requires. No measurement or device qualification is claimed here.

## Proof order

1. A GPU sea that matches the CPU oracle within tolerance, and a body floating on it through the asynchronous query.
2. One ship's wake: visible V arms and stern trail, foam that turns to lace and fades, and the wake surviving
   source-off.
3. Two bodies crossing each other's wakes, each responding through the query, with stated latency error.
4. An inland domain with a floater and a wake; then save, load and graphics-off.
5. Two clients and an authority:
   * a scheduled huge wave reaches both clients within the height and timing tolerance;
   * a vessel crossing a lake wake gets the same bump on both;
   * the divergence test passes;
   * a no-GPU authority passes the conformance test.
6. A fleet of 20 or more ships visibly waking within budget, single player and multiplayer.

Reject any step that relies on:
* scheduled bobbing;
* a brightness-mask wake;
* a synchronous readback or a stall;
* camera-dependent forces;
* replicated water grids.
