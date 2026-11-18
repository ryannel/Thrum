#!/usr/bin/env python3
"""Prove the cooked mesh-stage path through Metal submissions in the proving ground."""

import argparse
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import tempfile

from cooked_draw_parity import png

ROOT = Path(__file__).resolve().parents[1]
SIDE = 128
VALLEY_CASES = ("valley-cull-equal", "shadow-equal", "shadow-parity")


def frame(output, name, attachment):
    data = (output / f"{name}-{attachment}.raw").read_bytes()
    if len(data) != SIDE * SIDE * 8:
        raise AssertionError(f"{name} {attachment} has wrong extent")
    return list(struct.iter_unpack("<eeee", data))


def half_step(value):
    if value == 0:
        return 2 ** -24
    _, exponent = math.frexp(abs(value))
    return max(2 ** (exponent - 11), 2 ** -24)


def compare(output, case, pairs):
    for label, indexed, mesh in pairs:
        colour_a = frame(output, f"{case}-{indexed}", "colour")
        colour_b = frame(output, f"{case}-{mesh}", "colour")
        temporal_a = frame(output, f"{case}-{indexed}", "temporal")
        temporal_b = frame(output, f"{case}-{mesh}", "temporal")
        fields = {
            "colour": (colour_a, colour_b, (0, 1, 2, 3)),
            "depth": (temporal_a, temporal_b, (1,)),
            "motion": (temporal_a, temporal_b, (2, 3)),
            "reactive": (temporal_a, temporal_b, (0,)),
        }
        geometry = sum(pixel[1] > 0 for pixel in temporal_a)
        visible_motion = sum(pixel[1] > 0 and 0 < max(abs(pixel[2]), abs(pixel[3])) < 65504
                             for pixel in temporal_a)
        if geometry < 100 or (label == "history" and visible_motion < 100):
            raise AssertionError(f"{label}: insufficient geometry or motion {geometry}/{visible_motion}")
        if label in ("warm", "cut"):
            invalid = sum(pixel[1] > 0 and pixel[0] == 1 and pixel[2:] == (65504, 65504)
                          for pixel in temporal_a)
            if invalid != geometry:
                raise AssertionError(f"{label}: invalid history on {invalid}/{geometry} geometry pixels")
        failures = {}
        maxima = {}
        for field, (reference, candidate, channels) in fields.items():
            rgba = bytearray()
            failures[field] = 0
            maxima[field] = 0
            for a, b in zip(reference, candidate, strict=True):
                errors = [abs(a[c] - b[c]) for c in channels]
                maximum = max(errors)
                maxima[field] = max(maxima[field], maximum)
                # Colour admits two half-float rounding steps through the independently compiled
                # vertex and interpolation programs. Reverse-Z is observed by the reconstruction
                # witness at half precision. Motion is pixel-space, with an absolute budget of
                # 1/4096 pixel plus two half rounding steps. Reactive and invalid
                # motion sentinels are discrete.
                tolerances = []
                for c in channels:
                    steps = 1 if field == "depth" else 2
                    tolerance = steps * half_step(max(abs(a[c]), abs(b[c])))
                    if field == "motion":
                        tolerance = 0 if max(abs(a[c]), abs(b[c])) >= 65504 else tolerance + 1 / 4096
                    if field == "reactive":
                        tolerance = 0
                    tolerances.append(tolerance)
                fail = any(not math.isfinite(error) or error > tolerance
                           for error, tolerance in zip(errors, tolerances, strict=True))
                failures[field] += fail
                scale = 200 if field == "colour" else 6000
                signal = min(255, math.ceil(maximum * scale)) if math.isfinite(maximum) else 255
                rgba.extend((255, 0, 0, 255) if fail else (signal, signal, signal, 255))
            png(output / f"{case}-{label}-{field}-difference.png", rgba, SIDE, SIDE)
        print(json.dumps(dict(label=label, geometry_pixels=geometry, valid_motion_pixels=visible_motion,
                              exclusions=0, maxima=maxima, failures=failures)))
        if any(failures.values()):
            raise AssertionError(f"{case} {label}: samples outside tolerance {failures}")


def scene_witnesses(output):
    shadowed = frame(output, "parity-warm-indexed", "colour")
    clear = frame(output, "parity-no-shadow", "colour")
    depth = frame(output, "parity-warm-indexed", "temporal")
    clear_depth = frame(output, "parity-no-shadow", "temporal")
    shadow = sum(z[1] > 0 and z[1] == z_clear[1] and max(b[c] - a[c] for c in range(3)) > 0.025
                 for a, b, z, z_clear in zip(shadowed, clear, depth, clear_depth, strict=True))
    geometry = sum(z[1] > 0 for z in depth)
    if min(shadow, geometry - shadow) < 12:
        raise AssertionError(f"Sun attenuation must leave lit and shadowed geometry, got {shadow}/{geometry}")
    checker = sum(z[1] > 0 and colour[0] - colour[2] > 0.5
                  for colour, z in zip(shadowed, depth, strict=True))
    if min(checker, geometry - checker) < 100:
        raise AssertionError(f"exercises both checker tones on geometry, got {checker}/{geometry}")
    print(json.dumps(dict(receiving_shadow_pixels=shadow, warm_checker_pixels=checker, geometry_pixels=geometry)))


