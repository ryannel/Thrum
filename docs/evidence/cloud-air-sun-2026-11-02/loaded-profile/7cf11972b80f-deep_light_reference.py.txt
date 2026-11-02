"""Two bounded K48 ordinary-tau references, independent of source integration.

The two compared functions are exp(-tau) and exp(-.25*tau), not a fitted tau
that stands for filtered light. Input is an ordered stream of finite (depth_m,
nonnegative cumulative_tau) pairs. No implicit float32 conversion is performed:
use float32_samples when pinning CPU/GPU identical endpoints. The bound concerns
that supplied piecewise-linear tau curve only; quadrature, float32 storage,
lateral footprints and the GPU's arithmetic require separate qualification.

Min-removal state is <=K+1 knots plus scalar counters/bounds; tau-tube holds
<=K anchors plus the previous/current endpoint. The analytic triangle/disjoint-
piece arguments are exact in real arithmetic. libm extrema and the reported floating guard are a
CPU reference implementation, not an interval proof or uniform GPU certificate.
Invalid results are diagnostic curves, never admitted lighting or clear fallback.
"""
from bisect import bisect_right
from dataclasses import dataclass
import math
import struct
import sys

EXPONENTS = (1.0, .25)
MAX_KNOTS = 48
MAX_SAMPLES = 2048
COMPRESSION_ALLOCATION = .005
NUMERICAL_ALLOWANCE = 2e-6
NUMERICAL_GUARD_PER_REMOVAL = NUMERICAL_ALLOWANCE


@dataclass(frozen=True)
class LocalError:
    analytic: tuple[float, float]
    numeric_guard: tuple[float, float]
    locations: int
    extrema: int


@dataclass(frozen=True)
class CurveResult:
    knots: tuple[tuple[float, float], ...]
    accumulated_analytic: tuple[float, float]
    accumulated_numeric_guard: tuple[float, float]
    bounds: tuple[float, float]
    valid: bool
    reason: str
    allocation: float
    work: dict
    method: str = 'min_removal'

    @property
    def analytic_bounds(self):
        """Triangle sums for min-removal; disjoint-piece maxima for tau-tube."""
        return self.accumulated_analytic


def float32_samples(samples):
    """Explicit endpoint conversion; duplicate rounded depths fail compression.

    Conversion error is NOT covered by the compression bound. This helper lets
    a caller retain/replay exactly the float32 endpoints used by a GPU helper.
    """
    for depth, tau in samples:
        yield tuple(struct.unpack('<f', struct.pack('<f', float(x)))[0]
                    for x in (depth, tau))


def _sample(value):
    if len(value) != 2:
        raise ValueError('sample must be a depth/tau pair')
    depth, tau = map(float, value)
    if not math.isfinite(depth) or not math.isfinite(tau) or tau < 0:
        raise ValueError('finite depth and nonnegative finite tau required')
    return depth, tau


def _difference(k, a, b):
    # Absolute exp difference without cancellation in nearly transparent curves.
    return math.exp(-k * min(a, b)) * -math.expm1(-k * abs(a - b))


def local_removal_error(a, b, c):
    """Sup error of old A-B-C vs straight A-C, for both active transforms.

    On each half write old tau=a0+d*u, new tau=b0+e*u, u in [0,1].
    Derivative equality is log(d/e)=k*((a0-b0)+(d-e)*u). Equal or
    zero slopes need no division/root: extrema are endpoints. A/C errors are
    exactly zero, B is shared, so <=3 locations per transform, <=6 total.
    """
    a, b, c = _sample(a), _sample(b), _sample(c)
    if not a[0] < b[0] < c[0] or not a[1] <= b[1] <= c[1]:
        raise ValueError('strict ordered depths and monotone tau required')
    span = c[0] - a[0]
    if not math.isfinite(span):
        raise ValueError('finite depth span required')
    fraction = (b[0] - a[0]) / span
    if not 0 < fraction < 1:
        raise ValueError('unresolved normalized depth split')
    mid = a[1] + (c[1] - a[1]) * fraction
    errors, guards = [], []
    locations = extrema = 0
    for k in EXPONENTS:
        error = _difference(k, b[1], mid)
        locations += 1
        for old0, old1, new0, new1 in ((a[1], b[1], a[1], mid),
                                      (b[1], c[1], mid, c[1])):
            d, e = old1 - old0, new1 - new0
            if d == e or d == 0 or e == 0:
                continue
            delta = d - e
            # log1p keeps nearby positive slopes from cancelling in log(d)-log(e).
            log_ratio = (math.log1p(delta / e) if abs(delta) < .5 * e
                         else math.log(d) - math.log(e))
            u = (log_ratio / k - (old0 - new0)) / delta
            if 0 < u < 1:
                error = max(error, _difference(k, old0 + d * u, new0 + e * u))
                locations += 1
                extrema += 1
        errors.append(error)
        # Stated engineering guard, deliberately separate from analytic estimates.
        # Capping the optical scale is justified only as an underflow-scale guard:
        # exp(-745) is already subnormal. This is NOT a proven libm/backend bound.
        guard = (0.0 if a[1] == c[1] else
                 64 * sys.float_info.epsilon * (1 + min(k * c[1], 745)))
        guards.append(guard)
    return LocalError(tuple(errors), tuple(guards), locations, extrema)


