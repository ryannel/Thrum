# Scene effects

An engine module implements `thrum_render::contribution::SceneContribution`. The frame host owns the device
generation, encoder, submission and frame clock. Attach contributions with `FrameHost::set_scene_contribution`;
use `SceneContributions` for an ordered list, including cooked assets. The external diagnostic in
`examples/v1-game/src/effect.rs` is an executable example. The owning [design](plans/EFFECT-GRAPH/DESIGN.md)
defines clock and history semantics; [C06 and C07](design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1)
define lifetime and temporal conventions.

`rebuild` prepares every pipeline and binding on `SceneSurface::device`, before readiness. A new device
generation starts a new effect epoch. Same-generation rebuilds retain state unless an owned descriptor changes.
`effect_graph` declares real resources and named compute passes. Resource indices and retained owner IDs are
local to the contribution; the ordered list remaps them. Prepared graph accesses describe physical usage.
The renderer validates dimensions, ranges, storage capabilities and cadence before accepting the attachment.

`encode_upload` receives the frame's source epoch, timeline, tick, tick duration and interpolation alpha.
Use `EffectState::begin` against the committed state: equal ticks keep contents, forward ticks advance up to
the declared bound, backwards ticks or another timeline reset. Captures run no state producer. First-epoch
and reset recordings write every texel, including time-sliced resources. `effect_runs` skips work without
recompiling. `encode_effect` records one declared pass under its own timing name. Mark histories produced after
the whole output is written. Resolve previous/next/result with `EffectState::side`; result selects the tentative
output when its producer ran and the committed side otherwise. Commit clocks, anchors, history sides and prior
deformation only in `submitted(true)`. A discarded encoder commits nothing.

Declare late render passes in `EffectGraph::after_opaque` with `AfterOpaquePass`. They run after opaque,
sky and the optional stars, in contribution order, before temporal consumers and the pre-tone input.
The late pass loads scene colour, motion and reactive attachments. Set `depth` to sample this frame's
reverse-Z depth at group 1 binding 0; depth storage follows the union of declared readers. Independently set
`refraction` to sample the shared RGBA16Float scene-colour copy at group 1 binding 1. It contains opaque, sky
and stars before any late effect. All refracting effects share one copy. A depth-only effect creates no copy;
an absent effect creates neither a late pass nor additional storage. The copy and late load/store passes end
the original render pass, so consumers should judge their measured cost when selecting recipes.

Create the late pipeline with the scene's view layout at group 0 and `AfterOpaquePass::input_layout` at
group 1, then any owned bindings. The renderer prepares and binds both shared groups. `draw_after_opaque`
sets the pipeline and owned groups and records draws. Compare sampled reverse-Z depth to the fragment's
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
lists named integer values, inclusive ranges and invalidations. `set_quality_knob` stages a change;
`commit_quality_knobs` applies it before frame preparation. `QualitySettings` coalesces requests and reports
the strongest invalidation. The effect invalidates its own state there; `ViewHistory` also resets the host's
reconstruction history until a frame submits. Ordered-list control names have a `member/` prefix.
The example exposes cache interval, volume slices and bounded catch-up ticks.

When timestamps are available, `SceneGpuTimings::execution(name)` distinguishes `Timed(0)` from `NotRun`
and `Unmeasured`. The last denotes transfers without counters. Disabled timing remains absent.
List order establishes execution only: SEA and CLOUDS still own disjoint medium admission and the explicit
joint or approximate integration policies required by [C08](design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4).
