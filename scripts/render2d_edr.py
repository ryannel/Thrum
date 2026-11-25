"""Verify submitted 2D pixels and window display modes on Metal."""
from array import array
import csv
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys

from apple_display_proof import all_format, window_arm
from apple_presentation import mac_binary, read_rows
from capture_spool import decoded_frames

SDR_FORMATS = {"Bgra8Unorm", "Bgra8UnormSrgb", "Rgba8Unorm", "Rgba8UnormSrgb"}


def pixel_card(environment):
    return dict(environment, THRUM_SCENE="sprites", THRUM_SPRITE_SCALE="1",
                THRUM_SPRITE_PROBE="edr-card")


def bright_indices(extent):
    width, height = extent
    # The scale-one card snaps its centre to the whole-pixel grid.
    left, top = width // 2, height // 2 - 16
    return [y * width + x for y in range(top, top + 16) for x in range(left, left + 16)]


def verify_sdr(arm):
    if not all(row["format"] in SDR_FORMATS and row["headroom"] == 1
               and row["configures"] == 1 for row in arm["presented"]):
        raise ValueError("the forced SDR window did not stay in its 8-bit mode")


def first_capture(arm, source, encoding):
    captures = [row for row in arm["rows"] if "capture_submission" in row and row["presented"]]
    if {row["capture_submission"] for row in captures} != {row["submission"] for row in arm["presented"]}:
        raise ValueError(f"capture and presentation submission identities differ: {captures}, {arm['presented']}")
    if not all(row["capture_source"] == source and row["encoding"] == encoding for row in captures):
        raise ValueError(f"the default tap did not select {source} {encoding}")
    raw = (Path(arm["path"]) / "submitted.rgba").read_bytes()
    row = captures[0]
    if len(raw) != row["pixel_bytes"]:
        raise ValueError("raw frame and encoding record disagree")
    return row, raw


def encode(value):
    code = 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1 / 2.4) - 0.055
    return round(255 * code)


