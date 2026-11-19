# Design proof: the lookout's rim

19 November 2026, by the planner of `SHOWCASE` M9. This is arithmetic on the checked-in generator,
`examples/v1-game/assets/showcase/generate.py`, at main `ebb79d64c`. Nothing here was drawn on a GPU. The
Sun direction is taken from the two N·L values `ATMOSPHERE` M6's builder measured at the 17.4 h stop, 0.104 on
flat ground and 0.573 on its near-lip normal (`docs/evidence/ATMOSPHERE/M6/README.md`, on the branch
`build/atmosphere` at `b65cab08f` until M6 merges), which give a unit direction of about (−0.992, 0.104, 0.07).

## What the mesh stores at the rim

The row of vertices nearest the camera (z = 16.58 m), going west from the camera at x = 0. The camera stands at
(0, 244.92, 16), about 5 m above the lookout's top.

| x (m) | Height (m) | Stored normal | True slope dh/dx | N·L stored | N·L of the true surface |
|---:|---:|---|---:|---:|---:|
| −24.7 | 239.96 | (0.002, 1.000, −0.007) | −0.017 | 0.10 | 0.09 |
| −40.4 | 240.02 | (−0.029, 1.000, 0.008) | 0.007 | 0.13 | 0.11 |
| −60.3 | 238.61 | (−0.542, 0.840, 0.010) | 0.496 | 0.63 | 0.53 |
| −84.7 | 205.83 | (−0.874, 0.486, 0.001) | 2.044 | 0.92 | 0.94 |
| −113.6 | 138.47 | (−0.899, 0.439, 0.000) | 2.343 | 0.94 | 0.95 |

The top is flat to x = −55 and then rolls over. The mesh has one column at −40.4 and the next at −60.3, so one
strip of triangles 20 m wide spans the flat ground and the start of the fall. It drops 1.4 m, a slope of 4°,
and the camera sees all of it. Its far vertices carry a normal turned 33° toward the west, because the
generator's difference takes in the 33 m drop to the column after. Shading interpolates across the strip, so 20 m
of nearly flat ground climbs from N·L 0.13 to 0.63 at the 17.4 h Sun. From 5 m up, that strip is about 25
pixels tall at 1080p; the builder measured its steepest part across 8. That is the bright band.

## What the true surface shows

The same row, sampled every metre from the generator's height function. A point is seen while the ground falls
away more gently than the camera's line of sight to it.

| x (m) | Slope dh/dx | N·L | Sight-line slope | |
|---:|---:|---:|---:|---|
| −40 | 0.008 | 0.112 | 0.122 | seen |
| −45 | 0.009 | 0.115 | 0.107 | seen |
| −50 | 0.018 | 0.123 | 0.096 | seen |
| −53 | 0.045 | 0.150 | 0.092 | seen |
| −55 | 0.017 | 0.124 | 0.090 | seen |
| −56 | 0.098 | 0.202 | 0.089 | hidden |
| −58 | 0.289 | 0.377 | 0.093 | hidden |
| −60 | 0.465 | 0.515 | 0.103 | hidden |

So the ground the camera can see keeps N·L between 0.07 and 0.15 out to x = −55, and the part that turns to the
Sun is behind the edge. A mesh whose visible triangles carry the normals of the ground they cover shows no band.
Rows added across a smooth roll-over, with the generator's difference kept, still tilt the last visible row a
little: with rows at −53, −55 and −57 the row at −55 gets a slope of about 0.05, N·L 0.15. A crease, where the
top's last row keeps the top's normal and the face beyond has rows of its own, leaves none.

## How far the fault reaches

For every triangle that faces the camera in the west view, the largest difference between a vertex's stored N·L
and the triangle's own:

| Distance from the camera (m) | Triangles | Largest difference |
|---|---:|---:|
| 0 to 100 | 71 | 0.471 |
| 100 to 500 | 346 | 0.582 |
| 500 to 1,500 | 1,012 | 0.020 |
| 1,500 to 3,000 | 1,276 | 0.021 |
| 3,000 to 6,000 | 2,375 | 0.030 |
| 6,000 to 12,000 | 2,470 | 0.076 |
| 12,000 to 30,000 | 1,485 | 0.126 |

The fault is the lookout's rim and nothing farther: beyond 500 m the stored normals agree with the triangles. The
thin bright line on the far ridge in the same frame is therefore not this fault, and stays with `ATMOSPHERE` M6
to judge.

## The sunrise stop

Not computed here. `ATMOSPHERE` M6's builder cast rays through the shipped triangles: at 6.1 h the Sun's centre
is about 1.06° above level, and rays from the sphere and the rock to its centre and to eight points on its limb
all meet the east terrain 6.94 to 7.27 km away. The lookout is in that ridge's shade at the stop, so no prop
there can cast a shadow. The same rays clear the terrain at noon.

## To reproduce

Load `generate.py` as a module and call `terrain()`, which returns positions, normals, texture coordinates and
triangles. Row `j` nearest z = 16 and columns 70 to 80 give the first table; `ground(x, z)` differenced over
0.1 m gives the second; the third takes each triangle whose face normal points toward the camera and whose
centre lies within 40° of west.
