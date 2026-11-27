# Scene effects

An engine module implements `thrum_render::contribution::SceneContribution`. The frame host owns the device
generation, encoder, submission and frame clock. Attach contributions with `FrameHost::set_scene_contribution`;
use `SceneContributions` for an ordered list, including cooked assets. The external diagnostic in
`examples/v1-game/src/effect.rs` is an executable example. The owning [design](plans/EFFECT-GRAPH/DESIGN.md)
defines clock and history semantics; [C06 and C07](design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1)
define lifetime and temporal conventions.

`rebuild` prepares every pipeline and binding on `SceneSurface::device`, before readiness. A new device
generation starts a new effect epoch. Same-generation rebuilds retain state unless an owned descriptor changes.
`effect_graph` declares real resources, named simulate passes and `opaque_reads` for the fixed opaque draw.
Late-only outputs do not become forward inputs. Resource indices and retained owner IDs are
local to the contribution; the ordered list remaps them. Prepared graph accesses describe physical usage.
The renderer validates dimensions, ranges, storage capabilities and cadence before accepting the attachment.

`encode_upload` receives the frame's source epoch, timeline, tick, tick duration, interpolation alpha and
display phase. The phase comes directly from the world's clock at its consumed target (observation plus lead).
Observation and consumed target are host times; display time is world time. In a running span, tick `C`
falls due at host time `t_C`, and the phase is `(observation + lead - t_C) / tick_duration`.
The frame sample and submitted capture record carry `tick_due_time` while interpolating; effects receive
no host timestamp. World display time is `(C - 1 + phase) * tick_duration`, equivalently
`(C - 1) * tick_duration + observation + lead - t_C` below the bound. It is not a host timestamp.
The origin `t_C - C * tick_duration` is fixed within a span. Starts and resumes begin spans;
pause and overload end them and discard the elapsed part-tick. Resume is refused until queued work
commits, then holds at the committed world tick until its first new tick commits. Queued ticks, late
commits and holds do not move due times. A rewind changes the world's tick, retaining its host due time
and beginning a span with a different origin. Display time excludes stopped time and discarded fractions;
across spans its difference is the consumed-target difference less the origin difference.
While interpolating, presentation time in ticks is `tick - 1 + display_phase`; the phase extends past one up
to `1 + 50 ms / tick_duration`. Alpha remains the phase held to one. Endpoint-only states and captures give
one. Sampling never admits a tick. Predict presentation-only quantities from committed state a tick at a time
by their own step; quantities known only at endpoints keep alpha. See
[C02](design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world).
At a stop or queue, display time moves to the committed tick, by at most a tick forward or 50 ms back
on that tick; subsequent queued commits each add a tick. Draining a queue returns to the same span's
target. The first interpolating frame after resume advances from the resume's committed tick, never
back. At the phase bound display time holds at the committed tick plus 50 ms. The first span has no hold.
The resident world lane shows the newest eligible clock/page pair among the published pair, the pair shown
and the refused pair retained from an earlier poll. An interpolating pair qualifies when its tick was due
by the observation; a stopped clock always qualifies. A published pair newer than the one shown becomes
the refused pair. This keeps `t_C <= observation` through repeated render-owner preemption without going
backwards or stalling on one tick. A delayed frame can show an earlier tick than the world's newest, with
a lead or without. It costs at most one extra page lease, adds no clock read, never stops authority, and
changes neither alpha's arithmetic nor the consumed target.
Use `EffectState::begin` against the committed state: equal ticks keep contents, forward ticks advance up to
the declared bound, backwards ticks or another timeline reset. Captures run no state producer. First-epoch
and reset recordings write every texel, including time-sliced resources. `effect_runs` skips work without
recompiling. `encode_effect` records one declared pass under its own timing name. Mark histories produced after
the whole output is written. Resolve previous/next/result with `EffectState::side`; result selects the tentative
output when its producer ran and the committed side otherwise. Commit clocks, anchors, history sides and prior
deformation only in `submitted(true)`. A discarded encoder commits nothing.

A simulate pass with an `AttachmentClear` access draws before the Sun passes and opaque shading.
Declare one mip and layer of an effect-owned, single-sample 2D colour texture and no other writer for that
subresource. In `rebuild`, choose the colour format using `SceneSurface::texture_format` and create an
`EffectDrawTarget` with the clear colour and the pipeline's blend choice. Return it from `effect_draw_target`
at the pass's local index. The executor opens and times the pass. In `draw_effect`, record draws into that open
pass with the effect's prepared pipeline and bindings. A skippable draw requires persistent storage and runs
on the first frame of each effect epoch. A skipped pass leaves its last submitted contents available to readers.

