"""Independent real-number checks for the candidate R0 light/storage bound.

Run: python3 docs/evidence/render/R0-ARITHMETIC-BOUND-ORACLE.py
This is not a WGSL/backend or pixel proof.
"""

from math import cos, pi, sin, sqrt
from random import Random
from struct import pack


HALF_MAX = 65_504.0
STORAGE_SCALE = 1 / 1024


def ggx_d(alpha: float, c_h: float) -> float:
    alpha2 = alpha * alpha
    return alpha2 / (pi * (1 - c_h * c_h + alpha2 * c_h * c_h) ** 2)


def smith_g(alpha: float, c_i: float, c_o: float) -> float:
    alpha2 = alpha * alpha
    g_i = sqrt(alpha2 + (1 - alpha2) * c_i * c_i)
    g_o = sqrt(alpha2 + (1 - alpha2) * c_o * c_o)
    return 2 * c_i * c_o / (c_i * g_o + c_o * g_i)


def direct_specular(
    alpha: float, wi: tuple[float, float, float], wo: tuple[float, float, float],
    f0: float, illuminance: float,
) -> float:
    c_i, c_o = wi[2], wo[2]
    if c_i <= 0 or c_o <= 0:
        return 0.0
    v = tuple(a + b for a, b in zip(wi, wo))
    length = sqrt(sum(x * x for x in v))
    h = tuple(x / length for x in v)
    c_h = h[2]
    wo_h = sum(a * b for a, b in zip(wo, h))
    fresnel = f0 + (1 - f0) * (1 - wo_h) ** 5
    d = ggx_d(alpha, c_h)
    g = smith_g(alpha, c_i, c_o)
    return illuminance * c_i * d * g * fresnel / (4 * c_i * c_o)


def envelope(min_roughness: float, channel_illuminances: list[float],
             ambient: float, emission: float) -> float:
    alpha = max(0.045, min_roughness) ** 2
    direct = sum(channel_illuminances)
    return STORAGE_SCALE * (
        emission + ambient / pi + direct * (1 / pi + 1 / (2 * pi * alpha ** 3))
    )


def hemisphere(rng: Random) -> tuple[float, float, float]:
    c = rng.random()
    phi = 2 * pi * rng.random()
    s = sqrt(1 - c * c)
    return s * cos(phi), s * sin(phi), c


def main() -> None:
    n = (0.0, 0.0, 1.0)
    alpha_min = 0.045 ** 2
    d_normal = ggx_d(alpha_min, 1.0)
    assert abs(d_normal - 77_624.7245685) < 0.001
    dielectric = direct_specular(alpha_min, n, n, 0.04, 100_000)
    metal = direct_specular(alpha_min, n, n, 1.0, 100_000)
    assert abs(dielectric - 77_624_724.5685) < 0.01
    assert abs(metal - 1_940_618_114.213) < 0.1
    assert STORAGE_SCALE * dielectric > HALF_MAX
    assert STORAGE_SCALE * metal > HALF_MAX
    try:
        pack("e", STORAGE_SCALE * dielectric)
    except OverflowError:
        pass
    else:
        raise AssertionError("f16 witness unexpectedly fit")

    saturated_blue = 1 / 0.0722
    illustrative = envelope(
        0.32,
        [100_000, *(1_000 * saturated_blue for _ in range(4))],
        1_000,
        10_000,
    )
    assert abs(illustrative - 22_552.8633298) < 0.001
    assert illustrative < HALF_MAX

    # The analytical inequality is the proof. This deterministic search
    # catches formula/transcription mistakes at varied grazing directions.
    rng = Random(0xA11CE)
    max_ratio = 0.0
    for _ in range(100_000):
        alpha = max(0.045, rng.random()) ** 2
        wi, wo = hemisphere(rng), hemisphere(rng)
        actual = direct_specular(alpha, wi, wo, rng.random(), 1.0)
        upper = 1 / (2 * pi * alpha ** 3)
        ratio = actual / upper
        assert ratio <= 1 + 1e-12
        max_ratio = max(max_ratio, ratio)
    print(f"normal D={d_normal:.7f}; dielectric stored={STORAGE_SCALE * dielectric:.2f}; "
          f"metal stored={STORAGE_SCALE * metal:.2f}")
    print(f"illustrative fixture stored bound={illustrative:.2f}; "
          f"randomized max/analytical bound={max_ratio:.6f}; 100000 samples")


if __name__ == "__main__":
    main()