def sprite(output, binary, env):
    env = dict(pixel_card(env), THRUM_MAC_EDR_PIXELS="1")
    float_arm = window_arm(output, binary, env, "float", "2", frames=4, tap=True)
    sdr_arm = window_arm(output, binary, env, "sdr", "sdr", frames=4, tap=True)
    all_format(float_arm, "Rgba16Float", "ExtendedSrgbLinear", 1)
    verify_sdr(sdr_arm)
    if not all(row["headroom"] == 2 and row["scene_kind"] == "TwoD" for row in float_arm["presented"]):
        raise ValueError("the float proof did not run a sprite scene at H=2")
    float_row, raw = first_capture(float_arm, "Edr", "rgba16float-le")
    sdr_row, sdr = first_capture(sdr_arm, "Sdr", "rgba8-srgb")
    extent = float_row["extent"]
    if extent != sdr_row["extent"] or len(raw) != 2 * len(sdr):
        raise ValueError("the capture arms did not use a common drawable grid")

    # Interpret the raw tap as stored little-endian half-float words.
    words = array("H", raw)
    if sys.byteorder != "little":
        words.byteswap()
    if max(words) != 0x4000:
        raise ValueError("float samples did not reach and stay within H=2")
    marked = set(bright_indices(extent))
    code_for = [encode(value) for value in memory_half_values()]
    for pixel in range(len(words) // 4):
        start = pixel * 4
        if pixel in marked:
            if words[start:start + 4].tolist() != [0x4000] * 3 + [0x3c00] or sdr[start:start + 4] != b"\xff" * 4:
                raise ValueError(f"bright sprite is not at the correct ceiling at pixel {pixel}")
        elif any(words[i] > 0x3c00 or abs(code_for[words[i]] - sdr[i]) > 1
                 for i in range(start, start + 4)):
            raise ValueError(f"float and SDR codes disagree outside the bright sprite at {pixel}")
    return {"float": float_row, "sdr": sdr_row, "bright_pixels": len(marked),
            "float_sha256": hashlib.sha256(raw).hexdigest(), "sdr_sha256": hashlib.sha256(sdr).hexdigest()}


def memory_half_values():
    import struct

    for word in range(0x3c01):
        yield struct.unpack("<e", word.to_bytes(2, "little"))[0]


def crowd(output, binary, env):
    env = dict(env, THRUM_FRAMES="4")
    float_arm = window_arm(output, binary, env, "float", "2", frames=4)
    sdr_arm = window_arm(output, binary, env, "sdr", "sdr", frames=4)
    all_format(float_arm, "Rgba16Float", "ExtendedSrgbLinear", 1)
    verify_sdr(sdr_arm)
    if not all(row["scene_kind"] == "TwoD" and row["headroom"] == 2 for row in float_arm["presented"]):
        raise ValueError("Crowd did not prepare a float-mode 2D host")
    return {name: arm["presented"] for name, arm in (("float", float_arm), ("sdr", sdr_arm))}


def table(path):
    with path.open() as rows:
        return list(csv.DictReader(rows, delimiter="\t"))


def observed(output, binary, env):
    display = output / "display"
    display.mkdir()
    observer = output / "observer"
    observer.mkdir()
    launch = dict(pixel_card(env), THRUM_MAC_EDR="2", THRUM_MAC_EDR_PROOF=str(display),
                  THRUM_MAC_QUALIFY=str(observer), THRUM_MAC_TAP="on", THRUM_MAC_REPLACE="1",
                  THRUM_V1_FRAMES="380", THRUM_RECIPE="portable")
    with (output / "launch.log").open("w") as log:
        subprocess.run(["/usr/bin/caffeinate", "-d", "-i", "-u", str(binary)], env=launch,
                       stdout=log, stderr=subprocess.STDOUT, timeout=150, check=True)

    surface = table(observer / "surface.tsv")
    images = table(observer / "images.tsv")
    retirements = table(observer / "retirements.tsv")
    present = {row["submission"]: row for row in read_rows(display / "frames.jsonl") if row.get("presented")}
    if not present or not all(row["format"] == "Rgba16Float" and row["headroom"] == 2
                              and row["scene_kind"] == "TwoD" for row in present.values()):
        raise ValueError("the observer changed the float surface or its headroom")
    generations = {row["submission_id"]: row["lease_generation"] for row in surface
                   if row["presented"] == "true"}
    expected_generations = set(generations.values())
    window = dict(line.split("=", 1) for line in (observer / "window.txt").read_text().splitlines())
    if (len(retirements) != 1 or int(retirements[0]["pending_staging"]) < 1
            or len(expected_generations) != 2 or window["format"] != "Rgba16Float"
            or any(row["tap_enabled"] != "true" for row in surface)):
        raise ValueError("missing float-surface replacement with the observer tap active")
    extent = [int(side) for side in window["extent"].split("x")]
    marked = bright_indices(extent)
    hashes = (observer / "hashes.txt").read_text().splitlines()
    frames = decoded_frames(observer / "frames.zst", "ZstdDelta", hashes, extent[0] * extent[1] * 4)
    checked = {}
    for image, rgba in zip(images, frames, strict=True):
        submission = image["submission_id"]
        generation = generations.get(submission)
        if generation is None or int(submission) not in present:
            raise ValueError("the 8-bit frame has no matching float presentation")
        if any(rgba[pixel * 4:pixel * 4 + 4] != b"\xff" * 4 for pixel in marked):
            raise ValueError(f"recorded sprite does not clip at white in submission {submission}")
        if generation not in checked:
            (observer / f"surface-{generation}.rgba").write_bytes(rgba)
        checked[generation] = checked.get(generation, 0) + 1
    if checked.keys() != expected_generations:
        raise ValueError("the observer recorded no images on one side of the replacement")
    return {"extent": extent, "recorded_frames_per_surface": checked, "retirements": retirements,
            "float_presentations": len(present), "bright_pixels": len(marked)}


def proof(action, output):
    binary, env = mac_binary(counters=True, crowd=action == "edr-crowd")
    check = {"edr": sprite, "edr-crowd": crowd, "edr-observed": observed}[action]
    record = check(output, binary, env)
    with binary.open("rb") as source:
        record["binary_sha256"] = hashlib.file_digest(source, "sha256").hexdigest()
    return record


def retain(source, destination):
    """Keep the receipts and a frame from each surface while dropping the full spool."""
    for pattern in ("*/*.jsonl", "*/*.tsv", "*/*.rgba", "*/window.txt",
                    "*/hashes.txt", "*/stale-recycle.txt", "launch.log"):
        for path in source.glob(pattern):
            target = destination / path.relative_to(source)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