Declare late render or compute passes in `EffectGraph::after_opaque` with `AfterOpaquePass`. They run after opaque,
sky and the optional stars, in contribution order, before temporal consumers and the pre-tone input.
Each pass declares effect-local `accesses` with the same ranges and history bindings as simulate passes.
A render pass loads scene colour, motion and reactive attachments. Local attachment or copy accesses are refused
at preparation, since these callbacks use the scene's attachments. A compute pass records through
`encode_after_opaque`, with `after_opaque_runs` selecting runtime skips; it writes declared private outputs.
The same cadence and full-frame history rules apply. Set `depth` to sample this frame's
reverse-Z depth at group 1 binding 0; depth storage follows the union of declared readers. Independently set
`refraction` to sample the shared RGBA16Float scene-colour copy at group 1 binding 1. It contains opaque, sky
and stars before any late effect. All refracting effects share one copy. A depth-only effect creates no copy;
an absent effect creates neither a late pass nor additional storage. The copy and late load/store passes end
the original render pass, so consumers should judge their measured cost when selecting recipes.

Create the late pipeline with the scene's view layout at group 0 and `AfterOpaquePass::input_layout` at
group 1, then any owned bindings. The renderer prepares and binds both shared groups. `draw_after_opaque`
sets the pipeline and owned groups and records draws. Compute passes receive the same view and target inputs,
including the view's inverse projection and extent; their outputs can feed a later history resolve and render
composition. `examples/v1-game/src/effect/after_compute.rs` demonstrates trace → resolve → render → compute,
including skips, discard and a final compute pass after the last colour writer.
Compare sampled reverse-Z depth to the fragment's
depth; a larger sampled value denotes a nearer opaque surface. A volume need not write depth.
Output pre-exposed scene-linear colour in target 0, motion in target 1, reactive coverage in target 2.
Motion is **previous pixel minus current pixel**, top-left origin, X right/Y down, in render pixels, from
unjittered positions and the last submitted deformation. Invalid correspondence is `(65504, 65504)` and
reactive coverage 1. Honour `ContributionFrame::history_reset`, device replacement and owned effect resets.
This hands inputs to reconstruction; the seam does not reconstruct the effect.

Atmosphere lighting is available through `SceneSurface::environment_products()` during rebuild. Its standalone
WGSL library declares group 0; when composing it with the scene view, remap that library's group attribute to a
free group (for example 3), put its unchanged layout at that index and bind its `group(frame.exposure_side)` there.
Its lighting values are un-pre-exposed: apply `environment_storage_scale()` once to the finished contribution.

`EffectPass::async_eligible` admits compute-only declarations without attachment/copy access and lists them in
`CompiledGraph::async_eligible`. It does not change order or use another queue. `FrameHost::quality_knobs`
lists named integer values, inclusive ranges, declared effects and invalidations. `set_quality_knob` stages a change;
`commit_quality_knobs` applies it before frame preparation. `QualitySettings` coalesces requests and reports
the strongest invalidation. The effect invalidates its own state there; `ViewHistory` also resets the host's
reconstruction history until a frame submits. Ordered-list control names have a `member/` prefix.
The example exposes cache interval, volume slices and bounded catch-up ticks.
`QualityError` distinguishes invalid declarations, unknown controls, out-of-range values and an absent
contribution. A refused request preserves both the committed value and any earlier valid staged request.

## Tick stage

Declare a `TickPass` in `EffectGraph::tick_passes` for compute that answers the game independently of
a view. Its declared reads can use persistent resources and the `Previous` side of histories. Allocate
its inputs, outputs and readbacks in `rebuild`, outside the frame graph. Every piece of frame state read
by the query must have been written by a submitted frame pass. Queue writes staged during a discarded
frame can still reach the device.

A post carries the world's `SourceTick` and the tick its answer is for. On the sky kit, use
`SkyGame::with_fixed_update_endpoint` to post with the endpoint of each committed tick, including tick
zero. Use `EffectState::committed_for` to compare a post with the last submitted frame's world and
timeline. If it returns no state, publish a marked answer without a field value or let the post expire.

The host asks after advancing the world and before recording a view. It owns the tick encoder and
submission. The contribution reports available posts in `tick_has_work`, stages query inputs in
`encode_tick_upload`, records compute in `encode_tick`, and copies its outputs in `encode_tick_readback`.
Leave posts queued through recording. Only `tick_submitted` removes submitted posts and starts
asynchronous maps. Tag the mailbox with its device generation and clear old results on replacement.
Drain answers on the game side without touching the device. See the
[tick stage design](plans/EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6).

## Secondary views

Declare one `SecondaryViewSlot::new(scale)` for each view you can use at once in
`SceneContribution::secondary_view_slots`. The scale sets the largest viewport as a fraction of each side
of the main render extent. It accepts 0.25 through 1.0. The renderer prepares the slot's resources up front,
with up to four slots across the scene and further limits from the device. Clone your own handles from
`SceneSurface::secondary_views` in `rebuild`. Replace those handles after a resize, republication or device
replacement.