def _add_up(previous, increment):
    return (math.nextafter(previous + increment, math.inf)
            if increment else previous)


def compress_tau_samples(samples, *, max_knots=MAX_KNOTS,
                         allocation=COMPRESSION_ALLOCATION,
                         max_samples=MAX_SAMPLES):
    """Streaming append/minimum-error interior removal with visible fallback.

    K48 is the selected candidate; smaller capacities are diagnostics. Ties use
    the earliest interior knot. Every deletion adds its local sup error to the
    previous global bound, including after later appends. Endpoints never leave.
    A selected 2e-6 per-removal numerical allowance is included separately from
    the analytic sum. This is an unqualified backend guard, not a proof.
    Input/schema/capacity failures return valid=False; no invalid curve may be
    used as a blocker-free estimate. A quality failure still consumes the bounded
    input stream for inspection; malformed/over-cap input stops at that failure.
    """
    if (not isinstance(max_knots, int) or isinstance(max_knots, bool)
            or not 2 <= max_knots <= MAX_KNOTS):
        raise ValueError('capacity must be an integer in [2,48]')
    if (not isinstance(max_samples, int) or isinstance(max_samples, bool)
            or not 2 <= max_samples <= MAX_SAMPLES):
        raise ValueError('endpoint cap must be an integer in [2,2048]')
    if not math.isfinite(allocation) or not 0 < allocation <= COMPRESSION_ALLOCATION:
        raise ValueError('allocation must be positive and <=.005')
    knots = []
    analytic, guard = [0.0, 0.0], [0.0, 0.0]
    work = dict(input_samples=0, appended_samples=0, removals=0,
                candidate_evaluations=0, transformed_locations=0,
                interior_extrema=0, peak_knots=0)
    reason = ''
    previous = None
    try:
        for value in samples:
            work['input_samples'] += 1
            if work['input_samples'] > max_samples:
                reason = 'endpoint_capacity_exceeded'
                break
            point = _sample(value)
            if previous is not None:
                if point[0] <= previous[0]:
                    raise ValueError('depths must increase strictly; duplicates unsupported')
                if point[1] < previous[1]:
                    raise ValueError('cumulative tau must be monotone')
                if not math.isfinite(point[0] - knots[0][0]):
                    raise ValueError('finite complete depth span required')
            previous = point
            knots.append(point)
            work['appended_samples'] += 1
            work['peak_knots'] = max(work['peak_knots'], len(knots))
            if len(knots) > max_knots:
                best = None
                for i in range(1, len(knots) - 1):
                    candidate = local_removal_error(knots[i-1], knots[i], knots[i+1])
                    work['candidate_evaluations'] += 1
                    work['transformed_locations'] += candidate.locations
                    work['interior_extrema'] += candidate.extrema
                    score = max(candidate.analytic)
                    if best is None or score < best[0]:
                        best = score, i, candidate
                _, i, selected = best
                for j in range(2):
                    analytic[j] = _add_up(analytic[j], selected.analytic[j])
                    guard[j] = _add_up(guard[j], max(NUMERICAL_GUARD_PER_REMOVAL, selected.numeric_guard[j]))
                del knots[i]
                work['removals'] += 1
    except (ValueError, TypeError, OverflowError) as error:
        reason = 'invalid_input: ' + str(error)
    bounds = tuple(_add_up(a, g) for a, g in zip(analytic, guard))
    if not reason and len(knots) < 2:
        reason = 'at_least_two_endpoints_required'
    if not reason and max(bounds) > allocation:
        reason = 'compression_allocation_exceeded'
    return CurveResult(tuple(knots), tuple(analytic), tuple(guard), bounds,
                       not reason, reason, allocation, work)


def interpolate_tau(knots, depth):
    """Piecewise-linear ordinary tau, only inside the supplied curve domain."""
    if (not math.isfinite(depth) or len(knots) < 2
            or depth < knots[0][0] or depth > knots[-1][0]):
        raise ValueError('query outside admitted depth domain')
    # No input samples or precomputed search arrays are retained by the compressor.
    i = max(0, min(len(knots) - 2, bisect_right(knots, (depth, math.inf)) - 1))
    a, b = knots[i:i+2]
    f = (depth - a[0]) / (b[0] - a[0])
    return a[1] + (b[1] - a[1]) * f


def curve_terms(result, depth):
    """Admitted point terms. An invalid result requires direct/refined fallback."""
    if not result.valid:
        raise ValueError('unavailable curve: ' + result.reason)
    tau = interpolate_tau(result.knots, depth)
    return tuple(math.exp(-k * tau) for k in EXPONENTS)


