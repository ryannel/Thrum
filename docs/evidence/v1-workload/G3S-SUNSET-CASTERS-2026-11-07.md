# G3s sunset caster holes — 7 November 2026

The fixed valley camera showed a 100-code step on the foreground at solar frame 77→78 (18.04–18.05 h). An
earlier note had placed it in the twilight illumination path. It was a shadow leak: the floor rim lit up for
two frames after the Sun had set behind distant terrain, then went dark when direct light ended.

![Solar frames 75–78. Top: main, with the orange rim flash at 76–77. Bottom:
fixed](g3s-first-light/sunset-casters-compare.png)

## Cause

- By frame 76 the Sun's centre is about 0.4° below horizontal. CPU ray truth gives the rim zero visibility: a
  939 m peak 12 km away stands 2.1° above it. The rendered rim read 0.12, 0.31, 0.57 and 0.81 over frames
  76–79.
- Debug captures isolated the last cascade. Its centre texel held a blocker, but up to half the filter's texels
  held nothing more than 1 km upstream.
- Along those rays the source mesh shows what the map should hold. The ray enters ridges through east faces 5.6
  and 10 km out, which face away from the Sun. It exits through west faces at 9.4 and 16.9 km, which meet the
  descending ray almost edge-on.
- The cooked caster pipeline culled back faces, so only those edge-on exits remained. They rasterize as
  sub-texel slivers with holes.

## Fix

The cooked caster pipeline draws both faces. Terrain is an open heightfield that bounds a solid, so a face a ray
enters through is as much a blocker as one it leaves through. With the Sun above the terrain those back faces lie
behind front faces and never win the depth test.

## Checks

- The rim reads 0.00 for frames 76–79. The final-image rim holds at 134–135 codes through frames 73–80, where
  main stepped 135 → 182 → 224 → 133.
- Ray-truth scores for solar frames 22–35 and sequence frames 20–21 are unchanged. The noon, sunrise, sunset,
  dawn and sequence captures are byte-identical to main.
- The largest late step is still frame 78, now 588 changed pixels (was 845) at 320×180. That is the distant
  ridge's lit face dimming over three frames as the disk sets, 214 → 202 → 168 → 131 codes.
- A paired run on a heavily contended GPU (load average 4.2) showed no measurable change: shadow p50 was
  1.19–1.48 ms against 1.40–1.48 ms. Logs are in `tmp/claude-root/g3s-shadow/bench/paired-2sided/` in the root
  session worktree.
- `./dev check` passed. Chrome's `sky-reflections` case passed with an unchanged Wasm SHA-256 (`fb37319b…`),
  since the web build does not include the cooked asset path.

## Independent review

A fresh-context reviewer saw only frames 68–86 and a clip. It judged the distant ridges' light receding smoothly
upward with no reversals or silhouette flicker, and failed the set on residual floor items. Each was checked
against main:
- **A faint warm band below the rim at frame 77** (rows 461–482). This is what remains of the same leak. The row
  470 mean rises 1.3 codes where main rose 2.5, and the peak pixel reaches 133 where main reached 141.
- **Single red specks at frames 69–70 and 73.** They are byte-identical on main, so they predate this change.
- **The floor slowly gains 1–1.5 codes as the Sun sets,** on main as well. This is the sky and ambient term, not
  direct light.

## Open
- The residual band at frame 77 sits on receivers below the rim. It is the same filter path as the flat-floor
  readings below, and the next item for this leak.

- Flat floor points still read 0.12–0.44 visibility at frames 77–79. Their N·L is slightly negative there, so
  they receive no direct light, but the same filter path could leak on a Sun-facing receiver. A GPU oracle for
  cooked casters with a Sun below the horizon does not exist yet.