def material_witnesses(output):
    original = frame(output, "material-original", "colour")
    accepted = frame(output, "material-accepted", "colour")
    refused = frame(output, "material-refused", "colour")
    next_frame = frame(output, "material-after-discard", "colour")
    depth = frame(output, "material-original", "temporal")
    if accepted != refused:
        raise AssertionError("refused material edit changed the previous material's output")
    for label, left, right in [("mesh stages", 0, SIDE // 2), ("indexed fallback", SIDE // 2, SIDE)]:
        indices = [y * SIDE + x for y in range(SIDE) for x in range(left, right)
                   if depth[y * SIDE + x][1] > 0]
        first = [i for i in indices if accepted[i][0] - original[i][0] > 0.025]
        second = [i for i in indices if next_frame[i][2] - accepted[i][2] > 0.025
                  and accepted[i][0] - next_frame[i][0] > 0.025]
        if len(indices) < 100 or any(len(changed) < 0.95 * len(indices) for changed in (first, second)):
            raise AssertionError(f"{label}: incomplete mixed material installation {len(first)}/{len(second)}/{len(indices)}")
        print(json.dumps(dict(path=label, pixels=len(indices), accepted_edit_pixels=len(first),
                              installed_after_discard_pixels=len(second))))


def cook_valley(store):
    subprocess.run([
        "cargo", "run", "--locked", "--offline", "-p", "thrum-cook", "--",
        "cook", "examples/v1-game/assets/showcase", str(store),
    ], cwd=ROOT, check=True)


def shadow_parity(output):
    for hour in (12, 17.3):
        images = [(output / f"shadow-{hour}-{path}.raw").read_bytes()
                  for path in ("indexed", "stages")]
        for layer, label in enumerate(("near", "far", "height")):
            start = layer * 1024 ** 2 * 4
            end = start + 1024 ** 2 * 4
            indexed, stages = [list(struct.iter_unpack("<I", image[start:end])) for image in images]
            coverage = sum(a != (0,) for a in indexed)
            coverage_changes = sum((a == (0,)) != (b == (0,))
                                   for a, b in zip(indexed, stages, strict=True))
            # Independently compiled vertex and mesh stages evaluate the same f32
            # clip-position expression. Admit four final Depth32Float rounding steps.
            deltas = [abs(a[0] - b[0]) for a, b in zip(indexed, stages, strict=True)
                      if a != (0,) and b != (0,)]
            failures = sum(delta > 4 for delta in deltas)
            print(json.dumps(dict(hour=hour, map=label, coverage=coverage,
                                  coverage_changes=coverage_changes, maximum_ulp=max(deltas, default=0),
                                  tolerance_ulp=4, failures=failures, edge_exclusions=0)))
            if not coverage or coverage_changes or failures:
                raise AssertionError(f"{hour} h {label} shadow map differs from the indexed caster")


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("case", choices=("selection", "prepared", "eligibility", "residency",
                                         "cull-equal", "parity", "material", "edit", "lifecycle",
                                         "shadows-off", *VALLEY_CASES))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if os.uname().sysname != "Darwin":
        parser.error("MESH-SHADING M1 proofs require macOS Metal")
    valley = args.case in VALLEY_CASES
    milestone = "M2" if valley or args.case in ("edit", "lifecycle", "shadows-off") else "M1"
    scratch = ROOT / f"tmp/mesh-shading-{milestone}"
    scratch.mkdir(parents=True, exist_ok=True)
    output = args.output.resolve() if args.output else Path(tempfile.mkdtemp(prefix=f"{args.case}-", dir=scratch))
    output.mkdir(parents=True, exist_ok=True)
    print(f"proof output {output}", flush=True)
    print(subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), flush=True)
    env = {**os.environ, "THRUM_REQUIRE_GPU": "1", "THRUM_MESH_STAGE_PROOF_DIR": str(output)}
    if valley:
        store = output / "store"
        cook_valley(store)
        env.update(THRUM_SCENE="valley", THRUM_STARS="off", THRUM_MESH_STAGE_VALLEY_STORE=str(store))
    case = "publication_edit" if args.case == "edit" else args.case.replace("-", "_")
    subprocess.run([
        "cargo", "test", "--locked", "--offline", "-p", "v1-game", "--features", "draw-probe",
        "--test", "mesh_stage_gpu", ("valley::" if valley else "metal::") + case,
        "--", "--exact", "--nocapture", *(["--ignored"] if valley else []),
    ], cwd=ROOT, env=env, check=True)
    if args.case == "shadow-parity":
        shadow_parity(output)
    if args.case == "parity":
        compare(output, "parity", [(phase, f"{phase}-indexed", f"{phase}-mesh") for phase in ("warm", "history", "cut")])
        scene_witnesses(output)
    if args.case == "eligibility":
        compare(output, "eligibility", [(limit, f"{limit}-indexed", f"{limit}-mixed")
                for limit in ("max_storage_buffer_binding_size", "max_task_workgroup_total_count")])
    if args.case == "material":
        material_witnesses(output)


if __name__ == "__main__":
    main()
