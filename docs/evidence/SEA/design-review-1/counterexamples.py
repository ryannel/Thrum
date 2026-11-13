"""Small algebraic witnesses for the SEA design review; not engine tests.

Run from the repository root:
  python3 docs/evidence/SEA/design-review/counterexamples.py
"""

import math


def main():
    # Horizontal pinhole camera, sea at y=0. Screen ordinate divided by focal length.
    camera_height, body_height, distance = 2.0, 2.0, 20.0
    direct = (-camera_height / distance, (body_height - camera_height) / distance)
    reflected = (-(camera_height + body_height) / distance, -camera_height / distance)
    receiving_distance = camera_height * distance / (camera_height + body_height)
    print("Reflection support, calm sea:")
    print(f"  body screen ordinate interval: {direct}")
    print(f"  reflected body interval:       {reflected}")
    print(f"  sea point reflecting body top: {receiving_distance:.1f} m from camera")
    print("  Padding must cover the receiving interval, not only the opaque silhouette.")

    # Literal alternating-tick-bank update, starting from consecutive committed endpoints.
    banks = {0: 10, 1: 9}
    new_tick = 12
    banks[new_tick % 2] = new_tick
    print("\nSkipped tick, literal alternating bank update:")
    print(f"  after 10 -> 12: {sorted(banks.values())}; required endpoints: [11, 12]")
    assert sorted(banks.values()) != [11, 12]

    # One-channel, clear-air limit isolates the exposure mismatch from medium ownership.
    radiance, exposure, transmission = 10.0, 0.25, 0.5
    scene_colour = radiance * exposure
    wanted = exposure * transmission * radiance
    raw_copy_composed = exposure * transmission * scene_colour
    print("\nGathered scene colour used as physical behind-surface radiance:")
    print(f"  expected stored contribution: {wanted:.3f}")
    print(f"  raw-copy substitution:        {raw_copy_composed:.3f}")
    print("  Dividing out exposure alone does not remove the source's aerial transport.")

    # Equal-lifetime transfer F -> A -> sink, no external births, F(0)=1, A(0)=0.
    time_over_lifetime = 0.1
    fresh = math.exp(-time_over_lifetime)
    aged = time_over_lifetime * math.exp(-time_over_lifetime)
    print("\nFoam with no external cause, fresh -> aged -> sink:")
    print(f"  fresh: 1 -> {fresh:.6f}; aged: 0 -> {aged:.6f}")
    print("  Aged increases while total inventory decreases.")
    assert aged > 0 and fresh + aged < 1


if __name__ == "__main__":
    main()
