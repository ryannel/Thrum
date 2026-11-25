#!/usr/bin/env python3
"""Run the offline finite-disk shadow diagnostic on the version 1 reconstruction."""

from collections import Counter
from dataclasses import dataclass
from functools import lru_cache
from math import cos, inf, nextafter, pi, radians, sin, tan


RADIUS = tan(radians(0.266))
PHASES = (0.001, 0.002, 0.003)
PATCH = (-2.0, 0.0, 2.0)
PARTITION_X = min(PATCH) - 64.0
COMMON_GRID = (96, 128)
REFERENCE_GRIDS = (COMMON_GRID, (192, 256), (384, 512))
CASE_CAP = 20_000_000
CAP_CONTROL = 2_000

Point = tuple[float, float, float]
Triangle = tuple[Point, Point, Point]
Bounds = tuple[tuple[float, float], ...]


class CapReached(Exception):
    """The next operation would exceed the preset work cap."""


class Work:
    def __init__(self, cap=None):
        self.cap = cap
        self.spent = 0
        self.counts = Counter()

    def charge(self, kind):
        if self.spent == self.cap:
            raise CapReached
        self.spent += 1
        self.counts[kind] += 1


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def quad(a, b, c, d):
    return ((a, b, c), (a, c, d))


def ridge(distance, height, half_width):
    faces = ()
    profile = ((1 - distance, -2), (-distance, height), (-distance - 1, -2))
    for (x, y), (next_x, next_y) in zip(profile, profile[1:]):
        faces += quad((x, y, -half_width), (next_x, next_y, -half_width),
                      (next_x, next_y, half_width), (x, y, half_width))
    return faces


def cube():
    extents = ((-160.12, -159.88), (0.20, 0.44), (0.23, 0.47))
    faces = ()
    for axis, limits in enumerate(extents):
        other_axes = [index for index in range(3) if index != axis]
        for limit in limits:
            corners = []
            for signs in ((0, 0), (0, 1), (1, 1), (1, 0)):
                corner = [0.0] * 3
                corner[axis] = limit
                for other, sign in zip(other_axes, signs):
                    corner[other] = extents[other][sign]
                corners.append(tuple(corner))
            faces += quad(*corners)
    return faces


@dataclass(frozen=True)
class Query:
    name: str
    receiver: Point
    phase: float


CENTRE = Query("patch", (0.0, 0.0, 0.0), PHASES[1])


@dataclass(frozen=True)
class Case:
    name: str
    terrain: tuple[Triangle, ...]
    caster: tuple[Triangle, ...]
    queries: tuple[Query, ...]


def patch_queries(heights):
    return tuple(Query("patch", (x, height, z), phase)
                 for height in heights for x in PATCH for z in PATCH for phase in PHASES)


def fixtures():
    patch = patch_queries((0.0,))
    moving = tuple(
        query
        for index, z in enumerate((-1.5, -0.5, 0.5, 1.5), 1)
        for query in (Query(f"move-{index}-start", (0.0, 0.0, z), PHASES[0]),
                      Query(f"move-{index}-end", (0.0, 0.0, z + 0.5), PHASES[1]))
    )
    broad = ridge(256, 0.512, 8)
    close = ridge(66.5, 1.0, 8)
    caster = cube()
    straddling = tuple(face for face in close
                       if min(p[0] for p in face) < PARTITION_X < max(p[0] for p in face))
    assert len(straddling) == 2
    return (
        Case("open", (), (), patch),
        Case("broad-edge", broad, (), patch),
        Case("narrow-crest", ridge(1024, 3.072, 1), (), patch + moving),
        Case("crest-ablation", (), (), patch + moving),
        Case("close-umbra", close, (), patch),
        Case("straddling-only", straddling, (), (CENTRE,)),
        Case("compact-caster", (), caster, patch),
        Case("overlap", broad, caster, patch_queries((0.0, 0.15))),
    )


@lru_cache
def disk_rows(grid):
    rows, columns = grid
    samples = []
    for row in range(rows):
        theta = pi * (row + 0.5) / rows
        altitude = -RADIUS * cos(theta)
        half_chord = RADIUS * sin(theta)
        # d(altitude)/d(theta) times chord width is proportional to sin(theta)^2.
        sides = tuple(half_chord * (2 * (column + 0.5) / columns - 1)
                      for column in range(columns))
        samples.append((altitude, half_chord, sin(theta) ** 2, sides))
    return tuple(samples)


