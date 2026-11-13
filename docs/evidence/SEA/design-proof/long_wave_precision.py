"""SEA design proof: how closely can f32 GPU arithmetic follow the f64 long-wave evaluator?

Run: uv run --with numpy python docs/evidence/SEA/design-proof/long_wave_precision.py
192 deep-water components, wavelength 20-400 m, significant height 6 m (a gale). The point set is the
field cache's reach: up to 4 km from an anchor that sits 50 km from the world origin, after a 24 h session.

wrapped : per component, f64 on the CPU reduces (k . anchor - omega t + phase) to one period and uploads it
          as f32; the GPU adds k . (x - anchor) in f32. This is the selected mechanism.
naive   : the GPU evaluates k . x - omega t from absolute f32 position and time. The wrong substitute.
A pessimistic GPU cosine is modelled by adding a uniform error of +/-1e-4 to every f32 cosine and sine.
"""
import math

import numpy as np

G = 9.80665
rng = np.random.default_rng(7)
n = 192
lam = np.exp(rng.uniform(math.log(20.0), math.log(400.0), n))
k = 2.0 * math.pi / lam
w = np.sqrt(G * k)
theta = rng.normal(0.0, 0.6, n)
kx, kz = k * np.cos(theta), k * np.sin(theta)
shape = np.minimum(lam ** 1.5, np.percentile(lam ** 1.5, 90))  # energy rises with wavelength, then is capped
a = np.sqrt(shape)
hs = 6.0
a *= (hs / 4.0) / math.sqrt(0.5 * float(np.sum(a * a)))
phase = rng.uniform(0.0, 2.0 * math.pi, n)

anchor = np.array([50_000.0, -30_000.0])
t = 86_400.0
m = 20_000
r = 4_000.0 * np.sqrt(rng.uniform(0, 1, m))
ang = rng.uniform(0, 2 * math.pi, m)
off = np.stack([r * np.cos(ang), r * np.sin(ang)], axis=1)
pos = anchor + off

arg = pos[:, :1] * kx + pos[:, 1:] * kz - w * t + phase
h64 = np.sum(a * np.cos(arg), axis=1)
sx64 = np.sum(-a * kx * np.sin(arg), axis=1)

f = np.float32
theta0 = np.mod(anchor[0] * kx + anchor[1] * kz - w * t + phase, 2.0 * math.pi).astype(f)
argw = theta0 + off[:, :1].astype(f) * kx.astype(f) + off[:, 1:].astype(f) * kz.astype(f)
noise_c = rng.uniform(-1e-4, 1e-4, argw.shape).astype(f)
noise_s = rng.uniform(-1e-4, 1e-4, argw.shape).astype(f)
hw = np.sum(a.astype(f) * np.cos(argw), axis=1, dtype=f)
hw_bad = np.sum(a.astype(f) * (np.cos(argw) + noise_c), axis=1, dtype=f)
sxw_bad = np.sum(-(a * kx).astype(f) * (np.sin(argw) + noise_s), axis=1, dtype=f)

argn = pos[:, :1].astype(f) * kx.astype(f) + pos[:, 1:].astype(f) * kz.astype(f) - w.astype(f) * f(t) + phase.astype(f)
hn = np.sum(a.astype(f) * np.cos(argn), axis=1, dtype=f)

print(f"significant height {4.0 * math.sqrt(0.5 * float(np.sum(a * a))):.2f} m, sum of amplitudes {float(np.sum(a)):.1f} m, RMS slope_x {float(np.sqrt(np.mean(sx64 ** 2))):.4f}")
print(f"wrapped, exact f32 cosine   : height error max {np.abs(hw - h64).max() * 1000:8.4f} mm")
print(f"wrapped, cosine off by 1e-4 : height error max {np.abs(hw_bad - h64).max() * 1000:8.4f} mm, slope error max {np.abs(sxw_bad - sx64).max():.2e}")
print(f"naive absolute f32          : height error max {np.abs(hn - h64).max() * 1000:8.1f} mm")