def tube_delta(anchor_tau, epsilon):
    """Ordinary-tau tube half-width sufficient for BOTH transformed errors.

    If both terms are already <=epsilon, any nonnegative downstream slope is
    safe and the width is infinite. This avoids exp(+k*tau) overflow entirely.
    """
    if (not math.isfinite(anchor_tau) or anchor_tau < 0
            or not math.isfinite(epsilon) or not 0 < epsilon < 1):
        raise ValueError('finite nonnegative anchor tau and epsilon in (0,1) required')
    delta = math.inf
    for k in EXPONENTS:
        anchor_term = math.exp(-k * anchor_tau)
        if anchor_term > epsilon:
            ratio = epsilon / anchor_term
            width = -math.log1p(-ratio) / k
            delta = min(delta, math.nextafter(width, 0.0))
    return delta


def compress_tau_tube(samples, *, max_knots=MAX_KNOTS,
                      allocation=COMPRESSION_ALLOCATION,
                      max_samples=MAX_SAMPLES):
    """O(N) actual-endpoint tube fit; separate candidate from min-removal.

    For one anchor keep intersected slope ranges for all intervening knots.
    Test a proposed endpoint's ACTUAL slope against the PREVIOUS intersection.
    If it fails, commit the immediately previous accepted input endpoint (whose
    line was already valid), reset at that anchor and reprocess the current input.
    No arbitrary feasible slope is emitted as an invented endpoint. Every input
    needs at most two visits. Nonoverlapping accepted pieces use MAX error, not
    the cumulative removal sum required by the other compressor.

    The nominal tube tolerance reserves 2e-6 for a separately reported numerical
    guard. This backend allowance remains unqualified; it is not a uniform GPU
    certificate. Capacity failure remains unavailable, never silent truncation.
    """
    if (not isinstance(max_knots, int) or isinstance(max_knots, bool)
            or not 2 <= max_knots <= MAX_KNOTS):
        raise ValueError('capacity must be an integer in [2,48]')
    if (not isinstance(max_samples, int) or isinstance(max_samples, bool)
            or not 2 <= max_samples <= MAX_SAMPLES):
        raise ValueError('endpoint cap must be an integer in [2,2048]')
    if not math.isfinite(allocation) or not NUMERICAL_ALLOWANCE < allocation <= COMPRESSION_ALLOCATION:
        raise ValueError('tube allocation must exceed numerical allowance and be <=.005')
    epsilon = allocation - NUMERICAL_ALLOWANCE
    knots, analytic = [], [0.0, 0.0]
    work = dict(input_samples=0, appended_samples=0, endpoint_visits=0,
                anchor_builds=0, emitted_pieces=0, peak_knots=0)
    anchor = previous = None
    lower, upper, delta = 0.0, math.inf, 0.0
    reason = ''

    def emit(endpoint):
        if len(knots) >= max_knots:
            raise ValueError('knot_capacity_exceeded')
        for j, k in enumerate(EXPONENTS):
            analytic[j] = max(analytic[j], min(epsilon, math.exp(-k*anchor[1])))
        knots.append(endpoint)
        work['emitted_pieces'] += 1
        work['peak_knots'] = max(work['peak_knots'], len(knots))

    try:
        for value in samples:
            work['input_samples'] += 1
            if work['input_samples'] > max_samples:
                reason = 'endpoint_capacity_exceeded'
                break
            point = _sample(value)
            if previous is None:
                anchor = previous = point
                knots.append(point)
                delta = tube_delta(anchor[1], epsilon)
                work['anchor_builds'] += 1
                work['appended_samples'] += 1
                work['peak_knots'] = 1
                continue
            if point[0] <= previous[0] or point[1] < previous[1]:
                raise ValueError('strict increasing depth and monotone tau required')
            if not math.isfinite(point[0] - knots[0][0]):
                raise ValueError('finite complete depth span required')
            for attempt in range(2):
                work['endpoint_visits'] += 1
                dx, dtau = point[0]-anchor[0], point[1]-anchor[1]
                slope = dtau/dx
                if not math.isfinite(slope):
                    raise ValueError('unresolved finite slope')
                if lower <= slope <= upper:
                    if not math.isinf(delta):
                        lower = max(lower, (dtau-delta)/dx)
                        upper = min(upper, (dtau+delta)/dx)
                    previous = point
                    work['appended_samples'] += 1
                    break
                if attempt or previous == anchor:
                    raise ValueError('unresolved tube intersection')
                emit(previous)
                anchor = previous
                lower, upper = 0.0, math.inf
                delta = tube_delta(anchor[1], epsilon)
                work['anchor_builds'] += 1
        if not reason and previous is not None and previous != anchor:
            emit(previous)
    except (ValueError, TypeError, OverflowError) as error:
        reason = ('knot_capacity_exceeded' if str(error) == 'knot_capacity_exceeded'
                  else 'invalid_input: ' + str(error))
    if not reason and len(knots) < 2:
        reason = 'at_least_two_endpoints_required'
    guard = ((NUMERICAL_ALLOWANCE,)*2 if work['emitted_pieces'] else (0.0, 0.0))
    bounds = tuple(a+g for a, g in zip(analytic, guard))
    if not reason and max(bounds) > allocation:
        reason = 'compression_allocation_exceeded'
    return CurveResult(tuple(knots), tuple(analytic), guard, bounds,
                       not reason, reason, allocation, work, 'tau_tube')
