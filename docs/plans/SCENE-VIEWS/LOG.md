# SCENE-VIEWS log

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
