# Bounded authored E1 source — implementor dogfood, 2 November 2026

Status: paper build recipe, not runtime implementation or qualification. Its
constant-light source subset has a bounded provisional F3 staged proof; the
storm JSON and F4 products below remain proposals. Shared source/transport
semantics are selected in the
[environment design](../modules/ENVIRONMENT.md#selected-bounded-authored-source-and-light-handoff)
and [C08/C09](../CONTRACTS.md); the storm JSON, products and function
names below remain proposals pending F3/F4 admission. This applies
[composition](../COMPOSITION.md), [initial rendering](../RENDER-SPEC.md) and
the [source research](ENVIRONMENT-SOURCE-RESEARCH-2026-11-02.md). No E1 code,
renderer, physics, audio or device qualification ran for this recipe.

## Commission, bounded success and exclusions

An implementor should be able to author one source, cook it through ordinary F3/F4, tick it under F1, and make a roof, ground, finite pond, plant, P3 body, V3 light path and A1 listener read the same forcing. The defining interaction is *interval rain becomes exactly one receiver's mass while light/wind/audio and both views use the same source epoch*. R0 separately retains a constant-light path that allocates none of this optional weather machinery.

The first source provides a prescribed authored Sun, uniform-in-horizontal-position layered wind, a translated periodic optical-depth map, static piecewise-constant rain footprints with time-varying rates, optional constant gas thermodynamic facts, and an optional authored fog descriptor. It does not simulate weather or conserve atmospheric water. Physical precipitation is an identified exchange from an external reservoir. It offers no Moon, astronomical calendar, pollutant, physical thermal irradiance or cloud-density-volume capability unless a separately declared provider supplies one.

The first rain proof uses static triangles and a **fixed projected footprint finite pond inside retaining walls**. Its finite stage may change within the admitted range while remaining below the wall crest and beneath every overlapping roof; its projected water/ground receiver order cannot change. It is not the R3 changing-tide/wet-shoreline exposure proof. A tide overtopping a terrain patch, changing wet area, dynamic roof, tree interception, wall rain, porous canopy or oblique rain needs a different exposure operator. Required unsupported interaction rejects admission; it is not approximated by depositing on both old and new receivers.

Reference provider limits are proposed fixture-admission bounds: one 512 m ×512 m horizontal source region, wind height 0–1000 m, receiver height −10–500 m, world duration at most 3600 s, one Sun, at most 8 wind-height knots, 256 total scalar/vector temporal knots, 16 rate segments per driver, 64² cloud samples, 128² rain cells, 1024 receiver patches and 32768 nonzero exposure rows. They are not engine-wide maxima or measured capacity. This proof preloads its whole gameplay closure; no streaming or ML/network data ingest is needed.

## Proposed F3 source records

JSON is the ordinary editable source; use existing F3 source-carried type/version, stable reference and exact-byte admission mechanisms. The following payload examples are proposed type contents, not a second project manifest. Every `id`/`*_ref` below is a lowercase RFC-4122-form 128-bit UUID under FOUNDATION-SPEC. Human labels are optional `fields.label` text and never resolve references. The companion-record table below closes every reference in these examples; no name-to-ID lookup is permitted. These are proposed fixture source bytes, not files already created by this paper exercise.

The R0 payload can be just:

```json
{
  "type": "thrum.environment.authored",
  "schema": 1,
  "id": "10000000-0000-4000-8000-000000000001",
  "fields": {
  "profile": "constant-light-v1",
  "sun": {
    "light_ref": "10000000-0000-4000-8000-000000000002",
    "toward_source_unit": [0, 1, 0],
    "perpendicular_illuminance_lux": 100000,
    "unit_luminance_linear_rec709": [1, 1, 1],
    "angular_radius_rad": 0.00465,
    "direction_model": "geometric-straight",
    "applied_attenuation": []
  },
  "ambient_irradiance_lux": [1000, 1000, 1000],
  "background_radiance_cd_m2": [0, 0, 0],
  "unsupported": ["wind", "cloud_tau", "precipitation", "air", "thermal", "moon", "air_quality"]
  }
}
```

F2 accepts that profile only if retained consumers do not require the unsupported fields, or separately bound providers supply them. It has no time-update job or weather-grid allocations. Its 100000 lux directional source and [1000,1000,1000] lux ambient diffuse fill are authored fixture values, not measured daylight. The ambient field is hemispherical incident irradiance, isotropic with the same RGB value for every surface normal. V3 evaluates `(1-metallic)*base*ambient_irradiance_lux/pi`; 1/pi is the Lambert BRDF, not another radiance-to-irradiance conversion. It supplies no sky, specular environment or background; the separately authored R0 background is [0,0,0] cd/m². These exact fields/values were reconciled with the render dogfood owner. The colour vector satisfies RENDER-SPEC's `Y(c)=1` convention. V4 alone handles exposure/display.

A bounded authored storm payload is:

```json
{
  "type": "thrum.environment.authored",
  "schema": 2,
  "id": "10000000-0000-4000-8000-000000000010",
  "fields": {
  "profile": "authored-vertical-rain-v1",
  "world_support_m": {"min": [-256, -10, -256], "max": [256, 1000, 256]},
  "valid_ticks": [0, 216000],
  "day_phase": {
    "initial_seconds": 0,
    "rates": [{"from_tick": 0, "num": 1, "den": 1}, {"from_tick": 1800, "num": 120, "den": 1}]
  },
  "weather_phase": {
    "initial_seconds": 0,
    "rates": [{"from_tick": 0, "num": 1, "den": 1}, {"from_tick": 1800, "num": 1, "den": 2}]
  },
  "sun": {
    "light_ref": "10000000-0000-4000-8000-000000000011",
    "initial_toward_source_unit": [0, 1, 0],
    "rotation_axis_unit": [0, 0, 1],
    "period_phase_seconds": 86400,
    "peak_perpendicular_illuminance_lux": 100000,
    "intensity_envelope": "positive-world-y-squared",
    "unit_luminance_linear_rec709": [1, 1, 1],
    "angular_radius_rad": 0.00465,
    "direction_model": "geometric-straight",
    "applied_attenuation": []
  },
  "wind": {
    "reference_height_m": 10,
    "vertical_multiplier": [[0, 0], [10, 1], [1000, 2]],
    "weather_phase_vector_knots_mps": [[0, [6, 0, 0]], [60, [0, 0, -6]], [3600, [0, 0, -6]]]
  },
  "cloud": {
    "plane_height_m": 1000,
    "map_origin_xz_m": [0, 0],
    "spacing_xz_m": [64, 64],
    "periodic_node_dimensions": [2, 2],
    "vertical_tau_nodes_x_fastest": [0, 0.4, 0.4, 0.8],
    "advection_wind_height_m": 1000,
    "low_sun_policy": "authored-fade-to-clear-0.2-to-0.1"
  },
  "precipitation": {
    "origin_xz_m": [0, 0],
    "cell_size_xz_m": [32, 32],
    "dimensions": [2, 2],
    "cell_multiplier_x_fastest": [1, 1, 1, 1],
    "weather_phase_liquid_flux_knots_kg_m2_s": [[0, 0.002777777777777778], [3600, 0.002777777777777778]],
    "weather_phase_solid_flux_knots_kg_m2_s": [[0, 0], [3600, 0]],
    "exposure_ref": "10000000-0000-4000-8000-000000000012",
    "input_reservoir_ref": "10000000-0000-4000-8000-000000000013"
  },
  "air": {
    "temperature_k": 288.15,
    "local_absolute_pressure_pa": 101325,
    "vapour_gas_mass_fraction": 0.005,
    "saturation_law": "nws-liquid-magnus-bounded-v1"
  },
  "unsupported": ["moon", "thermal_irradiance", "air_quality", "cloud_density_volume", "optical_air", "terminal_sky"]
  }
}
```


The light references resolve to W1 source-identity records; the environment record owns their complete selected directional descriptor/driver, so the referenced record contributes no competing transform/intensity. The first storm fixture deliberately has **no optical air or terminal sky** and has a headless numeric proof and an optional two-view shadow-consumer proof using the same source. An R1/R3 air-render variant must add real ready fog/HDRI content through RENDER-SPEC; this paper does not invent a sky-asset hash or claim the broader rendered fixture is ready.

Every companion record uses `{id,type,schema:1,fields}` with the following exact IDs/types/field values. This table is an equivalent complete record specification; expansion to JSON is mechanical, not reference binding by label.

| UUID | Type; complete fields for this proposed fixture |
| --- | --- |
| `10000000-0000-4000-8000-000000000002` | `thrum.light.identity`; `{"label":"R0 light","driver_binding":"incoming-environment-sun"}` |
| `10000000-0000-4000-8000-000000000011` | `thrum.light.identity`; `{"label":"Shared Sun","driver_binding":"incoming-environment-sun"}` |
| `10000000-0000-4000-8000-000000000013` | `thrum.environment.external_reservoir`; `{"label":"Prescribed rain","material":"water","inventory":"unbounded-external","momentum_policy":"zero-horizontal-no-impact-force","energy_policy":"unrepresented"}` |
| `10000000-0000-4000-8000-000000000012` | `thrum.environment.vertical_exposure_recipe`; `{"label":"Static roof and enclosed pond","source_ref":"10000000-0000-4000-8000-000000000010","entry_world_y_m":10,"receiver_refs":["10000000-0000-4000-8000-000000000020","10000000-0000-4000-8000-000000000021","10000000-0000-4000-8000-000000000022","10000000-0000-4000-8000-000000000023"],"uncovered_sink_ref":"10000000-0000-4000-8000-000000000014"}` |
| `10000000-0000-4000-8000-000000000014` | `thrum.environment.export_sink`; `{"label":"Outside represented catchment","material":"water","boundary":"exported"}` |
| `10000000-0000-4000-8000-000000000020` | `thrum.fixture.static_surface_receiver`; `{"label":"Ground west","xz_rectangle_m":[0,0,32,64],"world_y_m":0,"response_patch_id":"20000000-0000-4000-8000-000000000020","response_cell_id":"30000000-0000-4000-8000-000000000020","initial_liquid_kg":0,"initial_solid_kg":0}` |
| `10000000-0000-4000-8000-000000000021` | Same type; `{"label":"Ground north-east","xz_rectangle_m":[32,32,64,64],"world_y_m":0,"response_patch_id":"20000000-0000-4000-8000-000000000021","response_cell_id":"30000000-0000-4000-8000-000000000021","initial_liquid_kg":0,"initial_solid_kg":0}` |
| `10000000-0000-4000-8000-000000000022` | Same type; `{"label":"Roof","xz_rectangle_m":[0,0,16,32],"world_y_m":3,"response_patch_id":"20000000-0000-4000-8000-000000000022","response_cell_id":"30000000-0000-4000-8000-000000000022","initial_liquid_kg":0,"initial_solid_kg":0}` |
| `10000000-0000-4000-8000-000000000023` | `thrum.fixture.closed_shallow_receiver`; `{"label":"Finite enclosed pond","xz_rectangle_m":[32,0,64,32],"flat_bed_world_y_m":-1,"initial_depth_m":1,"grid_dimensions":[4,4],"water_domain_id":"20000000-0000-4000-8000-000000000023","bed_patch_id":"20000000-0000-4000-8000-000000000024","wall_crest_world_y_m":1,"stage_valid_m":[-0.25,0.25],"edges":"closed-reflective","liquid_density_kg_m3":1000}` |

The fixture receiver schemas are proposed W2 source inputs, not new production surface/water solvers. A rectangle `[x0,z0,x1,z1]` expands to corners `(x0,y,z0),(x0,y,z1),(x1,y,z1),(x1,y,z0)` and upward triangles `(0,1,2),(0,2,3)` with one-to-one stable geometry identity equal to its record ID. A `response_patch_id`, `response_cell_id`, `water_domain_id` or `bed_patch_id` in this schema *declares* the corresponding embedded identity; it is not an unresolved record reference. Pond cell indices `(ix,iz)` are product-local subindices within that declared domain/revision and never standalone durable UUIDs. The generated bed has its separate declared patch identity. Retaining walls are the four rectangle boundary segments extruded from bed to crest and carry closed hydraulic/solid boundaries; their vertical faces have zero projected rain area. Geometry and collision cook use these same rectangle definitions, avoiding an independently authored exposure mesh. The scene closure comprises the selected source record and its transitive companion records; R0 and storm are separate entry alternatives, not two overlapping sources activated together. Any unavailable schema/provider is a real build prerequisite, not permission to silently accept these examples through today's incomplete F3 path.

All listed fields have fixed meanings in version 1. No arbitrary extra JSON key becomes a runtime control. Apply F3's existing opaque-value/unknown-field policy without reinterpreting an unrecognized field as one of these quantities. Product compatibility includes this provider schema/algorithm version. An unknown required profile/version rejects rather than guessing.

The source envelope is metres/seconds/kg/radians in C01 world coordinates. The wind's reference height and cloud plane are absolute world Y for this bounded scene, **not height above the nearest terrain**. The scene's datum record states world Y=0; a terrain-relative/provider-geospatial adapter is separate. World support is half-open on X/Z, closed over its listed height bounds; point equality at a positive X/Z edge is outside. The periodic cloud map is mathematically repeatable outside its tile, but queries still require the world receiver support. Rain support is exactly `[0,64) × [0,64)` here; outside it returns `OutsideSupport`, not an implicit zero-precipitation world. This bounded proof has no required rain receiver beyond that support.

F3/F2 admission checks finite values, unique IDs, positive dimensions/spacing/period, strictly increasing unique temporal knots and rate start ticks, complete valid time coverage, normalized nonzero directions/axis within 1e-6 length error (cook normalizes once), nonnegative lux/tau/flux, `Y(c)=1` within 1e-6, valid RGB basis, and no duplicated authority. Rates use `0<=num<=240` and `den` from `{1,2,4,8,16,32,64}`; initial offsets are finite. Phase tracks must cover every phase reached over admitted world time, or explicitly use an admitted periodic track whose end values match. The example reaches weather phase 1815 seconds at 3600 world seconds and therefore fits its 3600-second tracks. `valid_ticks` names inclusive endpoint bounds; a tick interval is admitted only when both endpoints fit. With retained tick-start P3/vegetation sampling, F2 additionally requires the derived maximum wind time derivative over the run/support to be <=0.5 m/s², bounding one-tick hold error below 0.008334 m/s. A faster authored transition needs a separately declared integrating consumer schedule or a rejected profile, not silent smoothing. Linear wind interpolation is vector interpolation, never wrapped-bearing interpolation. Require horizontal vectors here (`u_y=0`) and speed at every height/time <=40 m/s. Multiplier interpolation is linear, nonnegative and only valid on its height range.

Air is optional and constant across this support; it is a prescribed gas state, not a hydrostatic atmosphere. Define `q=m_v/(m_dry+m_v)` excluding condensed phases. Require 273.15–313.15 K, positive pressure, `0<=q<1` and derived RH in [0,1] for this bounded liquid-water saturation fit; colder/mixed-phase or supersaturated state rejects this air profile. This limitation does not prevent a separate authored snow flux; no physical consistency between these independently prescribed quantities is claimed. Missing air is `Unsupported`; P3 requiring density needs an admitted constant-density substitute. Thermal irradiance remains unavailable; surfaces use their own declared prescribed temperature/rates. Haze/fog and q are independent authored controls. Moon/pollutants are absent, never zero light or “healthy air” values.

## Products, preparation and activation

F4 reads the pinned admitted source only. It emits one immutable CPU `AuthoredEnvironmentProduct` with validated descriptors, f64 track/prefix coefficients, normalized light vectors, cloud f32 node data, rain f32 multipliers, bounds and an exact dependency manifest. F64 computations use the *decoded cooked f32 node/multiplier values* as the canonical spatial samples so CPU and GPU do not use different authored grids. Temporal coefficients remain f64. Do not recook all cells at each tick.

The optional `CloudProjectionProduct` is just that same cloud grid and a typed sampling-layout descriptor. V1 can create R32Float GPU storage only when V3/shadow consumers need it. Use four `textureLoad` taps/manual bilinear interpolation, aligning lattice node i,j with texel i,j; do not accidentally add a hardware half-texel convention. The source does not own a GPU queue. Fog coefficient/geometry and HDRI products remain ordinary V3/F4 products under RENDER-SPEC, with source identity/time linkage; headless preparation excludes HDRI convolution, shaders, cloud textures and view histories.

W2/F4's `ExposureProduct` is a shared derived product of the admitted static receiver geometry, not a new environment scene registry. Each receiver references its stable surface patch or finite-water-domain identity and the same geometry/product revision used by collision and presentation. A water receiver is the admitted pond footprint, not its current wave mesh. The product binds source/rain-grid revision, receiver topology/transform revision, precipitation direction `(0,-1,0)`, upper entry plane and water-stage ordering bounds. Its rows carry `(rain_cell_index, receiver_index_or_sink, projected_overlap_area_m2)`; surface metadata separately carries true patch area and shared surface mapping.

A concrete cook for finite static triangles is: project nonvertical receiver triangles to XZ; clip their polygons to rain cells; construct the planar arrangement of projected edges and pairwise equal-height lines within overlapping projected triangles; for each resulting open face, evaluate triangle planes at a face interior point and select the highest admitted first-hit receiver below the precipitation entry plane. No edge/order crossing remains inside that face. Sum its area into the selected row. Stable topology resolves stitched triangles of the same receiver; coincident distinct receivers require an explicit unique owner or reject. Zero-projected-area vertical faces receive no vertical rain. Uncovered area goes to a named exported sink; it is not silently discarded. Verify row areas are nonnegative and each cell's rows sum to that cell's horizontal area within the cook tolerance below. Sorting triangle heights at only the cell centre is not this operator.

For the fixed-footprint pond, replace its dynamic interior surface with its vertical receiving prism for exposure ownership, conditional on stage remaining in its admitted interval and below the walls/roof. Its bed/underlying ground is masked within that prism. A stage outside those bounds, a changed wet footprint or a changed overlying solid invalidates the exposure product. Before any tick writes, E1 checks the actual committed water topology/stage validity. **No stale exposure fallback deposits on both water and ground.** A changing R3 shoreline requires a new common-tick exposure partition/revision or a proved conservative transfer operator; that broader joint design remains open here.

Load-time admission prepares CPU source, exposure, retained surface/water/P3 products and initial state together before traversal or queries. F2 checks that every required consumer has the necessary quantity/support and substitute. A game can bind the constant-light profile without weather, and can bind shadow-only projection without visible sky. Registration uses existing composition's schema/product/stage declarations and ordinary static modules; no new plugin registry or speculative crate is required.

## Exact authored operators and phase maps

For F1 tick n, `t_n=n/60` world seconds; convert the integer tick directly to f64 rather than repeatedly adding 1/60 to an independent clock. Each phase's rate is piecewise constant over admitted rate segments, with `theta(t)=theta_0+sum r_k*length([t_0,t] intersection segment_k)`. Zero rate freezes that driver's authored controls; it does **not** pause water integration, rainfall per world second or cloud translation under a held nonzero wind. Pause of physical world time belongs to F1.

The Sun direction is Rodrigues rotation of the normalized initial direction around the normalized fixed axis by `2*pi*(theta_day/period)`, reducing phase modulo period before trig evaluation. Light colour/angular radius are unchanged. `E_perp=peak_lux*max(s_y,0)^2` is this provider's explicit authored artistic elevation envelope, before atmospheric/cloud attenuation. It is not a solar irradiance/ephemeris model. Both direction and this intensity reach every consumer under one light identity. A game wanting a constant source uses R0; accurate calendar positions or physical lunar photometry are not reconstructed from this formula.

Let `v(theta_weather)` be linear interpolation between vector knots and `a(y)` linear interpolation of vertical multipliers. `u(y,t)=a(y)*v(theta_weather(t))` m/s. This creates correlated turning wind with an authored vertical profile, not turbulence, canopy shelter or pressure-driven circulation. Consumers choose their own admitted height/filter, not a private wind law. Cloud displacement is `D(t)=integral_0^t u(y_cloud,s) ds` in world metres. Split integration at driver-rate boundaries and the world times where monotone phase reaches each wind knot; within a segment u is linear in world time, so a trapezoid is exact analytically. With zero driver rate, integrate the held vector over elapsed world time. Do not use current wind times absolute time.

For cloud receivers below the fixed plane, intersect `q=p+(h-p_y)*s/s_y` when `s_y>0.1`. Sample the periodic node field at `q_xz-D(t)` using Euclidean floor/modulo and four bilinear weights. Define `fade=0` for `s_y<=0.1`, `(s_y-0.1)/0.1` for `0.1<s_y<0.2`, and `1` above. The output is `tau_cloud=fade*tau_vertical/s_y`, and `T_cloud=exp(-tau_cloud)`; below 0.1 return exactly tau=0/T=1 under the explicitly selected authored clear fallback, tagged `AuthoredLowSunClear`. No division occurs on that branch. This is a thin-plane and low-Sun artistic approximation. Above-plane receivers are unsupported; it cannot shade points inside/above a 3D cloud. V3 applies this factor once on each named direct source-to-receiver path and integrates air/fog separately. No visible-cloud density or self-shadow claim follows.

Require cloud node tau in [0,8] and the bilinear map's maximum Euclidean gradient <=0.01 m⁻¹, checked from corner-gradient bounds; the 2×2 sample satisfies this. These are this projection's proposed error controls, not atmospheric laws. No map is updated when only time changes: upload D, phase/light and source epoch. Rain does **not** translate with this map in version 1; its footprints are static, and joint scalar controls are authored approximations. A moving weather front/mass footprint needs a separately derived swept-area integration, not endpoint sampling disguised as exact flux.

Rain cell i supplies `P_l,i(t)=w_i*L(theta_weather(t))`, similarly solid, in kg/(horizontal m²·world s). For interval `[t_n,t_n+1)`, split at phase-rate and scalar-track knots, integrate linearly varying flux by exact segment trapezoids, and multiply by each cooked row's projected area. `DeltaM_receiver=sum_rows A_row*integral P_i(t)dt`. The return is **mass kg**, not depth, true-area density or a point rate. For a surface receiver, add `DeltaM` to its owner's mass store in kg and derive the density change `DeltaM/A` using true area A; for finite pond density rho, add `DeltaV=DeltaM/rho` through its selected source operator. A prescribed infinite sea would log external exchange and could not claim finite stage rise. Solid precipitation into a pond needs the water provider's admitted phase/storage rule; the first fixture uses zero solid rain and rejects a pond receiving a nonzero solid flux without that capability.

The thermodynamic diagnostic uses `e=q*p/(epsilon+(1-epsilon)*q)`, `epsilon=287.05/461.5`, `T_C=T_K-273.15`, `e_sat=611.2*exp(17.67*T_C/(T_C+243.5))` Pa and `RH=e/e_sat`. Vapour and RH are not independently stored authorities. Zero q gives RH=0 and `NoFiniteDewPoint`; asking for a numeric dew point fails rather than substituting a temperature. No state evolves by this diagnostic.

## API proposals and concrete result policy

These signatures describe required operations; they do not commission an extensible generic query language. Use existing typed stage access and snapshot/product leases.

```rust
prepare_tick(previous: EnvironmentSnapshotLease, interval: TickInterval,
             controls: CommittedEnvironmentControls) -> Result<ForcingPlan, PrepareError>

sample_points(snapshot: EnvironmentSnapshotLease, points: &[WorldPoint],
              requested: QuantityMask, out: &mut ReusablePointResults) -> Result<(), BatchError>

sample_forcing(plan: &ForcingPlan, world_time: AdmittedSubstepTime,
               points: &[WorldPoint], requested: QuantityMask,
               out: &mut ReusablePointResults) -> Result<(), BatchError>

integrate_precipitation(plan: &ForcingPlan, exposure: ExposureProductLease)
    -> Result<DepositBatch, ExchangeError>

extract_presentation(committed_pair: CommittedEnvironmentPair,
                     time: PresentationTime) -> Result<EnvironmentPresentation, SampleError>
```

`ForcingPlan` is immutable prescribed forcing for exactly one integer tick interval and contains source/product/control revisions, phase endpoints, wind integral endpoints and the validated exposure binding. Physical queries within a tick use its admitted time range; they cannot retain a mutable solver reference. `sample_points` requests the snapshot endpoint time exactly; arbitrary stale/earlier time is not silently substituted. `sample_forcing` supplies analytic prescribed forcing to a declared water substep schedule without advancing F1. `extract_presentation` can evaluate only within the admitted adjacent committed endpoints and with their unchanged input/control semantics; it never exposes a future authoritative state.

The caller supplies reusable point-result capacity; too little capacity returns `InsufficientOutputCapacity` before writing any results. Each requested quantity returns a value with unit/support/time/role/error metadata, or its own typed failure. One unsupported optional field does not erase another valid field in the batch. Each batch reports original index correspondence, so sorting by provider/region internally cannot attach another point's wind. A required consumer validates all its requested results before staging effects. Physical queries cannot read the GPU projection. The interface returns source optical depth, not already air/cloud-attenuated sunlight disguised as a raw light descriptor.

`DepositBatch` carries interval `[n,n+1)`, source UUID/revision, exposure revision, world epoch and immutable rows with liquid/solid kg. Receiver identity is `(Geometry/Object/PatchId,ResponseCellId)` for a surface, or `(WaterDomainId,product-local cell index)` within an exact domain revision. Stable geometry identity is the companion receiver UUID; surface patch/cell IDs are its declared embedded UUIDs. Water/bed and bridge top/underside identities are distinct. A future drain/contact transfer additionally needs a named drain edge/event and producer tick/sequence. IDs are not regenerated from a human label or view index.

Delivery is sequential per complete source→consumer stream. Each consumer records its last applied interval and batch digest: the next interval stages normally, an identical duplicate returns `AlreadyApplied`, a same-ID/different-digest batch returns `ConflictingReplay`, and a gap/out-of-order future interval returns `MissingPredecessor`. Zero-input intervals still advance that cursor with a zero-mass receipt; there is no unbounded set of historical per-particle IDs. Save includes cursors/digests. Candidate cursors advance only with the same W1 commit as receiver state. A failed tick retries the same interval; it cannot generate a new ID and deposit twice. This is engine-stage idempotence, not a network transport guarantee.

| Condition | Concrete result | Required-consumer consequence |
| --- | --- | --- |
| Valid endpoint/interval and complete products | Owned typed values/immutable deposit batch | Stage effects; publish only after all owners pass. |
| Point outside a quantity's XZ/height support | `OutsideSupport(quantity,point,bounds)` | Required force/exposure consumer refuses; optional visual path uses only its pre-admitted alternate. |
| Quantity unsupported in the bound profile | `Unsupported(quantity,profile)` | F2 normally rejects before run; runtime diagnosis cannot invent calm/zero/RH. |
| Tick before source validity | `NotYetValid` | Block activation. |
| Tick after source validity, required earlier product or mismatched endpoint | `Stale(valid_until,requested)` | Halt/defer the next tick before publication; no hold-after-expiry policy exists in this profile. |
| Missing admitted CPU source/exposure/receiver product | `NotReady(dependency)` | Loading or failure; no partial rain delivery and no clear-weather substitution. |
| Wrong world/source/model/geometry/exposure revision | `EpochOrDependencyMismatch` | Discard candidate/late async result; retain previous committed state. |
| Water stage/topology no longer satisfies exposure ordering | `ExposureInvalidated` | Reject before deposit publication; dynamic shoreline needs a new exposure operator/product. |
| Candidate water/surface solve exceeds admitted support/positivity/finite limits | `CandidateOutsideModel` | Discard the complete tick's candidates and receipts, enter visible stopped/fault state. |
| NaN, invalid direction/units/negative mass from source | `InvalidSource` | No publication; diagnose source identity/field. |
| Same complete deposit batch repeated | `AlreadyApplied` | Return its stored receipt, change no mass. |
| Lower display tier or second camera | Valid unchanged physical source | Only presentation resources/queries differ. |

Optional unsupported quality substitutions are explicit profile bindings, not a universal fallback callback. In this concrete storm source only low-Sun clear attenuation is selected as a tagged authored substitute. Gas-density substitution is not implicitly enabled by merely having a P3 consumer.

## F1 schedule and consumer handoffs

1. **Pre-tick barrier:** W2 has preloaded the full bounded closure. Admit ordered runtime controls and validate source/exposure/receiver revisions against X[n], including pond stage/order. The only live source control admitted here is `precipitation_enabled:bool`, committed by an ordinary authoritative command with input identity. Editing source tracks/phase maps/geometry remains restart work. Compute one `ForcingPlan(n)`; no view invokes it.
2. **Read X[n]:** P3 reads committed surface traction and physical water state; it requests wind at each declared body point/height and applies its own force law once. Vegetation reads the same source and advances its own response, with prior/current deformation. A prescribed source is not changed by P3, vegetation or water feedback.
3. **Stage interval forcing:** E1 computes one precipitation input register and immutable receipts. Surface owners add incoming kg to their single liquid/solid stores, then run their selected transfer/material response. New contact/splash deposition from this tick belongs to the next designated update, once by ID. No source-level “wetness” scalar accumulates a second reservoir.
4. **Finite pond:** bind the selected CPU first-order hydrostatic/HLL shallow-water family in PLAY-ENVIRONMENT-SPEC, using the fixture's 4×4 closed reflective grid. Before each admitted HLL substep, integrate that subinterval's liquid rain exactly into `h` as `DeltaM/(rho*A_cell)`, with zero added horizontal momentum `q`. Re-evaluate CFL limits using the source-updated depth before that flux step. This split is a **proposed rain-source operator to adopt with the water owner**, not a claim already implemented. Rain impact impulses, vertical momentum and energy are outside this profile; water mass addition is represented. P3 bodies still read X[n]; new body-water reaction records follow the chosen baseline's next-step/one-way rules, not reciprocal rain/body pressure invention.
5. **Validate and commit:** validate every candidate, nonnegative reservoirs, source/transfer ledgers, stage/order and consumer cursors. W1/F1 publishes the source endpoint, surface/water/vegetation candidates and confirmed events as one X[n+1]. Failure leaves X[n] and prior cursors intact. Never publish water then discover that the surface delivery failed. Provider candidate buffers are not read by P3 partway through the step.
6. **Presentation/audio:** V2/V3/V5 and A1 read committed endpoints. V3 owns surface/volume transport, one source-path cloud factor and any separate admitted air/fog; water is a V3 surface/material consumer of the same light. Vegetation shader deformation uses its retained response, not shader-private wind phase. V5's culled streaks do not determine source mass. A1 continuous wind/rain parameters use the explicitly selected listener and simulation-time source; confirmed impact/drip events have once-only identity. A second camera does not create another source tick or move the listener. Audio callbacks receive precomputed values/events and do not lock world/source arrays.

Physical P3/vegetation wind is a point sample at t_n in the first schedule, with its temporal hold error accounted below; a later integrating consumer can use the plan's exact subinterval wind. Water stress/wave growth consumes the same U10 through its own declared response; this source does not regenerate a sea spectrum instantaneously from wind. Thermal surfaces select the terrain owner's prescribed temperature/rate model. If they require W/m² energy balance instead, this provider cannot satisfy them. A gameplay visibility/pollutant consumer cannot read optional fog/sky pixels as an authority.

Headless authority executes exactly steps 1–5, CPU queries, physical responses and retained game/audio-event production as selected. It has no views, shaders, GPU map, HDRI preparation or image histories. The optional rendered proof creates two shadow consumers after the same numeric source/response path; it is a distinct presentation proof, not “headless with cameras”. No full R1/R3 air/sky result is claimed.

## Worked fixture and lifecycle traces

**Normal first tick.** At t=0, day/weather phase are zero, Sun points up with E_perp=100000 lux, U10=(6,0,0), cloud-height wind=(12,0,0), and rain is 1/360 kg/(m²·s) after the cooked constant representation. The 512 m source and 64 m rain domain activate once. P3/vegetation sample t=0; source rain integrates `[0,1/60)`. Candidates publish at tick 1 with correctly advanced phases/displacement. Every consumer output references that source/tick, not its host callback time.

**Roof, ground and pond.** Cooked first-hit projected areas are roof 512 m², ground 2560 m² and pond 1024 m²; sum=4096 m². The pond's separate bed receives no direct rain. Under ideal rational P=1/360 and one 1/60 s tick, mass is respectively 512/21600, 2560/21600 and 1024/21600 kg, total 4096/21600 kg. The actual oracle also evaluates the exact decoded f64 temporal knot value, so decimal encoding error is distinct from ledger error. At 60 seconds, the ideal values are 85.3333333, 426.6666667 and 170.6666667 kg. Pond input volume is 0.1706666667 m³, implying mean stage increase 0.0001666667 m before other named exchanges; its HLL stage need not be spatially uniform at every instant. Surface kg/true-area is a derived material input. No multiplication by sloped area occurs in this mass projection.

Use initially zero surface mass and disable drainage/phase exchange for this first conservation oracle under an explicitly selected terrain test-material profile. Do not mistake that profile for the full surface-response model. A separate source-off/transfer proof enables the terrain owner's prescribed film/absorption/evaporation/drain model and checks its complete named exchanges. The E1 source does not invent its constitutive rates or traction curve.

**Wind and independent day/weather phase rates.** At world 30 s both phases equal 30 s; day rate then becomes 120, weather rate 1/2. At world 60 s, day phase=3630 s and weather phase=45 s, U10=(1.5,0,−4.5). Integrated U10 displacement over 0–30 s is (135,−45) m in XZ, and over 30–60 s is (67.5,−112.5) m: cloud-height displacement at 60 s is therefore **(405,−315) m**. Replacing it with current cloud wind times 60 s produces (180,−540) m and must fail. All physical transfers still integrate 60 world seconds, never 3630 day seconds or 45 weather seconds. A zero weather rate holds wind/rain controls but retained wind still advects cloud texture and rain still deposits per world second. Source phase-rate tracks are artistic control, not accelerated thermal physics.

**Two views.** At committed tick 3600 the main view extracts at an admitted interpolation time; the half-rate secondary uses its own previous image sample time and either reevaluates this analytic source with matching parameters/control history or resets. Source update count remains one per tick; no source arrays copy per view. Moving either origin changes only evaluation representation. A low-Sun sample receives the explicit authored fade/clear tag in both CPU and GPU paths. Missing past control/parameter generation invalidates correspondence; previous OS frame is not a substitute. Cloud-shadow-only rendering allocates no visible-cloud ray marcher, density volume or cloud-view history.

**Source-off and tails.** An authorized precipitation-disable command takes effect at the pre-tick barrier. The whole interval beginning at that tick has zero input mass, with a zero receipt advancing each consumer cursor. Already committed wetness, absorbed water, snow and pond volume remain; surface transfers and water waves continue under their owners. This source has no physical in-flight rain inventory, so it cannot promise delayed atmospheric fallout; V5 visual streaks may fade under their own nonauthoritative lifetime. Stopping only visible streaks never stops physical rain. Removing cloud presentation does not remove its retained shadow query; removing the last retained quantity consumer can retire its optional products after leases/fences release.

**Save/restart/device loss.** P7 captures tick, exact source/build/product closure, committed precipitation gate/control revision, surface/water state, exchange cursors and required event cursors. Source phases and D(t) are derived from tick plus immutable maps; saving optional acceleration caches must not create rival phase authority. Restore recomputes these values or validates caches against that derivation, prepares matching products, then publishes. Editing wind/day track B does not mutate run/save A; restart B gets its own phase origin/control epoch, and an A save cannot silently restore against B. Seek is a disposable replay/restart operation with restored physical state, not changing source time under old wetness. Device loss recreates only optional GPU projections; source physics/state continues under the platform's pause policy. Mobile suspend uses F1 pause/bounded catch-up, never an unbounded wall-clock rainfall jump.

**Missing/stale or changing shoreline.** No optical-air provider is needed in the first source, so its absence does not fault physical rain. Missing required exposure, source or water CPU data blocks activation. A late async product for another world/source revision is discarded. At valid_ticks end, the next interval is refused; an explicit new profile/run is needed to extend validity. A water stage/wet-area/terrain change invalidating first-hit ordering refuses the complete tick candidate before publish. Reusing the old static partition, blending bed and water weights, or treating a dynamic tide as this fixed pond is rejected. The terrain dogfood's changing-shore test is separately dry/no-rain; neither document claims the wet-front exposure mechanism is complete.

## Error budgets and independent oracles

These are proposed rejection thresholds for this fixture, not measured accuracy. A test must identify source/product/operator version and exact values actually decoded. Source error, numerical evaluation, geometric partition, consumer approximation and CPU/GPU conversion are reported separately.

| Mechanism | Bound/meaning | Oracle and falsifying case |
| --- | --- | --- |
| Phase/time | One F1 tick; evaluate from integer tick/rate map. At the fixture's 30/60 s points, phase must match the exact rational construction within 1e-9 s. | Independent integer/rational arithmetic for phase sums; rate=0, rate knot exactly on tick, final valid endpoint and wrap. A second view may not increase phase. |
| Sun | Unit direction error <=1e-12 in CPU f64 for cardinal rotation fixtures; nonnegative authored intensity. GPU/source f32 projection compared separately, angular error <=1e-5 rad. | Cardinal `R_z` cases, independent rotation matrix versus Rodrigues evaluator, t and t+period, zero/below-horizon intensity. No external ephemeris oracle is claimed for an authored orbit. |
| Wind and D(t) | CPU displacement error <=1e-8 m over the fixture's 3600 s; point velocity <=1e-10 m/s versus the decoded piecewise affine oracle. Tick-start hold error <=L_t/60; derivative cap is consumer admission. | Independently integrate rational fixture segments and verify (405,−315) m at 60 s; current-wind-times-time must fail. Compare vertical interpolation at 0/10/1000 m and out-of-range failure. |
| Cloud interpolation | CPU bilinear values agree with independently evaluated corner weights within 1e-12 tau; nonnegative output and continuous low-Sun fade. CPU/GPU absolute transmittance error <=1e-3 for the bounded support. | Constant map, checker/affine tile interiors, exact periodic seam, negative coordinates, axis swizzle/half-texel trap, low-Sun thresholds, rebase. GPU uses canonical cooked nodes. This is sampled projection agreement, not physical cloud-shadow accuracy. |
| Thin-plane/model error | Unsupported cloud-side/above-plane/depth structure, no volumetric self-shadow claim. Low-Sun fade is an explicit authored approximation. | Ask an above-plane query and require Unsupported/OutsideSupport; do not “pass” by visually rendering a slab. A volume provider is required to extend the behavior. |
| Exposure geometry | Fixture row sums within 1e-8 m² absolute/cell, no negative area; each represented footprint assigned once. General cook must report projected/true area separately. | Integer-coordinate rectangle areas above; independent high-precision polygon area/vertical-ray enumeration for triangular stress fixtures. Roof-edge, coincident receiver, narrow sliver and pond/bed identity cases. Cell-centre-only exposure is a known wrong substitute. |
| Interval deposit | At each complete interval, residual between total external input and receiver+sink kg <=`1e-10*max(1,input_kg)`; no applied mass <−1e-12 kg. Threshold does not authorize clipping negative source flux. | Exact constant/ramp integral versus independent rational/algebraic antiderivative; split a tick into subintervals and require equal total; source-off, repeated batch, conflict and missing predecessor. Check per receiver, not only global total. |
| Surface/water transfer | Liquid/solid phase transfer cancels in total; prescribed sinks/receivers logged once. Pond ΔV matches rain kg/rho plus its selected HLL boundary/other sources within the water owner's solver tolerance. | Closed reflecting pond, no bodies/drains first; whole-domain volume sum independent of source register. Then selected surface-tail and P3/water fixtures with all named exchanges. Repository checks do not establish HLL/source positivity. |
| Air diagnostic | q/RH use one declared gas denominator and bounded saturation law; no implicit pressure level/geopotential conversion. | Independently compute e/p/RH round-trip, q=0 dew-point absence, inconsistent redundant RH rejection and temperature outside [273.15,313.15] K rejection. No medical/exposure claim. |
| Snapshot/lifecycle | No mutable alias, stale product or partial receiver/cursor publication. Discrete controls/events do not blend. | Inject missing product, wrong epoch, candidate water-stage refusal, save interruption, restored cursor, GPU loss and half-rate secondary view. Compare before/after source plus all receiver hashes, not source alone. |

The cloud GPU tolerance must be qualified for the actual shader arithmetic and coordinate conversion. Analytical support: periodic coordinates are reduced near the finite tile, gradient is bounded, `s_y>0.1` avoids singular division, and `|Delta exp(-tau)|<=|Delta tau|` for nonnegative tau. The larger Sun-intersection displacement at low elevation still needs direct CPU/GPU comparison. If the tolerance fails, improve precision/rebasing or tighten the admitted projection; do not weaken physical queries or claim a GPU texture is their oracle. A physically correct shadow is a separate appearance/model qualification even when both evaluators agree.

Rain gate controls have discrete commit semantics. A pre-tick command selects interval-n forcing; its endpoint/control event becomes externally visible with X[n+1]. Snapshot-n point rain remains a description of its earlier committed endpoint, so physical interval consumers must use `ForcingPlan`, not sample-and-multiply that snapshot point rate. Presentation holds the prior discrete gate until the corresponding commit and resets affected streak correspondence as needed; it does not interpolate booleans. This declares the up-to-one-tick publication delay rather than inventing a partially updated snapshot.

## Work, residency and preparation cost

The source has no entitlement to a separate frame budget. Account under [BASELINE-SPEC](../BASELINE-SPEC.md#budget-allocation-and-failure-rules), including two due ticks in a 30 Hz callback. No timings or device capacities are claimed.

Per tick, phase/track cursors and prefix lookup are O(active tracks + crossed knots); a pathological tick can cross at most the admitted 256 total knots, and it must not create unbounded subjobs. Cloud D integrates the crossed wind segments, independent of views. Query work is O(Q log H) for H<=8 vertical wind layers; Sun is evaluated once per requested time/batch. Cloud optical queries use four node loads plus projection/exp per point, not one cloud simulation per receiver. Rain computes two scalar integrals, multiplies N_p static cell weights and accumulates E<=32768 sparse exposure rows. At 60 Hz the row cap is 1,966,080 rows/s, before any surface/HLL/body work. Q, metadata lookup, allocations and lease operations deserve counts: batch per provider/time, use caller-owned output/scratch and reuse exact invariant light/track coefficients. No per-leaf source allocation or per-query schema lookup belongs on the hot path.

The static-exposure cook has potentially quadratic overlap construction. Bound it explicitly for this fixture: at most 2048 source triangles, 65536 intersecting triangle pairs per cell-batched cook, 131072 intermediate arrangement vertices and 32 MiB private cook scratch. A product exceeding these proposed limits refuses/defer-to-different-cook before activation; it does not omit hard polygons. Use spatial broadphase/clip-by-rain-cell before pair tests, and reserve scratch before construction. These are selected worker-profile limits, not universal F4/source byte ceilings. Track cook time and memory separately from runtime.

A conservative logical-byte account for the proposed maximum products is cloud 64²×4=16384 B; rain 128²×4=65536 B; 256 knot entries×32=8192 B; 32 phase-rate entries×32=1024 B; 32768 exposure rows×16=524288 B; 1024 receiver descriptors×96=98304 B; and (16384+1) u32 row offsets=65540 B. Total **779268 B** before container headers/allocator alignment/source manifest. These wire/storage widths are proposed layouts to verify, not Rust `size_of` claims. Old/new source product overlap is at least twice this amount when a qualified transition actually retains both; restart can release old products first. Surface/HLL/body state, P7 captures and consumer histories are additional owners, not hidden in this number.

Budget up to 1 KiB for an immutable source endpoint and retain two committed plus one staged endpoint: 3 KiB in this proposed layout. A 1024-target deposit batch at 32 B/target is 32768 B, with separate per-consumer cursors/digests bounded by admitted target streams; a conservative two-batch overlap is 64 KiB. Do not copy all source grids into each endpoint. P7 can lease immutable products and capture gates/cursors with the world; save state and render endpoint are distinct products. Any retained old endpoint/product is charged until its last CPU reader releases it.

Shadow-only GPU products add one shared 64² R32Float cloud map=16 KiB plus small source constants; no map upload is required merely because D/time changes. Additional per-view targets/histories belong to the renderer dogfood and its overlap budget. Two views share the source/map and add only their own presentation work. Headless has none of these GPU products. R0 retains a constant light/fill descriptor and no phase/rain/exposure grids or weather job. Source-off does not release the retained surface/pond state it created; only optional source work with no retained consumer can retire. Future rich density volumes, spectral LUTs, forecast datasets and ML models require their own content/runtime costs.

Measure source work and critical path as `F1/control validation -> source plan -> exposure/deposit -> surface/water candidates -> shared commit -> extraction/upload -> V3 transport`. P3 queries read X[n] and do not wait for GPU. No asynchronous weather job can publish into a later epoch without revalidation. Timing should distinguish CPU evaluation/row reduction, HLL/response work, extraction and GPU passes, with source queries/bytes/allocations and endpoint/lease peak alongside normal, source-off, save/restart and two-view cases. Once basic checks pass, no extra unrelated benchmark campaign is implied by this document.

## Decisions exposed by dogfooding and integration requests

| Implementor would otherwise invent | Resolution for this recipe; authority consequence |
| --- | --- |
| Human names or UUIDs? How does a light reference resolve? | FOUNDATION-SPEC UUID record envelopes; complete companions above, embedded identities explicitly declared. Human labels never bind. One incoming environment descriptor per light identity. |
| Is R0 ambient luminance or irradiance? | Reconciled with render owner: `ambient_irradiance_lux` RGB [1000,1000,1000], isotropic incident diffuse fill, Lambert 1/pi; independent black background. No sky/specular product inferred. |
| Does a render callback update source time, and which seconds does rain use? | One F1 clock; versioned phase maps; fluxes are per world second. Weather-rate zero holds controls, not physical transport. |
| How does changing wind move the existing cloud field? | Integrate the actual layered world-time wind; exact piecewise-affine primitive, not current velocity times total age. |
| What does a cloud map value mean, and can it shade above itself? | Periodic nodal vertical tau with explicit planar source-ray projection, low-Sun authored fade and below-plane support. No 3D appearance guarantee. |
| Can rain intensity at the frame endpoint become total deposit? | Separate point and exact interval-mass APIs; static cell-mean footprints and cooked first-hit areas; consumer kg storage/true-area conversion. |
| Does pond rain use a new local bucket? | Existing CPU HLL/hydrostatic model with an explicit zero-horizontal-momentum source split and volume ledger; no static-pond finite-volume claim. |
| What if water moves over ground or an animated roof? | Static exposure invalidates; halt/refuse before complete tick publication. Fixed-footprint enclosed pond first, dynamic R3 wet-front exposure remains a separate joint operator. |
| Can missing pressure/cloud/species become plausible defaults? | Typed absence and F2-required substitution; only declared low-Sun clear/cloud case is selected here. |
| How are retry, source-off and save ordered? | Complete interval identity/digest/cursor, zero receipts, same W1 commit; source-off removes new input while response tails persist. Save pins exact source/products/state and gate/cursors. |
| Must physical heat or air quality come from this sky? | No physical irradiance/pollutant authority in this provider. Use selected prescribed thermal/material model or reject a stronger requirement. |

Proposed canonical follow-ups, for owners to reconcile rather than edits made here: (a) F3/F2 adopt the exact UUID-bearing source/identity/profile schema and required-quantity validation; (b) C09 identify the exposure revision/receiver mapping and complete interval batch/cursor semantics, distinguishing static bounded support from dynamic wet-front replacement; (c) E1/water specify/qualify the HLL rain-source split and CFL/commit refusal behavior; (d) C08/RENDER-SPEC align the direct ambient-irradiance R0 field and keep raw Sun/cloud attenuation ownership explicit; (e) C02/C10 carry discrete source-control publication and phase/cache/save semantics. The existing broad contracts already allow these behaviors; naming them in this recipe is not proof that source schemas or adapters exist.

**Remaining missing designs, not unrun tests:** dynamic precipitation exposure when shoreline/wet area or solids change; moving/oblique/porous rainfall interception; physical meteorology/thermodynamic energy closure; physically evolving 3D clouds; astronomical/Moon photometry; pollutant transport/exposure; solid precipitation into this selected pond; and physical unloaded-region catch-up. A game requiring any of them is outside this recipe. **Unrun qualification:** implementing the bounded schema/cook/evaluator and owners, numerical source/geometry/flux checks, response/HLL/source integration, presentation correctness/motion, lifecycle/headless/two-view/save behavior and named-device cost. These cannot be checked off by this paper walkthrough or `check-docs`.
