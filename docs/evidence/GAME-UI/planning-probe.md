# GAME-UI planning probe, 13 November 2026

Throwaway calculations the planner ran for [the design](../../plans/GAME-UI/DESIGN.md), on questions that reading
alone could not settle. None is engine code and none ran on a GPU, so nothing here is a baseline: the milestones
repeat each result through the engine. The source stayed in the planner's scratch folder and is not kept.

Machine: Apple M3 Max, macOS 26.6.2. Python 3 for probes 2 to 5. Probe 1 is a Swift script on CoreText, used only
to measure glyph boxes; no font or CoreText output is kept.

## 1. How large is a cooked font?

**Method.** One outline font (the system's Arial, as a stand-in for any Latin text face) at four sizes, 12, 16, 24
and 40 points, and three scales. For each of 334 characters (Basic Latin, Latin-1 Supplement, Latin Extended-A and
fifteen punctuation marks and arrows) the glyph's bounding box at size × scale was rounded out to whole texels
and given a one-texel transparent border. The table sums those boxes. Packing loss, mip levels and an outline
variant are not included.

| Scale | Texels, four sizes | As RGBA8 | Largest glyph box (40 pt) |
|---|---|---|---|
| 1 | 413,774 | 1.6 MiB | 42×42 |
| 2 | 1,402,768 | 5.4 MiB | 82×81 |
| 3 | 2,984,796 | 11.4 MiB | 122×121 |
| All three | 4,801,338 | 18.3 MiB | |

**Reading.** One scale of one face fits a single 2048×2048 page with room to spare, and the three scales together
are about three times the one in use. So a font cooks to **one sheet for each scale** and only the scale in use is
resident: 11.4 MiB at worst in place of 18.3. An outline variant roughly doubles a scale.

## 2. Does rounding the pen keep spacing even?

**Method.** Two ways to put glyphs on whole pixels were compared over 20,000 random lines of 40 glyphs with
fractional advances between 2 and 30 pixels. (a) The pen keeps its fraction and each glyph is drawn at the pen
rounded to a whole pixel. (b) Each advance is rounded once, when the font is cooked. Row (a) was measured; row (b)
follows from the rounding itself.

| | Worst offset of a glyph | Worst error in the gap between two neighbours | Same pair, same gap everywhere |
|---|---|---|---|
| (a) rounded pen | 0.5 px | 1.0 px | No: the gap depends on where the line starts |
| (b) rounded advances | 0 | 0.5 px against the outline's own advance | Yes |

**Reading.** Under (a) the gap between the same two digits changes by a pixel when a right-aligned counter grows
a digit or a label moves by part of a pixel, which reads as shiver. Under (b) a run is rigid: its glyphs sit at
whole-pixel offsets from its origin that never change, and only the origin snaps. The design takes (b).

## 3. Does the event-ownership rule ever hand the game half of a press?

**Method.** A model of [the routing rule](../../plans/GAME-UI/DESIGN.md#8-who-gets-an-event): six contacts (keys,
pointer buttons and touches alike), pressed, moved and released at random, some pressed on a control; an exclusive
scope opened and closed at random. 20,000 runs of 400 events. After every event the model checked four things:
the game never gets a release for a press it was not given; the game gets no event while an exclusive scope is
open; after an interruption the game holds nothing; and when no scope is open, what the game believes is held is
exactly the contacts it owns.

**Result.** No run broke any of the four.

## 4. Can direction keys reach every control?

**Method.** [The direction rule](../../plans/GAME-UI/DESIGN.md#7-focus) was run on 96 grids (1 to 6 columns, 1 to 4
rows, four cell shapes and gaps), on a ragged layout of three cards above two centred ones, and on 2,000 random
scatters of six controls that do not overlap. Reachable means every control can be reached from the first by
direction keys alone. Reversible means that a move followed by the opposite move returns to where it began.

| Layout | Reachable | Reversible |
|---|---|---|
| 96 rows, columns and grids | 96 of 96 | 96 of 96 |
| Three cards above two | Yes | No |
| 2,000 random scatters | 1,996 of 2,000 | Not measured |

**Reading.** The design claims reachability and reversibility for rows, columns and grids only. Elsewhere a game
names a control's neighbours, and next and previous, which walk the tree's order, always reach every control.

## 5. Do authored colours and a one-code allowance survive a half-float target?

Added on 13 November 2026 for the float mode of [the interface stage](../../plans/GAME-UI/DESIGN.md#1-the-interface-stage),
after [the design review](design-review.md) found the EDR output path had no stage.

**Method.** Each 8-bit code was decoded with the sRGB curve, rounded to the nearest half-float, encoded again
with the curve and multiplied by 255. Then every half-float from 0 to 16 was encoded with the same curve,
continued above 1 by the same formula, and the difference between each neighbouring pair was taken, in codes.
The arithmetic is IEEE half-float rounding in Python's `struct`; no GPU ran.

| Question | Result |
|---|---|
| Worst error of an opaque code after the half-float | 0.037 of a code; all 256 round back to themselves |
| Largest step between neighbouring half-floats, values up to 1 | 0.08 of a code |
| Up to 2, 4, 8 and 16 | 0.11, 0.15, 0.20 and 0.26 of a code |

**Reading.** An opaque texel can be required to return its authored code in the float mode, with 0.46 of a
code to spare, which agrees with [RENDER-2D's probe](../RENDER-2D/planning-probe.md) of the same target format.
A comparison "within one code" of the sRGB curve is meaningful up to a headroom of 16: storage accounts for at
most 0.27 of it, and the rest is left for the GPU's own rounding of a blend, as in the 8-bit mode. A GPU's
blend was not measured; M1.1 and M1.5 do that.
