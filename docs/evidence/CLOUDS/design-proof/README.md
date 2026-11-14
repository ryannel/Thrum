# CLOUDS design proof

Three reproductions. The first was run while [the design](../../../plans/CLOUDS/DESIGN.md#check-that-it-can-work) was written, on
13 November 2026, the second for its revision a day later, and the third when the design was judged again from
first principles on 14 November. Each is a short CPU script of the design's
relations with its output beside it. They are evidence for the design's choices and the source of the figures
in its cost account. They are not engine code, no engine test depends on them, and their constants are a first
profile, not the look.

| Script | Output | What it shows |
|---|---|---|
| `cloud_field_proof.py` | `cloud-field-proof.txt` | **A to C, the weather field.** Cells on a space-time jittered lattice in the air frame, with a stratiform deck, take the sky from empty to a closed deck as one control rises; a column's optical depth never steps in time; the field under wind is the still field shifted by the travel, exactly; births are a Poisson process in time with no peak at the generation period. **D, the march.** With a coarse occupancy map, a sky ray needs 17 to 42 texture reads for fair, broken and overcast skies. **E, the Sun's path.** One volume of optical depth summed along the Sun's ray gives ground shade within 0.02 to 0.05 RMS in transmittance of a 5 m brute-force march at a 60° and a 20° Sun (0.09 at 6°), and cloud self-shadow within 0.02 once three local samples cover the first 120 m (0.05 to 0.08 at 6°) |

| `revision_proof.py` | `revision-proof.txt` | Run on 14 November 2026 for the design's revision, against the counterexamples of [the design review](../design-review.md). **A, the Sun's path.** With the product's columns held at a floor elevation, the factor under open sky, in a gap and at a cloud bank's edge changes continuously as the Sun sinks from 8° to 1°, where the replaced rule jumped by 1.0; a receiver leaving the field fades to a far mean transmittance with no jump. **B, placed shapes.** With the query defined as the shape's own column, an L-shaped tower's volume form, column form and their blends never exceed the query at any point, where a dome of equal mean is exceeded at 26% of points. **C, the coarse density.** The Sun's path with the detail noise at its mean is within 0.001 RMS in transmittance of the full density over an 80 m column and reads no noise. **D, slabs.** A stacked volume read down to one texel a slab, with the coordinate held inside the slab, never reads a neighbour |
| `first_principles_proof.py` | `first-principles-proof.txt` | Run on 14 November 2026 for [the first-principles judgement](../../../plans/CLOUDS/DESIGN.md#judged-from-first-principles). Its parts carry on from the design's lettering. **E, the first order's phase.** The droplet's own phase (Henyey–Greenstein plus Draine, Jendersie and d'Eon 2023) against the retained reference's lobe: the lobe is 3.9 times too bright at 5° from the Sun, 46 times too dim at half a degree and 2 to 3.4 times too dim looking away from the Sun, with the same energy and nearly the same mean cosine. **F, drift.** A 60 Hz tick shown on a 120 Hz display steps a near cloud by 0 then 37 pixels at 60× in a 40 m/s wind; taken at the display's time it moves 18 then 18 |

Run the first with `uv run --with numpy python docs/evidence/CLOUDS/design-proof/cloud_field_proof.py` from the
repository root; it takes about a minute. Run the second with
`python3 docs/evidence/CLOUDS/design-proof/revision_proof.py` and the third with
`python3 docs/evidence/CLOUDS/design-proof/first_principles_proof.py`; each needs only the standard library and
takes a few seconds.

What they do not show: any look, the temporal reconstruction of the cloud layer, GPU time, or the detail noise
as the engine will generate it. Those are first measured in M3 of
[the done-when file](../../../plans/CLOUDS/DONE.json).
