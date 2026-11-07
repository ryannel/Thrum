# Independent model derivation — 2026-11-01

Written before opening `notes/engine-rebuild/`. This is a challenge set, not a selected engine design.

## Starting constraints

Reusable Bevy-independent 3D engine; overhead, isometric, third-person and first-person games on modern Mac, PC, browser and native mobile. Strong image quality and performance. A game may omit a visible sky but still use sun lighting, atmospheric extinction or cloud shadows. Different games may need different water and air models. No pure 2D or MMO target; ordinary networking is undecided. AI assists development, not shipped runtime behavior. Feature configuration need not imply universal runtime swappability or plugin packaging.

## First-principles model split

A phenomenon has at least four separable authorities: (1) authored geometry/material/volume boundaries; (2) world evolution and gameplay state; (3) query representation for consumers such as buoyancy, audio, AI, particles and lighting; (4) rendered light transport and appearance. A single provider may own several, but those are not automatically the same numerical field. Each combination needs stated consistency bounds and synchronization. A GPU surface that looks credible is not by itself a buoyancy surface; CPU reconstruction may match only a deliberately restricted spectral component.

Water regimes cannot be collapsed by one parameter set. Deep-water dispersive gravity waves obey omega^2 = g k tanh(k h) (capillarity adds sigma k^3/rho); at kh >> 1, phase speed changes with wavelength, while at kh << 1 it approaches sqrt(g h). A height-field free surface cannot represent overturning, spray, vertical jets, caves or separated falling sheets. Incompressible flow near obstacles and constrictions needs pressure/velocity evolution; a prescribed spectral height field has no mass-conserving response to a dam, shoreline or waterfall. A shallow-water depth/flux solver can represent inundation and long-wave propagation where horizontal scale greatly exceeds depth, but not resolved breaking or turbulence. Particle/volume methods carry different memory and compute costs and can support topology changes. Stylized/static/animated textures may suffice when consumers do not require physical dynamics. Thus water model choice is per use case/region/quality target, with explicit edge conditions and transition requirements; some models could coexist in one world.

The engine should separate water *regions* and their physical/query/render roles. A river, lake, surf zone, sea, puddle and waterfall might be adjacent yet use distinct models. A boundary between regimes needs a coupled exchange of relevant conserved quantities (at minimum mass flux and often momentum/energy) if gameplay depends on transfer. If models are merely visual, use explicit one-way driving and seam hiding, not a false conservation claim. Overlapping regions need priority/blending semantics for queries and visibility. Query fidelity should be declared: surface elevation/normal/velocity, depth, current, wetness, occupancy, submerged volume, or force; rates, latency, spatial/temporal resolution, deterministic guarantee and out-of-domain behavior. A consumer should request only the fidelity it needs and know when the answer is approximate or unavailable.

Atmosphere is similarly not one skybox. Sun/moon direction and irradiance, spectral or RGB aerial extinction, local fog/media, cloud optical depth and shadows, sky radiance/background, exposure/tonemapping, and weather time evolution may be separable. Physical atmosphere methods solve/approximate radiative transfer across scales; a cubemap or gradient may present an adequate background yet not provide consistent aerial perspective or lighting. Local volumetric media can overlap an atmosphere. A no-sky game can retain daylight and clouds' shadows while suppressing background rendering. The authority for light and media should be explicit so sky, reflections, surfaces, clouds and probes do not infer different suns. Alternative sky/air algorithms should be selected according to view range, camera altitude, sun motion, indoor/outdoor mixing and platform budget. Models with different parameter meanings should not be forced behind a universal live-swappable interface; shared consumed outputs might still be stable.

CPU/GPU ownership is architectural: simulation may be CPU authoritative and upload fields; GPU authoritative with asynchronous, delayed readbacks; or split coarse CPU query and fine GPU render fields. Cross-device deterministic gameplay and frequent physics queries argue against relying on GPU readback. Visual only systems can be GPU resident. The declared consistency envelope includes time phase, transform, precision, origin shifts and quality changes. Authoring/cooking must encode region extents, boundary and seam data, model-specific parameters and required derived assets (bathymetry, SDF/flow maps, spectra, LUTs, cloud fields), plus platform fallbacks and reproducible versions.

## Decisions to test against the existing design

1. Is model selection per game, per region, per view/quality, or all three? Which changes are build/load/live, and where does migration apply?
2. Is there one world authority for water topology/regions and one for sun/weather state, even when solvers/renderers differ?
3. Which state is shared at region boundaries, and which seams are visual approximations? What invalid combinations are ruled out?
4. Does the query contract express consumer-required quantity and fidelity/latency, or accidentally promise a universal exact height field?
5. Are render and physics representations explicitly distinct, aligned by a stated error budget, and synchronized across CPU/GPU?
6. Can sky/background be absent while light, aerial media and cloud shadow still operate? Are clouds, atmosphere, local fog and lighting composed in a specified order/space?
7. Do asset authoring/cooking and target profiles validate required maps, LUTs, query approximations, transitions and fallback behavior?
8. Are complex models chosen only where their unique phenomena matter, with whole-frame/scene and memory budgets including simultaneous regions and cameras?

## Competing architecture patterns to assess

- Single global solver with quality scaling: simple authority, weak for distinct topologies/regimes.
- Region typed providers under a common registry with typed capabilities/queries: supports mixed waters, but boundary adapters and conflict policy become first-class.
- Unified field/volume solver across all regimes: strongest physical coupling in theory, likely prohibitive for broad platform/scale range and may still need render approximation.
- Render-first layered effects plus coarse CPU water: efficient for many games, but cannot claim dynamic physical coupling without a separate model.
- Game-level selection of one environment stack: simpler and often valid; fails worlds needing coexisting sea/river/falls or indoor/outdoor/planet-scale atmospheric regimes.

No approach is selected here. The audit should avoid demanding plug-ins or a single universal interface where typed model composition better fits the phenomena.
