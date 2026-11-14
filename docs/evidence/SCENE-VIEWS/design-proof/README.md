# SCENE-VIEWS design proofs

Reproductions run while [the design](../../../plans/SCENE-VIEWS/DESIGN.md#check-that-it-can-work) was written,
on 14 November 2026. They are evidence for the design's choices. They are not engine code, no engine test
depends on them, and none is a GPU or a visual measurement.

Run `python3 views_proof.py`, about a second; its output is `views-proof.txt`.

| Part | What it shows |
|---|---|
| A | Jitter: an edge in a view half the main view's size each way, read by projection and averaged over 16 jitter phases. With no jitter in the view the averaged edge wanders 1.95 px as the true edge slides; with a jitter of the view's own, 0.33 px; with the frame's phase at the view's extent, 0.19 px, and it sits 0.02 px from the true edge |
| B | Air below a mirror plane: taken at the altitude it stands at, the part of a line of sight under the plane is 0.5% too dense in molecules and 3.4% in aerosol for a camera 40 m up, and the view's camera stands that far below the plane. Folding altitudes about the plane removes both |
| C | An oblique near plane crowds stored depth toward one value: one step of a 32-bit depth spans 0.27 m at 3 km for a camera 2 m from the plane and 5.4 m for one 0.1 m from it, against 0.33 mm for the standard reversed projection |
