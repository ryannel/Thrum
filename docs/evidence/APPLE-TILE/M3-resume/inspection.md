# M3 builder visual inspection

The inspection plan is in [OWNERSHIP.md](OWNERSHIP.md), written before these captures. Source is
`e41686a0f4164cd164fed403dd0e75c63946937b`; the native executable is
`b8a331bb6646a72ae4b7fc4e6632de0a5eb47c5af269044f9b5d6b96e271c6e3`.
Coverage uses chronological 10 fps samples across each complete original-time sequence, selected full-resolution
frames and a native-scale crop. Source verification and movie PTS checks precede image selection.

## Native observations

- Inspected the retained Sun, night, mirror/local-light and 1000×800 letterbox full frames whose pixels the fresh
  M0 exact matrix matches. Sun disk/horizon, starfield, terrain silhouettes, mirror, cubes, rock, light and shadows
  remain. Both Unorm/sRGB surface branches and source-coordinate mapping pass exact comparisons.
- Inspected both complete route strips for each recipe. The approach, camera cut and near-plane progression retain
  scene content and exposure. Portable full frames 775/777 bracket history epoch 1→2 at submission 776; Apple full
  frame 760 confirms intact post-cut content. No stale pre-cut view or newly missing geometry was observed.
- Inspected every variants and render-stall strip under both recipes. The day/night switch introduces/removes stars
  while retaining the visible sphere/rock and terrain. Apple night full frame 493 retains the starfield, dark
  silhouette and reflection. The deliberately held output in the stall cases resumes its pan intact.
- Inspected `portable-route/native-shadow-crop.png` at its native 640×480 size. It is a crop of full frame 775 at
  x=950, y=400, made with `ffmpeg -vf crop=640:480:950:400`. Cube edges, local-light falloff and shadow content remain.

The exact matrices establish linear/SDR bytes and sample/exposure identity; these observations establish visible
coverage. Scene-linear half-float words are compared numerically. Original-time previews and full selected source
PNGs are retained alongside the strips. Physical panel cadence, compositor output and performance are unmeasured.

## Chrome observations

Inspected all three day strips, both night strips, all three resize strips, full day/night post-cut frames and the
1000×800 composed-canvas PNG. Partial final strip sheets contain padding after the terminal source sample.
The approach, cut and near-plane progression retain the terrain, mirror, cubes, rock, local light and shadows.
The bright night light disk and reflection are saturated as in the byte-identical retained Chrome oracle.
Stars and the night terrain silhouette remain intact; no stale pre-cut view or missing content was observed.

The complete day and night runs each retain 482 source images; resize retains 480. All three have zero renderer and
export omissions. Both route histories advance 1→2. Resize records presenter generations 1→2→3, first at 961×541
and then at 1000×800 through the existing CDP observer. Its composed-canvas image retains centered black bars and
the final near-plane source proportions. The observer records CSS/backing bounds, DPR 1 and page clock. This is
headless Chrome composition, separate from T2 pixels and physical display output.

The first day attempt had zero renderer omissions and two page export omissions at the unchanged eight-in-flight
cap. It is retained as a refused observer run and is not counted as complete motion evidence. One retry with the
same capture settings completed; its export peak was 5 (night 4, resize 3). No queue, threshold or renderer change
was made. Focused tests passed: 2 Metal input tests, 5 timing tests and 8 optional-layout frame-stat tests.
