"""Small counterexamples to the reviewed design, run from the repository root.

These inspect the proposal's arithmetic, not an engine implementation.
"""

import runpy


proof = runpy.run_path("docs/evidence/PUDDLES/design-proof/puddle_proof.py")
earth = proof["wet_run"](
    1.0, 3.0, 0.5, 15.0, 0.0, 3600, rate=10.0, total=3600
)
print("Earth: rain 10 mm/h, soak 15 mm/h, pores 3 mm, relief 0.5 mm")
for seconds in (600, 1080, 1200, 1800, 3600):
    film, pores = earth[seconds - 1]
    print(f"  {seconds}s: film={film:.6f} mm, pores={pores:.6f}")

print("\nMean-ground mirror plane: puddle level 0.05 m, plane 0.20 m")
print("  Post segment from waterline to plane excluded by above-plane clip: 0.15 m")
print("  Level difference 0.15 m is inside WATERS' 1 m plane tolerance")

print("\nOne-canopy rule: opaque roof underside 3 m, half-through crown at 2 m")
lowest_height, share = min(((3.0, 0.0), (2.0, 0.5)))
for height in (0.0, 2.5, 3.1):
    exposure = 1.0 if height > lowest_height else share
    print(f"  Point height {height} m: designed exposure {exposure}")
print("  Vertical rain below the opaque roof should be zero, away from edges")