# The reference shoots two-sided rays directly at the original world-space triangles.
def ray_triangle(direction, triangle):
    edge1, edge2, offset, q, distance_numerator = triangle
    normal = cross(direction, edge2)
    det = dot(edge1, normal)
    if abs(det) < 1e-14:
        return False
    u = dot(offset, normal) / det
    v = dot(direction, q) / det
    return u >= 0 and v >= 0 and u + v <= 1 and distance_numerator / det > 0


def ray_mask(query, faces, grid, work):
    triangles = []
    for a, b, c in faces:
        work.charge("triangle")
        edge1, edge2 = sub(b, a), sub(c, a)
        offset = sub(query.receiver, a)
        q = cross(offset, edge1)
        triangles.append((edge1, edge2, offset, q, dot(edge2, q)))
    elevation = query.phase
    cosine, sine = cos(elevation), sin(elevation)
    mask = []
    for altitude, _, _, sides in disk_rows(grid):
        for side in sides:
            work.charge("sample")
            direction = (-cosine + altitude * sine, sine + altitude * cosine, side)
            for triangle in triangles:
                work.charge("triangle")
                if ray_triangle(direction, triangle):
                    mask.append(True)
                    break
            else:
                mask.append(False)
    return mask


# The candidate selects terrain once for the full receiver box and phase interval.
def bounds(points):
    return tuple((min(axis), max(axis)) for axis in zip(*points))


@dataclass(frozen=True)
class Node:
    bounds: Bounds
    face: Triangle | None = None
    children: tuple["Node", ...] = ()


def source_tree(leaves, work):
    if len(leaves) <= 1:
        return leaves[0] if leaves else None
    middle = len(leaves) // 2
    children = (source_tree(leaves[:middle], work), source_tree(leaves[middle:], work))
    work.charge("bound_prepare")
    corners = [tuple(axis[side] for axis in child.bounds)
               for child in children for side in (0, 1)]
    return Node(bounds(corners), children=children)


def swept_ray_slopes(phases):
    low, high = phases
    # Y / -X is tan(elevation + atan(altitude)); use both disk extrema.
    low_slope = tan(low - radians(0.266))
    high_slope = tan(high + radians(0.266))
    # For this nonnegative phase interval, the minimum forward X is at the
    # largest elevation and highest disk altitude. This encloses the full disk.
    max_side_slope = RADIUS / (cos(high) - RADIUS * sin(high))
    return ((low_slope, high_slope), (-max_side_slope, max_side_slope))


def admitted_faces(node, receivers, slopes, work):
    if node is None:
        return
    work.charge("bound_test")
    x, y, z = node.bounds
    depth = (receivers[0][0] - x[1], receivers[0][1] - x[0])
    # An undecided bound stays admitted. All reconstructed far nodes lie ahead
    # of the patch, but a bound crossing the receiver plane can be undecided.
    if depth[0] > 0:
        for source, receiver, slope in zip((y, z), receivers[1:], slopes):
            offsets = [distance * rate for distance in depth for rate in slope]
            low = nextafter(receiver[0] + min(offsets), -inf)
            high = nextafter(receiver[1] + max(offsets), inf)
            if source[1] < low or source[0] > high:
                return
    if node.face is not None:
        work.charge("triangle")
        yield node.face
    for child in node.children:
        yield from admitted_faces(child, receivers, slopes, work)


def prepare(case, work):
    far, near = [], []
    for face in case.terrain:
        work.charge("triangle")
        if all(p[0] <= PARTITION_X for p in face):
            work.charge("bound_prepare")
            far.append(Node(bounds(face), face=face))
        else:
            near.append(face)
    root = source_tree(far, work)
    work.charge("bound_prepare")
    heights = [query.receiver[1] for query in case.queries]
    receivers = (PATCH[::2], (min(heights), max(heights)), PATCH[::2])
    work.charge("bound_prepare")
    phases = (min(query.phase for query in case.queries), max(query.phase for query in case.queries))
    reusable = tuple(admitted_faces(root, receivers, swept_ray_slopes(phases), work))
    return reusable, tuple(near), (len(far), len(reusable), len(near), len(case.caster))


