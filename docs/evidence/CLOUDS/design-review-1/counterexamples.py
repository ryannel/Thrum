"""Small algebraic checks for the CLOUDS design review; not engine/GPU tests.

Run from the repository root:
    python3 docs/evidence/CLOUDS/design-review/counterexamples.py
"""

import math


print("1. Switching a spatial Sun-path field to one far mean")
local_tau = (0.0, 4.0)
far_tau = sum(local_tau) / len(local_tau)
for tau in local_tau:
    before, after = math.exp(-tau), math.exp(-far_tau)
    print(f"   tau={tau:g}: T immediately above cutoff={before:.6f}, "
          f"below={after:.6f}, jump={abs(after-before):.6f}")
print("   Shrinking the Sun-angle step does not shrink either jump.")

print("2. Equal column means do not imply a pointwise upper bound")
procedural = (1.0, 3.0)
placed = (3.0, 1.0)
print(f"   CPU columns={procedural}; placed columns={placed}")
print(f"   Equal means: {sum(procedural)/2:g} and {sum(placed)/2:g}; "
      f"pointwise bound: {all(b <= a for a, b in zip(procedural, placed))}")
print("   More generally, nonnegative A-B with integral zero requires A=B almost everywhere.")

print("3. Removing detail erosion changes a common-density shadow integral")
sigma, length, erosion = 0.05, 100.0, 0.5
full_tau = sigma * erosion * length
without_detail_tau = sigma * length
print(f"   Visible full density: tau={full_tau:g}, T={math.exp(-full_tau):.6f}")
print(f"   Without detail erosion: tau={without_detail_tau:g}, "
      f"T={math.exp(-without_detail_tau):.6f}")

print("4. Ordinary 3D mips do not preserve twelve stacked shapes")
# Each 128^3 slab is constant in X and Y. Thus texture.rs's eight-tap
# downsample reduces exactly to this rounded two-tap Z recurrence.
z = [20 * slab for slab in range(12) for _ in range(128)]
width = height = 128
level = 0
while True:
    if level >= 7:
        print(f"   mip {level}: {width}x{height}x{len(z)}, values={z}")
    if width == height == len(z) == 1:
        break
    z = [(z[2*i] + z[min(2*i+1, len(z)-1)] + 1)//2
         for i in range(max(1, len(z)//2))]
    width, height = max(1, width//2), max(1, height//2)
    level += 1
print("   Mip 7 has one texel per slab; mip 8 already averages adjacent shapes.")
