# SCENE-VIEWS log

- 2026-11-16 M1 builder: added checked camera, clip-plane, extent and shadow-profile admission;
  three public admission tests and `./dev check` pass. No slot transport or secondary draw is built.
  Metal reproduction at `1960293fb`: sharing a height map centred 40 km from the requested camera
  changes 283 pixels, by up to 2,904 stored steps, against M1.1's one-step allowance.
  Proposed: permit per-view height-map ownership, or define an admitted camera/support bound;
  the current design's unrestricted request and shared finite map cannot both meet that reference.
  [The handoff](../../evidence/SCENE-VIEWS/M1-handoff.md) records the mechanism, limits and checks.

- 2026-11-14 Owner, of water, through the lead: "Scenery reflections we must have, that's a main stay in high
  quality modern games. That's the standard we're after." `WATERS` settled a mirror view read by water and
  left the draw to this module, with [a brief](../ARCHITECTURE.md#scene-views).
- 2026-11-14 Planned from the lead's task: "plan the new module SCENE-VIEWS that WATERS M14 depends on. Choose
  methods from first principles and current practice." A design, a plan and a done-when file with three
  milestones: entities in a view on the Mac, cooked placements and the mirror example, then Chrome, cost and
  the close.
- 2026-11-14 Planner's decisions. A view is a slot declared at preparation and asked for each frame, drawn in
  the one frame graph. It shares the scene's shader and world data and has its own view record, visible
  lists, Sun cascades, cluster table and aerial volume. It is cut at a plane by a test in its own fragment
  entry. It takes the frame's jitter and keeps no history. A request is a rigid camera, so a mirror needs no
  reflection matrix.
- 2026-11-14 Changed from the brief. The brief asked whether a view shares the Sun's cascades: it cannot,
  because the shader picks a cascade by depth in its own view and the main view's are fitted to the main
  frustum, so a view has its own. "Reduced detail" is a least size in the view's pixels, since the engine has
  no levels of detail. The air for a camera below the plane is the aerial pass run again with altitudes
  folded about the plane. The milestone `WATERS` M14 waits for is M3, not M1, because M14 needs cooked
  placements and Chrome.
- 2026-11-14 Not chosen: an oblique near plane (the shader derives view depth from stored depth by the
  standard projection), a hardware clip distance (optional in WebGPU; kept as a recipe candidate), the main
  view's cascades (a shadow would appear in the water as its caster came on screen), a second prepared scene,
  and ray query (experimental in wgpu, a proposal for the web).
- 2026-11-14 Planner's reproductions, in [the evidence folder](../../evidence/SCENE-VIEWS/design-proof/).
  Part A: an edge in a half-size view wanders 1.95 px with no jitter and 0.19 px with the frame's. Part B:
  the aerosol below a mirror plane is up to 3.4% too dense unfolded. Part C: an oblique projection's depth
  step is 0.27 m at 3 km for a camera 2 m from the plane, against 0.33 mm.
- 2026-11-14 Asks of other modules: [five seam rulings](../ARCHITECTURE.md#seam-rulings-for-scene-views),
  and one sentence in C07 naming a secondary view. No other module's item changed except `WATERS` M14's
  `depends_on`.
- 2026-11-14 Not read: Unreal's source, a frame study of Red Dead Redemption 2, and how any shipped engine
  jitters a reflection capture. The GPU checks were not run by the planner; each check was tried with the
  loop's own command and fails as it should before the work exists.
- 2026-11-14 Design review asked for changes: [two Must Fix findings](../../evidence/SCENE-VIEWS/design-review.md).
  Both are resolved in the design, the items and [ruling 3](../ARCHITECTURE.md#seam-rulings-for-scene-views).
- 2026-11-14 `folded-air-transport`. The fold height is gone. A request with a clip plane may ask for air
  from the plane: the aerial pass and the shadowed decoder measure each line of sight from where it crosses
  the plane, at real positions, so the image holds the stretch from water to body and nothing else. The
  reader applies the stretch from camera to water once, from the main view's lent aerial pair. Nothing is
  subtracted or divided. [Part D](../../evidence/SCENE-VIEWS/design-proof/README.md) reproduces the review's
  case (40.2 times the true in-scatter with the fold) and the composed path to rounding. M1.5 and M2.3 are
  restated to it, with a Sun across the line of sight and a caster on each stretch.
- 2026-11-14 `knob-output-invariant`. M3.3 now says what a view's knob may change: the reflection inside the
  pool, and no pixel outside it, no pipeline and no resource.
- 2026-11-14 Suggestions taken, all three. A slot is prepared at its largest extent and the extent knob is a
  viewport inside it, so the knob creates nothing. Equality items compare under one shadow profile to one
  step of each stored format, and a larger difference is a defect, not the bound (M1.1, M1.6, M2.1). New
  item M1.11: two slots asked in one frame each hold the bytes they hold alone.
- 2026-11-14 `WATERS` follows: its mirror section asks for air from the plane and its M14.2 composes the
  two stretches. Not run by the planner: any GPU check. Declared: a wave-lifted reflecting point 0.3 m off
  the plane moves a term of 1e-4 of the body's radiance by 3.8%.
- 2026-11-14 Second design review asked for changes: one Must Fix,
  [`air-reference-bound`](../../evidence/SCENE-VIEWS/design-review.md). M1.5 and M2.3 held a view's air to
  bounds `ATMOSPHERE` meets against its own integrator and stored texels, as if they were bounds against
  an independent CPU integration. `ATMOSPHERE` promises none for solar aerial transport.
- 2026-11-14 Resolved without a new accuracy promise. With air from the plane a line's column is laid out
  from its crossing, so it is `ATMOSPHERE`'s own column for an observer there, and the reference for every
  air item is that product drawn from the crossing by the code as it stands. Allowances: one stored step
  for a stored column; for a shaded fragment, one step of the stored colour plus `Q` for each filtered read
  behind it. Transport accuracy stays `ATMOSPHERE`'s and unqualified, and the design says so.
- 2026-11-14 Items. M1.5 is now the stored column; new M1.12 is the shaded fragment, with the casters on
  either side of the crossing. M2.3's expected colour is built from three stored colours of the main view
  with the pool off, on a black and a lit marker from a camera 30 m from the reflection point, and the two
  wrong compositions must miss. M3.1 says every reference in Chrome is Chrome's own.
- 2026-11-14 [Part E](../../evidence/SCENE-VIEWS/design-proof/README.md): air from the view's camera is
  592 to 12,000 stored steps off in three of the four cases and 5 in the fourth, which the design names as
  one no test can tell; a column laid out from the camera doubles the in-scatter 20 m beyond a crossing
  230 m away. The pass's own quadrature is about one step from a fine one in a one-band stand-in.
- 2026-11-14 Not run by the planner: any GPU check, so that a correct view meets the rounding allowance on
  Metal and in Chrome is unrun. No suggestion was made in this review.
- 2026-11-14 Third design review asked for changes: one Must Fix,
  [`air-filter-crossing-boundary`](../../evidence/SCENE-VIEWS/design-review.md). The second revision left a
  line that never crosses the plane with no air. The decoder blends neighbouring lines, so a fragment just
  above the plane's horizon lost in-scatter: 38% in the review's case, 75.6 times the allowance.
- 2026-11-14 Resolved in the volume, with no new filter and no wider allowance. With air from the plane a
  view's volume is `ATMOSPHERE`'s for an observer at the plane's height on every line, crossing or not: the
  pass's column depends only on the observer's altitude and the line's direction, and under a level plane
  every crossing has one altitude. It is then the volume of the main camera M1.12 compares with, texel for
  texel, so the equality holds at the horizon. The decoder still measures from a fragment's own crossing.
- 2026-11-14 Narrowed: air from the plane needs a level plane, and is refused by name for any other. Under
  planes tilted 2° and 20°, lines one row apart start 17 m and 154 m apart in altitude, so a blend is no
  observer's and has no reference. Every plane `WATERS`, `RIVERS` and `PUDDLES` ask for is a water level. M1.5 loses its
  20° tilted case. An assumption line says so.
- 2026-11-14 Items. M1.5 is the whole stored volume against the pass's own for a camera on the plane, on a
  raised plane and at sea level, half of it lines that never cross. M1.12 gains the review's case, a line
  0.2° above the horizon of a plane 50 m up, and shows the emptied lines miss. M1.9 refuses air from a
  plane that is not level. M2 and M3 are unchanged; the example now stands 250 m above sea level.
- 2026-11-14 [Part F](../../evidence/SCENE-VIEWS/design-proof/README.md), with the pass's ground bound.
  Found on the way: the pass stops a descending column at the model's ground, so any observer near sea
  level reads thin haze just above the horizon. In the stand-in a view on the sea is 7% to 88% low there,
  and an ordinary main camera 3 m above the sea 7% to 62% low at 3 km. It is the product as it stands, so
  a view equals it. Proposed to `ATMOSPHERE` in [its log](../ATMOSPHERE/LOG.md), and an assumption here.
- 2026-11-14 Not chosen: renormalising the filter over crossing lines, and clamping a never-crossing line
  to the plane. Each differs from the main view's read by more than rounding, so neither has a reference.
- 2026-11-14 The review made no suggestion. Not run by the planner: any GPU check. Read for this revision:
  `aerial.wgsl`, the three decoders, the sampler in `atmosphere/gpu.rs`, `atmosphere.rs`' altitude range,
  and the camera volume of Hillaire's reference implementation.
- 2026-11-14 `ATMOSPHERE` planned its M9 on this module's proposal
  ([the air at the ground](../ATMOSPHERE/DESIGN.md#the-air-at-the-ground)): the aerial pass carries a line of
  sight on under the model's ground, with the air's density by its own law and the Sun's light dimmed by that
  air. A view's volume is that pass's own, so it follows. The design's passages on what the product loses at
  the horizon, its risk 7 and its last open question describe the pass before M9 and already say a view takes
  whatever the pass becomes; they stand as approved. M1.5 and M1.12 take their references from the pass as it
  runs, so they hold before M9 and after it. No design text or item changed; one assumption was reworded.
- 2026-11-14 Dependency audit, on the lead's task from the owner's word that too few milestones can start at
  once ([the dependency audit](../../evidence/dependency-audit-2026-11-14/README.md)). M1 now waits for `CULLING:M1` in place of `CULLING:M3`: a view needs the three culling modes
  on both routes, which M1 completed, and nothing of the valley's default mode. M1 may be built beside
  `CULLING` M3 in that module's files; the design's sixth risk already covers another builder there. No
  design text or item changed.
