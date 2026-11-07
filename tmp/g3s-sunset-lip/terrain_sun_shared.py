import math
from geometry import trace, unit

solar_radius = math.radians(.266)

def elevation_and_horizon(hours, origin):
    hour_angle = math.tau * (hours / 24 - .5)
    sin, cos = math.sin(hour_angle), math.cos(hour_angle)
    up = math.sqrt(.5) * cos
    azimuth = unit((-sin, 0, -up))
    sun = math.asin(up)
    low, high = 0.0, math.pi / 4
    for _ in range(20):
        trial = (low + high) / 2
        ray = (azimuth[0] * math.cos(trial), math.sin(trial), azimuth[2] * math.cos(trial))
        if trace(ray, origin) is None:
            high = trial
        else:
            low = trial
    return sun, (low + high) / 2

def disk(sun, horizon):
    fraction = max(-1, min(1, (sun - horizon) / solar_radius))
    return .5 + (math.asin(fraction) + fraction * math.sqrt(1-fraction*fraction)) / math.pi
