# CLOUDS design proof

One reproduction run while [the design](../../../plans/CLOUDS/DESIGN.md#check-that-it-can-work) was written, on
13 November 2026. It is a short CPU script of the design's core relations with its output beside it. It is
evidence for the design's choices and the source of the figures in its cost account. It is not engine code, no
engine test depends on it, and its constants are a first profile, not the look.

| Script | Output | What it shows |
|---|---|---|
| `cloud_field_proof.py` | `cloud-field-proof.txt` | **A to C, the weather field.** Cells on a space-time jittered lattice in the air frame, with a stratiform deck, take the sky from empty to a closed deck as one control rises; a column's optical depth never steps in time; the field under wind is the still field shifted by the travel, exactly; births are a Poisson process in time with no peak at the generation period. **D, the march.** With a coarse occupancy map, a sky ray needs 17 to 42 texture reads for fair, broken and overcast skies. **E, the Sun's path.** One volume of optical depth summed along the Sun's ray gives ground shade within 0.02 to 0.05 RMS in transmittance of a 5 m brute-force march at a 60° and a 20° Sun (0.09 at 6°), and cloud self-shadow within 0.02 once three local samples cover the first 120 m (0.05 to 0.08 at 6°) |

Run it with `uv run --with numpy python docs/evidence/CLOUDS/design-proof/cloud_field_proof.py` from the
repository root. It takes about a minute.

What it does not show: any look, the temporal reconstruction of the cloud layer, GPU time, or the detail noise.
Those are first measured in M3 of [the done-when file](../../../plans/CLOUDS/DONE.json).
