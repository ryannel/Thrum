"""Independent check of the revised join composed with the revised depth limiter.

Run with Python's standard library. This is a specification calculation, not
an implementation or GPU test. The scene has no wind, a 0.5 m amplitude swell
at sea, and a chart whose own waves exclude swell. At the mouth's edge the
join has already made mean level, depth and raw wave gain identical.

DESIGN.md 309-318 mixes level/current/gains/type. Lines 377-381 separately
make swash depth one number per body per tick, from that body's waves.
Part K of the planner's proof omits this latter limiter.
"""

import math


def crest(amplitude, depth, swash_depth):
    significant_height = 2 * math.sqrt(2) * amplitude
    limited_height = min(significant_height, max(0, 0.6 * (depth + swash_depth)))
    return limited_height / (2 * math.sqrt(2))


amplitude = 0.5
sea_hs = 2 * math.sqrt(2) * amplitude
sea_swash = 0.7 * sea_hs  # choose a margin height >= 1 m
chart_swash = 0.0        # no wind; no swell in the chart's own waves

print('Mouth edge: matched mean levels and depth; joined raw swell gain = 1 on both sides')
print(f'Sea swash depth: {sea_swash:.9f} m; chart own swash depth: {chart_swash:.9f} m')
print('depth(m) sea crest(m) chart crest(m) jump(m)')
for depth in (4.0, 1.0, 0.25, 0.0):
    sea = crest(amplitude, depth, sea_swash)
    chart = crest(amplitude, depth, chart_swash)
    print(f'{depth:8.2f} {sea:12.9f} {chart:14.9f} {sea-chart:11.9f}')

assert abs(crest(amplitude, 0.25, sea_swash) - crest(amplitude, 0.25, chart_swash) - 0.21) < 1e-12
print('The 0.25 m mouth has a 0.21 m step despite identical joined raw gains.')
print('A pointwise rule for swash depth/support at joins would change this result; none is specified.')