def project(query, faces, work):
    cosine, sine = cos(query.phase), sin(query.phase)
    projected = []
    for face in faces:
        work.charge("triangle")
        plane = []
        for vertex in face:
            x, y, z = sub(vertex, query.receiver)
            depth = -x * cosine + y * sine
            assert depth > 0, "reconstructed source must be ahead of the receiver"
            plane.append(((x * sine + y * cosine) / depth, z / depth))
        projected.append((*plane, plane[0]))
    return projected


def chord_intervals(planes, altitude, half_chord, work):
    sorted_intervals = []
    for plane in planes:
        work.charge("chord")
        crossings = []
        for (u0, v0), (u1, v1) in zip(plane, plane[1:]):
            if min(u0, u1) <= altitude <= max(u0, u1):
                if u0 == u1:
                    crossings.extend((v0, v1))
                else:
                    crossings.append(v0 + (v1 - v0) * (altitude - u0) / (u1 - u0))
        if not crossings:
            continue
        work.charge("interval_compare")
        low = max(-half_chord, min(crossings))
        work.charge("interval_compare")
        high = min(half_chord, max(crossings))
        work.charge("interval_compare")
        if low > high:
            continue
        # Insertion keeps each sorting comparison observable by the budget.
        index = len(sorted_intervals)
        for previous_low, _ in reversed(sorted_intervals):
            work.charge("interval_compare")
            if low >= previous_low:
                break
            index -= 1
        sorted_intervals.insert(index, (low, high))

    union = []
    for low, high in sorted_intervals:
        if union:
            work.charge("interval_compare")
            if low <= union[-1][1]:
                work.charge("interval_merge")
                union[-1] = (union[-1][0], max(union[-1][1], high))
                continue
        union.append((low, high))
    return union


def chord_mask(query, faces, work):
    planes = project(query, faces, work)
    mask = []
    integral, normalization = 0.0, 0.0
    for altitude, half_chord, weight, sides in disk_rows(COMMON_GRID):
        intervals = chord_intervals(planes, altitude, half_chord, work)
        for low, high in intervals:
            work.charge("sample")
            integral += weight * (high - low) / (2 * half_chord)
        normalization += weight
        for side in sides:
            work.charge("sample")
            for low, high in intervals:
                work.charge("interval_compare")
                if low <= side <= high:
                    mask.append(True)
                    break
            else:
                mask.append(False)
    return mask, integral / normalization


def candidate_run(case, cap):
    work = Work(cap)
    results = {}
    source_counts = None
    prep_spent = 0
    completed_spent = 0
    try:
        reusable, near, source_counts = prepare(case, work)
        prep_spent = work.spent
        faces = reusable + near + case.caster
        for query in case.queries:
            results[query] = chord_mask(query, faces, work)
            completed_spent = work.spent - prep_spent
    except CapReached:
        pass
    if source_counts is None:
        prep_spent = work.spent
    return results, work, source_counts, (prep_spent, completed_spent,
                                         work.spent - prep_spent - completed_spent)


def weighted_fraction(mask, grid):
    weighted = 0.0
    columns = grid[1]
    rows = disk_rows(grid)
    for row, (_, _, weight, _) in enumerate(rows):
        weighted += weight * sum(mask[row * columns:(row + 1) * columns]) / columns
    return weighted / sum(weight for _, _, weight, _ in rows)


def print_work(case, cap, answers, work, source_counts, spending, reference):
    print("work", case.name, cap, len(case.queries), len(answers), len(case.queries) - len(answers),
          *spending, work.spent, reference or "-", source_counts,
          dict(sorted(work.counts.items())), sep="\t")


