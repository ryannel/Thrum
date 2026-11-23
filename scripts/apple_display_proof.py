"""Metal display records and pixel comparisons for Apple presentation."""
from array import array
import json
import math
import os
import struct
import sys

from apple_presentation import RECIPE, ROOT, mac_binary, read_rows, run


def window_arm(root, binary, env, name, force, *, frames=40, tap=False):
    arm = root / name
    arm.mkdir()
    launch = dict(env, THRUM_MAC_EDR=force, THRUM_V1_FRAMES=str(frames),
                  THRUM_MAC_EDR_PROOF=str(arm), THRUM_RECIPE="portable")
    if tap:
        launch["THRUM_MAC_EDR_TAP"] = "1"
    run("/usr/bin/caffeinate", "-d", "-i", "-u", str(binary), env=launch, timeout=150)
    display = read_rows(arm / "display.jsonl")
    rows = read_rows(arm / "frames.jsonl")
    layers = [row for row in display if "layer_headroom" in row]
    records = [row for row in display if "real_current" in row]
    presented = [row for row in rows if row.get("presented") and "tick" in row]
    if not presented or not records or rows[-1].get("violations") != 0:
        raise ValueError(f"{name}: missing frames, screen samples or creation counters")
    if not all(row["forced"] and row["real_potential"] >= 1.0 for row in records):
        raise ValueError(f"{name}: required real and forced records are missing")
    return dict(rows=rows, records=records, layers=layers, presented=presented,
                pipelines=rows[-1]["created_pipelines"], path=str(arm))


def all_format(arm, format_name, colour, configures):
    for row in arm["presented"]:
        if (row["format"], row["color_space"], row["configures"]) != (format_name, colour, configures):
            raise ValueError(f"expected {format_name}, {colour}, {configures} configure(s): {row}")


def mac_edr(output):
    binary, env = mac_binary(counters=True)
    arm = window_arm(output, binary, env, "launch", "2")
    all_format(arm, "Rgba16Float", "ExtendedSrgbLinear", 1)
    if not arm["layers"] or not all(
        row["main_thread"] and row["preferred_high"] and row["layer_headroom"] == 2
        for row in arm["layers"]
    ):
        raise ValueError("AppKit did not configure the retained layer for H=2")
    if any(row["headroom"] != 2 for row in arm["presented"]):
        raise ValueError("render owner did not resolve H=2")
    return arm


def mac_edr_modes(output):
    binary, env = mac_binary(counters=True)
    retained = window_arm(output, binary, env, "retained", "step:2:1:12:1", frames=40)
    all_format(retained, "Rgba16Float", "ExtendedSrgbLinear", 1)
    if {row["headroom"] for row in retained["presented"]} != {1, 2}:
        raise ValueError("retained EDR surface did not draw both H=2 and H=1")
    sdr = window_arm(output, binary, env, "unavailable", "sdr")
    if not all(row["format"] in ("Bgra8Unorm", "Bgra8UnormSrgb", "Rgba8Unorm", "Rgba8UnormSrgb")
               and row["configures"] == 1 and row["headroom"] == 1 for row in sdr["presented"]):
        raise ValueError("a session without EDR did not stay SDR")
    up = window_arm(output, binary, env, "promoted", "promote:12", frames=60)
    formats = [row["format"] for row in up["presented"]]
    if formats[0] != sdr["presented"][0]["format"] or formats[-1] != "Rgba16Float":
        raise ValueError("started SDR but did not end on the EDR generation")
    for row in up["presented"]:
        edr = row["format"] == "Rgba16Float"
        if row["configures"] != 1 + edr or (edr and row["color_space"] != "ExtendedSrgbLinear"):
            raise ValueError(f"display upgrade configuration was not atomic: {row}")
    if len({retained["pipelines"], sdr["pipelines"], up["pipelines"]}) != 1:
        raise ValueError("changing potential headroom created a pipeline")
    return dict(retained=retained, unavailable=sdr, promoted=up)


def mac_edr_headroom(output):
    binary, env = mac_binary(counters=True)
    step = 12
    arm = window_arm(output, binary, env, "headroom", f"step:8:1.3:{step}:16", frames=40, tap=True)
    control = window_arm(output, binary, env, "control", "8", frames=40, tap=True)
    all_format(arm, "Rgba16Float", "ExtendedSrgbLinear", 1)
    present = {row["submission"]: row for row in arm["presented"]}
    images = {row["capture_submission"]: row for row in arm["rows"] if "max_rgb" in row}
    if present.keys() != images.keys():
        raise ValueError("every presented float drawable must have a matching readback")
    if not any(row["tick"] < step and row["headroom"] == 8 for row in present.values()):
        raise ValueError("missing pre-change H=8 frame")
    first = next((row for row in present.values() if row["headroom"] < 8), None)
    if first is None or first["tick"] > step + 1:
        raise ValueError(f"headroom did not reach the drawable within one display tick: {first}")
    if any(not row["presented"] or row["max_rgb"] > present[order]["headroom"]
           for order, row in images.items()):
        raise ValueError("drawable pixels exceeded current headroom")
    baseline = control["rows"][-1]["creation_report"]
    captured = arm["rows"][-1]["creation_report"]
    # The report includes differing frame counts in its summary. The aggregate
    # pipeline and live object rows must describe the same preparation.
    if arm["pipelines"] != control["pipelines"] or captured.split("rows:", 1)[1] != baseline.split("rows:", 1)[1]:
        raise ValueError("the headroom-only change altered creation records")
    return arm