Ask in `encode_upload` by calling `slot.request(&request)`. A slot starts idle on every recording. Build
the request with `SecondaryViewRequest::new`, a rigid right-handed `SyntheticView::perspective` and an
extent no larger than `slot.maximum_extent()`. Copy X and Z from `ContributionFrame::camera` exactly.
Height, direction and field of view can vary. For a horizontal mirror, reflect eye and target across the
surface and build a proper view with world up. Use a world-space clip plane whose positive side faces the
geometry to keep. `ViewAir::Plane` requires a level clip plane and starts the aerial transport at each ray's
crossing of the plane. A plane within 5 cm of the requested eye is moved out to 5 cm. A rejected request
has a named error. Use your fallback for an idle slot and its old contents.

The slot draws opaque entities and opted-in contributions, including cooked placements. It makes its own
visibility lists, local-light clusters, aerial volume and Sun cascades. It shares scene data, the terrain
height map, Sun and sky lighting products, exposure and jitter phase. It contains no sky, stars, fog effect,
water, other after-opaque effects, motion or history. The view has aerial perspective according to its
request. This is distinct from a fog effect composed by a reader.

Bind `slot.colour()`, `slot.depth()` and `slot.record()` in your own after-opaque pass and read them only
in the frame that successfully asked for the slot. The record's layout is `SecondaryView::record_wgsl()`.
Project the surface point with the record's jittered `view_proj`, divide by W, and convert X/Y to
top-left UV with `ndc * vec2(0.5, -0.5) + 0.5`. Test the projection and viewport before sampling. Multiply
the UV by `extent.xy` for viewport texels. For linear sampling, clamp to the viewport's texel centres and
divide by `extent.zw`, the prepared attachment size. RGBA16Float stores pre-exposed scene radiance with
coverage in alpha. Cleared pixels have zero colour and coverage. Depth32Float stores reverse-Z depth,
cleared to zero. To reconstruct a covered point, turn its viewport texel centre into NDC, use the
sampled depth as Z, multiply by `inverse_view_proj` and divide by W.

For a mirror, fill the uncovered fraction with your sky reflection in the same encoding. The covered
RGB already holds the reflected stretch through air from the plane. Apply the main camera's aerial
pair once from the pool to the camera. With storage scale E, reflected stored colour V, and main air
scatter S and transmission T, the result is `E * S + T * V` after filling uncovered sky. The reader owns
surface shading, filtering, extra fog and the fallback, including any choice of mip chain. The
[mirror pool](../examples/mirror/src/pool.rs) and [lookup shader](../examples/mirror/src/pool.wgsl)
show the bindings and the two-stretch composition.

Register your view's settings through `QualitySettings` and feed the committed values into each request.
The mirror declares the following ranges. It preserves history and uses prepared resources at both ends.

| Control | Mirror range | Work affected |
|---|---:|---|
| Extent | 25–100% per side | Viewport only. Both raster dimensions fall, reducing fragments while the attachments keep their full size. |
| Far distance | 64–32,000 m | Projection and frustum. Reduces distant geometry in the draw. |
| Least size | 0–16 pixels | Projected size threshold for entity compaction and indirect cooked meshlet culling. The direct route does not size-cull. |
| Sun cascade count | 0–2 | Number of per-view Sun depth draws. At zero the terrain height map still answers shadows. |
| Sun reach | 16–512 m | Coverage of the enabled cascades. A shorter reach fits a smaller volume, changing the casters and keeping the terrain map beyond it. |

An idle slot skips its view passes. An active slot adds per-view preparation work on the GPU and a
second opaque draw. For example, at 2560×1440 the full-size RGBA16Float and Depth32Float attachments
reserve 44,236,800 bytes together, exclusive of lighting and culling products and backend overhead.
Changing the slot count or the largest view extent requires preparation. [Scene view design](plans/SCENE-VIEWS/DESIGN.md#cost-account)
describes cost scope and pass ownership.

The endpoint table gives measured GPU time for each control on the authored mirror and the
100,513-instance field. The paired runs used an Apple M3 Max at 2560×1440, with capture off and
Portable and AppleTile selected separately. The figures were taken under load while another
builder's GPU workload ran. They apply to those conditions, with no quiet-machine figures claimed.
The resource table gives the logical bytes reserved before any view is requested.
[Measured endpoints](../.orchestrate/evidence/SCENE-VIEWS/M3/README.md#measured-control-endpoints) ·
[Prepared resource sizes](../.orchestrate/evidence/SCENE-VIEWS/M3/resource-bytes.md)

When timestamps are available, `SceneGpuTimings::execution(name)` distinguishes `Timed(0)` from `NotRun`
and `Unmeasured`. The last denotes transfers without counters. Disabled timing remains absent.
List order establishes execution only: SEA and CLOUDS still own disjoint medium admission and the explicit
joint or approximate integration policies required by [C08](design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4).
