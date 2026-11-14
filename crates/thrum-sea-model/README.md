# thrum-sea-model

CPU-only physical reference for SEA. Coordinates are horizontal x/z in metres,
vertical y is up, angles are radians, time is seconds. `Forcing` holds wind at
10 m, direction, fetch and swell trains. `Spectrum::build` prepares a fixed set
of identities: 192 nonperiodic long components and three 256-square bands.
`LongWaves` answers above a world point by inverting horizontal displacement;
`Cascade` offers both direct sums and an inverse transform at texel centres.
`SeaProvider` and `compare` measure per-axis errors over points and ticks.

Cascade modes use `loop_cycles` and `cascade_frequency`: the nearest whole number
of turns in `LOOP_SECONDS` (1,024 s), within half a cycle per loop of deep-water
dispersion. Direct sums and transforms use that same frequency and return the
time derivative of displacement in `Differential::velocity`. Long components
retain the exact deep-water relation and do not repeat at the cascade loop.

The height spectrum spans k=1e-5 to the finest patch's radial Nyquist. Long waves
own k<2π/20; successive cascades own half-open bands up to each patch's Nyquist.
The lengths are 160, 37√1.001 and 9√1.003 m. Shorter waves contribute a constant
slope variance, integrated down to a declared 1 cm cutoff; a gravity-only model
cannot extend slope variance to infinite frequency. The lattice omits Nyquist
rows to avoid ambiguous real-transform derivatives. Each band's lattice powers
are normalized to its continuous integral, conserving variance without counting
a wave twice. This is an approximation of spectral shape within lattice cells.

Wind uses fetch-limited JONSWAP and its fully developed Pierson–Moskowitz limit.
Frequency-dependent, normalized Fourier directional spreading is nonnegative;
swell uses a log-Gaussian radial train (sigma .08, truncated at eight sigma) and
normalized squared-cosine spreading. Midpoint quadrature runs in log k: 512
subintervals per long radial cell, 8192 per cascade. The test estimates error by
halving these counts and comparing with independent global 32768/65536 grids.

Forcing changes preserve every vector and phase. `Transition` interpolates
**power**, with `P(t)=target+(start-target)*exp(-elapsed/tau)`. Snapshot the old
transition at the change time and construct the next response from that snapshot.
Its response time is an explicit positive input. No preset or game table exists.

`LongWaves::new` refuses sums with `q*sum(a*k)>=1`: the fixed-point inverse is
then not guaranteed to converge. Contractive inputs are iterated to 1e-11 m,
with a 4096-iteration limit and an explicit error on exhaustion. Velocity follows
a material label; the height derivative holds the world point fixed, so the
advection term distinguishes them. `Differential::slope` is parametric, while
`Sample::slope` is the world gradient after inversion. Long-wave phase is wrapped
in f64 at a local anchor before local offsets are added, also exposing the phase
needed by the later GPU implementation.

Preparation is O(192*512 + 3*N²) work and O(3*N²) storage, excluding quadrature
counts; it is not per point. A point query is O(components*iterations), with an
allocated phase scratch vector. The scalar reference transform is O(11*N² log N),
allocating an eleven-channel complex grid and a column. GPU and frame-time evidence
is in [SEA M2](../../docs/evidence/SEA/M2.md). [Provenance](../../docs/evidence/SEA/PROVENANCE.md)
records the published sources and confirms no Broadsides code was lifted.