def compare_case(case):
    answers, work, source_counts, spending = candidate_run(case, CASE_CAP)
    reference = Work()
    centre_masks = {}
    movement_hits = {}
    errors = 0
    faces = case.terrain + case.caster
    for query in case.queries:
        rays = ray_mask(query, faces, COMMON_GRID, reference)
        if query == CENTRE:
            centre_masks[COMMON_GRID] = rays
        reference_coverage = weighted_fraction(rays, COMMON_GRID)
        if query.name.startswith("move-"):
            movement_hits[query.name] = sum(rays)
        if query not in answers:
            print("query", case.name, query.name, *query.receiver, query.phase,
                  sum(rays), f"{reference_coverage:.8f}", "unanswered", sep="\t")
            continue
        candidate, integrated = answers[query]
        omission = sum(ray and not hit for ray, hit in zip(rays, candidate, strict=True))
        overcoverage = sum(hit and not ray for ray, hit in zip(rays, candidate, strict=True))
        errors += omission + overcoverage
        print("query", case.name, query.name, *query.receiver, query.phase, sum(rays),
              f"{reference_coverage:.8f}", sum(candidate),
              f"{weighted_fraction(candidate, COMMON_GRID):.8f}", f"{integrated:.8f}",
              omission, overcoverage, sep="\t")
    print_work(case, CASE_CAP, answers, work, source_counts, spending, reference.spent)
    for grid in REFERENCE_GRIDS:
        if grid not in centre_masks:
            centre_masks[grid] = ray_mask(CENTRE, faces, grid, Work())
        rays = centre_masks[grid]
        print("refinement", case.name, *grid, sum(rays), f"{weighted_fraction(rays, grid):.8f}", sep="\t")
    retain = len(answers) == len(case.queries) and errors == 0 and (
        not faces or work.spent < reference.spent)
    return retain, centre_masks, movement_hits


def main():
    print(f"reconstruction-v1\tradius={RADIUS}\tpartition_x={PARTITION_X}\tcommon_grid={COMMON_GRID}")
    print("query\tcase\tquery\tx\ty\tz\tphase\tray_hits\tray_fraction\tchord_hits\tchord_sample_fraction"
          "\tchord_integral_fraction\tomission_hits\tovercoverage_hits")
    print("work\tcase\tcap\tqueries\tanswered\tunanswered\tprep\tcompleted_query_work\tunfinished_query_work"
          "\ttotal\treference_common_total\tfaces(far,admitted_far,near,caster)\toperations")
    print("refinement\tcase\trows\tcolumns\tray_hits\tray_fraction")

    # Fixture geometry and disk sample positions are shared inputs to both arms.
    cases = fixtures()
    for grid in REFERENCE_GRIDS:
        disk_rows(grid)
    results = {case.name: compare_case(case) for case in cases}
    centre = {name: result[1] for name, result in results.items()}
    for grid in REFERENCE_GRIDS:
        open_sun, broad, crest, close, straddling, caster, overlap = (
            centre[name][grid] for name in
            ("open", "broad-edge", "narrow-crest", "close-umbra", "straddling-only",
             "compact-caster", "overlap"))
        assert not any(open_sun) and 0 < sum(crest) < sum(broad) < len(broad)
        assert all(straddling) and close == straddling
        union = [terrain or cube for terrain, cube in zip(broad, caster, strict=True)]
        intersection = [terrain and cube for terrain, cube in zip(broad, caster, strict=True)]
        assert 0 < sum(intersection) < sum(caster)
        assert union == overlap and sum(overlap) == sum(broad) + sum(caster) - sum(intersection)
        print("centre-control", *grid, "straddling_hits", sum(straddling), "overlap_intersection",
              sum(intersection), "overlap_union", sum(union), sep="\t")

    moves = results["narrow-crest"][2]
    ablation = results["crest-ablation"][2]
    for index in range(1, 5):
        start, end = f"move-{index}-start", f"move-{index}-end"
        assert moves[start] != moves[end] and ablation[start] == ablation[end] == 0
        print("movement-control", index, moves[start], moves[end], ablation[start], ablation[end], sep="\t")

    cap_case = cases[-1]
    answers, work, sources, spending = candidate_run(cap_case, CAP_CONTROL)
    assert not answers and work.spent == CAP_CONTROL and spending[0] > 0 and spending[2] > 0
    assert sum(spending) == sum(work.counts.values()) == work.spent
    print_work(cap_case, CAP_CONTROL, answers, work, sources, spending, None)
    print("verdict", "retain" if all(result[0] for result in results.values()) else "park", sep="\t")


if __name__ == "__main__":
    main()
