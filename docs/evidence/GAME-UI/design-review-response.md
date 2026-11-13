# GAME-UI: the planner's answer to the design review, 13 November 2026

[The design review](design-review.md) of `faa2d843` asked for changes. This page records what the planner
decided for each finding and what was not chosen. [The design](../../plans/GAME-UI/DESIGN.md) and
[the done-when file](../../plans/GAME-UI/DONE.json) hold the result; this page is history.

## MF1: the float mode

The stage had no form for the EDR output that [APPLE-PRESENTATION](../../plans/APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence)
already designs, in which the present pass maps the scene into a linear `Rgba16Float` drawable and there is no
encoded image and no later copy. Settled against that design and [T2's](../../plans/T2-CAPTURE/DESIGN.md), in
mechanisms 1 and 2 and in ruling 1 of the module list:

| Asked by the review | Settled as |
|---|---|
| Target | The float target the present pass's transform writes: the drawable, or the final engine texture T2 has the host make for a captured frame |
| Placement | After the transform's draw, before the frame is tapped or presented, in the present pass's own render pass where it allows |
| Blend | The target's own linear blend, with no view. A row's tint is clamped to 0 to 1 first |
| SDR white | 1.0, as APPLE-PRESENTATION's transform defines it, at any headroom |
| Capture point | The float target after the stage, with its encoding in the capture's record |
| Uncovered pixels | Bit for bit, values above 1 included |

The 8-bit mode is unchanged and no intermediate was added. A float frame is compared through the sRGB curve,
continued above 1, in codes. [Probe 5](planning-probe.md#5-do-authored-colours-and-a-one-code-allowance-survive-a-half-float-target)
shows authored codes and a one-code allowance survive half-float storage to a headroom of 16.

M1.1, M1.2, M1.4, M1.5, M1.6 and M1.10 were written again to run each mode. M1.4 carries the proof that the
stage follows the EDR transform, because a 2D scene has no tone curve that would show an interface drawn too
early. Later milestones run in the forced 8-bit mode, since what they add is decided before the stage.

Researched: NVIDIA's [Rendering a Game for HDR Display](https://developer.nvidia.com/rendering-game-hdr-display),
which composites the interface over the tone-mapped scene and warns that a highlight bleeds through a
part-transparent panel. **Not chosen:** a layer of its own for the interface, composited by a shader that dims
the scene beneath it. It costs a full-screen target and a composite in every frame. It is an assumption in the
plan, for the owner.

`GAME-UI` M1 follows `RENDER-2D:M5`, which follows the V1 exit, which holds `APPLE-PRESENTATION` M6. So the
float mode's present pass exists before M1 starts, and no `depends_on` entry was added.

## MF2: a list and its sheets change together

The design let a font's sheets and table switch one frame before the text laid out from them. New mechanism 11:
a paint list carries a stamp of the sheet generations and the scale it was measured from, and the stage draws a
list only with exactly those sheets. The interface lays out a prepared layout beside the one being drawn, is a
reader of the publication under [CONTENT-LOOP's](../../plans/CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2)
rules 3 and 7, and takes the new layout when it reads the switch frame submitted. A discarded switch frame
leaves both layouts, and the next frame makes the same switch. Ruling 3 of the module list carries the reader;
that design's text did not change.

The same rule now carries a change of scale and an edit to interface art, which had the same fault: advances
differ at each scale, and an image node takes its size from its frame.

**Not chosen:**

- Letting the list lag and keeping the old sheets resident for it. One publication would then show in two
  frames, against CONTENT-LOOP's rule 4.
- Switching the shell's layout first and holding the old list on the render side. The old text would freeze
  while sheets load, and a refused publication would need a layout rolled back.

M2.6, M2.8 and M2.9 were written again to compare every submitted frame of a change with a fresh start on the
old sheets and on the new, with a font whose glyphs grow wider at the same cooked size. M3.14 is new, for
boxes. M4.2 says which layout a press is tested against.
