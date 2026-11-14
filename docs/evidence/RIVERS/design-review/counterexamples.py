"""Arithmetic checks of the RIVERS design at 51210c3771e9.

These implement the written sampling/advection rules, not the unbuilt engine.
Run from the repository root with Python 3; no dependencies or generated files.
"""


def disconnected_hollow():
    # A lateral section, repeated along a reach wide enough to resolve each part.
    # Low flow stays in the main channel; high flow overtops the separating ridge.
    bed = [0.0, 1.1, 0.2, 1.5]
    low_level, high_level = 1.0, 1.2
    low_wet = [True, False, False, False]
    high_wet = [z < high_level for z in bed]
    footprint = high_wet  # The design also grows this by a dry margin.
    low_rows = [low_level for _ in bed]  # Nearest low-sample wet cell is cell 0.
    encoded_wet = [inside and level > z for inside, level, z in zip(footprint, low_rows, bed)]
    assert not low_wet[2] and encoded_wet[2]
    print("DRY SUPPORT: disconnected hollow should be dry at the low sample.")
    print(f"  bed={bed}, low level={low_level}, high level={high_level}")
    print(f"  expected wet={low_wet}; chart wet={encoded_wet}")
    print(f"  invented depth in hollow={low_rows[2] - bed[2]:.2f} m")


def simultaneous_rings():
    # Both packets are in one connected region, still separated spatially.
    # Equal wave energy/source weights; no land barrier divides the two waters.
    pool_current, riffle_current, seconds = 0.2, 1.5, 5.0
    mean = (pool_current + riffle_current) / 2
    error = abs(mean - pool_current) * seconds
    assert error > 1.5  # M6.3's centre tolerance.
    print("RING CARRIAGE: two equal-energy packets in one connected region.")
    print(f"  common current={mean:.2f} m/s; pool/riffle={pool_current}/{riffle_current} m/s")
    print(f"  after {seconds:.0f} s, pool and riffle centres each err by {error:.2f} m")
    # Even choosing a different common time-varying velocity cannot satisfy both
    # centres: their integrated translations have to be equal.
    desired_separation = abs(riffle_current - pool_current) * seconds
    print(f"  desired translations differ by {desired_separation:.2f} m; two 1.5 m tolerances span 3 m")
    assert desired_separation > 3.0


def join_current():
    solver_current, basin_current, join_weight = 0.5, 0.0, 0.5
    map_current = (1 - join_weight) * solver_current + join_weight * basin_current
    assert map_current != solver_current
    print("JOIN: a wet cell inside the existing WATERS join.")
    print(f"  solver current={solver_current:.2f} m/s; joined map current={map_current:.2f} m/s")
    print("  M1.6's equality at every wet cell cannot coexist with this nonzero join.")


if __name__ == "__main__":
    disconnected_hollow()
    simultaneous_rings()
    join_current()