def stills(output, binary, env, recipe, source, headroom=None, *, lamps=False):
    from run_v1_showcase import PARAMS, ROUTE

    directory = output / f"{recipe}-{source}{'-lamps' if lamps else ''}"
    presets = ["night-lights"] if lamps else ROUTE
    options = dict(env, THRUM_RECIPE=recipe, THRUM_CAPTURE_SCENE=str(directory),
                   THRUM_CAPTURE_ROUTE_ONLY="1",
                   THRUM_CAPTURE_PARAMS=os.pathsep.join(str(PARAMS / f"showcase-{name}.params.json")
                                                       for name in presets))
    if lamps:
        # Use the game's existing LampStop scene and the camera from its local-light proof.
        options.update(THRUM_ROUTE_OVERLAY="lamp-stop", THRUM_CAPTURE_FIXED_EYE="[0.0, 242.4, 2.5]")
    if headroom is not None:
        options["THRUM_CAPTURE_EDR_HEADROOM"] = str(headroom)
    else:
        options["THRUM_CAPTURE_SOURCE"] = source
    run(str(binary), env=options, timeout=240)
    manifest = json.loads((directory / "stills.json").read_text())
    if manifest["recipe"] != RECIPE[recipe] or manifest["backend"] != "Metal":
        raise ValueError(f"incorrect fixed capture adapter or recipe: {manifest}")
    return directory, manifest


def route_environment(output):
    from run_v1_showcase import SOURCE, store_root
    binary, env = mac_binary(cook=True, counters=True)
    store = output / "store"
    run(str(ROOT / "target/release/thrum-cook"), "cook", str(SOURCE), str(store))
    return binary, dict(env, THRUM_SCENE="valley", THRUM_ASSET_STORE=str(store)), store_root(store)


def same_state(left, right):
    for key in ("name", "source_tick", "submission", "time_of_day", "pitch", "yaw"):
        if left[key] != right[key]:
            raise ValueError(f"stills disagree on {key}: {left} vs {right}")
    if not (left["exposure"] and right["exposure"] and
            left["exposure"][:7] == right["exposure"][:7]):
        raise ValueError(f"stills disagree on the resolved exposure: {left} vs {right}")
    if not left["surface_presented"] or not right["surface_presented"]:
        raise ValueError("capture was not associated with a surface submission")


def half_components(path, extent):
    words = array("H")
    words.frombytes(path.read_bytes())
    if sys.byteorder == "big":
        words.byteswap()
    if len(words) != math.prod(extent) * 4 or max(words) >= 0x7c00:
        raise ValueError(f"{path} contains invalid half components or the wrong extent")
    return words


HALF = tuple(v for (v,) in struct.iter_unpack("<e", struct.pack("<31744H", *range(0x7c00))))


def srgb(value):
    value = max(0, min(1, value))
    encoded = value * 12.92 if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
    return round(encoded * 255)


def mac_edr_identity(output):
    binary, env, cooked = route_environment(output)
    receipt = {}
    transfer = tuple(srgb(v) for v in HALF)
    for recipe in RECIPE:
        fp_path, fp = stills(output, binary, env, recipe, "edr1", 1)
        sdr_path, sdr = stills(output, binary, env, recipe, "submitted-sdr")
        rows = []
        for f, s in zip(fp["stills"], sdr["stills"], strict=True):
            same_state(f, s)
            words = half_components(fp_path / f["file"], fp["extent"])
            codes = (sdr_path / s["file"]).read_bytes()
            maximum = max(abs(a - transfer[b]) for a, b in zip(codes, words, strict=True))
            if maximum > 1:
                raise ValueError(f"{recipe} {f['name']} differs by {maximum} code values")
            rows.append(dict(name=f["name"], max_code_difference=maximum, pixels=len(words) // 4))
        receipt[recipe] = rows
    return dict(cooked_root=cooked, checkpoints=receipt)


def write_ppm(path, extent, pixels):
    width, height = extent
    path.write_bytes(f"P6\n{width} {height}\n255\n".encode() + pixels)


def previews(path, name, extent, words, headroom):
    scaled = [srgb(value / headroom) for value in HALF]
    neutral_white = 0x3c00
    rgb = bytearray()
    false_colour = bytearray()
    for i in range(0, len(words), 4):
        pixel = words[i:i + 3]
        rgb.extend(scaled[v] for v in pixel)
        peak = max(pixel)
        if peak <= neutral_white:
            gray = scaled[peak] // 3
            false_colour.extend([gray] * 3)
        else:
            level = min(1, (HALF[peak] - 1) / (headroom - 1))
            false_colour.extend([255, round(220 * level), round(96 * (1 - level))])
    write_ppm(path / f"{name}-divided-by-{headroom}.ppm", extent, rgb)
    write_ppm(path / f"{name}-above-white-{headroom}.ppm", extent, false_colour)


# Rectangles on the 1280x720 authored stills. The sky box clears the solar
# disc, and the two lamp boxes follow the warm specular spot and cool pool.
HIGHLIGHT_REGIONS = {
    "showcase-sunset-clear-sun": {
        "Sun disk": (731, 260, 738, 268),
        "sunset sky beside the Sun": (752, 243, 780, 280),
    },
    "showcase-night-lights": {
        "warm lamp": (286, 232, 331, 276),
        "cool lamp": (791, 410, 920, 521),
    },
}


def highlight_regions(name, extent, words):
    width, height = extent
    results = {}
    for region, (x0, y0, x1, y1) in HIGHLIGHT_REGIONS.get(name, {}).items():
        if not (0 <= x0 < x1 <= width and 0 <= y0 < y1 <= height):
            raise ValueError(f"{region} is outside the submitted image")
        peaks = [max(words[(y * width + x) * 4:(y * width + x) * 4 + 3])
                 for y in range(y0, y1) for x in range(x0, x1)]
        above = sum(peak > 0x3c00 for peak in peaks)
        if not above:
            raise ValueError(f"{name} {region} has no pixels above reference white")
        results[region] = dict(box=[x0, y0, x1, y1], above_white_pixels=above, peak=HALF[max(peaks)])
    return results


def mac_edr_route(output):
    binary, env, cooked = route_environment(output)
    previews_path = output / "previews"
    previews_path.mkdir()
    rows = []
    for lamps in (False, True):
        arms = {h: stills(output, binary, env, "portable", f"edr{h}", h, lamps=lamps) for h in (1, 2, 8)}
        arms["linear"] = stills(output, binary, env, "portable", "submitted-linear", lamps=lamps)
        arms["sdr"] = stills(output, binary, env, "portable", "submitted-sdr", lamps=lamps)
        rows.extend(compare_headroom_stills(arms, previews_path))
    return dict(cooked_root=cooked, checkpoints=rows, previews=str(previews_path))


def compare_headroom_stills(arms, previews_path):
    extent = arms[1][1]["extent"]
    rows = []
    for index, base in enumerate(arms[1][1]["stills"]):
        pixels = {arm: half_components(path / record["stills"][index]["file"], extent)
                  for arm, (path, record) in arms.items() if arm != "sdr"}
        for _, (_, record) in arms.items():
            same_state(base, record["stills"][index])
        linear = pixels["linear"]
        storage_scale, inverse_gain = struct.unpack("<ff", struct.pack("<II", *base["exposure"][:2]))
        exposed_knee = 0.76 * (storage_scale * inverse_gain)
        below = [i for i in range(0, len(linear), 4) if HALF[max(linear[i:i + 3])] < exposed_knee]
        if not below:
            raise ValueError(f"no below-knee samples at {base['name']}")
        highlight_stop = base["name"] in ("showcase-sunset-clear-sun", "showcase-sunset-sun", "showcase-night-lights")
        for headroom in (2, 8):
            words = pixels[headroom]
            above = sum(max(words[i:i + 3]) > 0x3c00 for i in range(0, len(words), 4))
            if HALF[max(words)] > headroom or any(words[i:i + 4] != pixels[1][i:i + 4] for i in below):
                raise ValueError(f"invalid headroom or below-knee identity at {base['name']}")
            if highlight_stop:
                previews(previews_path, base["name"], extent, words, headroom)
                if not above:
                    raise ValueError(f"{base['name']} has no values above white at H={headroom}")
            rows.append(dict(name=base["name"], headroom=headroom, above_white_pixels=above,
                             below_knee_identical=len(below), peak=HALF[max(words)],
                             highlights=highlight_regions(base["name"], extent, words)))
        if highlight_stop:
            path, sdr = arms["sdr"]
            raw = (path / sdr["stills"][index]["file"]).read_bytes()
            write_ppm(previews_path / f"{base['name']}-sdr.ppm", extent,
                      bytes(v for i, v in enumerate(raw) if i % 4 != 3))
    return rows
