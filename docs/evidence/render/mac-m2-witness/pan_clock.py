"""Conservative pan window when trace-domain signposts are absent."""
from __future__ import annotations

import argparse
from fractions import Fraction
import json
import math
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ANCHOR = re.compile(
    r"^m2 pan (start|end) mach_before=(\d+) mach_after=(\d+) unix_ns=(\d+) sequence=(\d+)$",
    re.MULTILINE,
)


def trace_clock(path):
    root = ET.parse(path).getroot()
    nodes = [node for node in root.findall("node") if node.find("schema").get("name") == "time-info"]
    if len(nodes) != 1:
        raise ValueError("expected one trace clock table")
    rows = nodes[0].findall("row")
    if len(rows) != 1:
        raise ValueError("trace clock changed within the recording")
    row, = rows
    if row.findtext("sample-time") != "0":
        raise ValueError("trace clock row is not at the trace origin")
    origin = int(row.findtext("mach-absolute-time"))
    numerator, denominator = (int(value.text) for value in row.findall(
        "mach-timebase-info/mach-timebase-info-field"
    ))
    if origin <= 0 or numerator <= 0 or denominator <= 0:
        raise ValueError("invalid Mach clock origin or timebase")
    scale = Fraction(numerator, denominator)
    unix = Fraction(row.findtext("time-since-epoch")) * 1_000_000_000
    return origin, scale, unix


def calibrate(time_info, game_log):
    origin, scale, trace_unix = trace_clock(time_info)
    matches = ANCHOR.findall(game_log.read_text())
    if [event for event, *_ in matches] != ["start", "end"]:
        raise ValueError("expected exactly one ordered start and end anchor")
    bounds = []
    offsets = []
    for _, before, after, unix, _ in matches:
        before, after, unix = map(int, (before, after, unix))
        if not origin <= before <= after or unix <= 0:
            raise ValueError("anchor missing or outside the trace clock")
        lower = (before - origin) * scale
        upper = (after - origin) * scale
        bounds.append((lower, upper))
        # gettimeofday reports microseconds. The trace epoch has nine decimal
        # digits. Allow one reported unit from each source in both directions.
        quantization_ns = 1001
        offsets.append((unix - (trace_unix + upper) - quantization_ns,
                        unix - (trace_unix + lower) + quantization_ns))
    offset_lower = max(row[0] for row in offsets)
    offset_upper = min(row[1] for row in offsets)
    if offset_lower > offset_upper:
        raise ValueError("clock anchor brackets disagree")
    error = max(abs(offset_lower), abs(offset_upper))
    begin = math.ceil(bounds[0][1] + error)
    end = math.floor(bounds[1][0] - error)
    if begin >= end:
        raise ValueError("clock uncertainty consumes the pan window")
    return {
        "method": "trace time-info with bracketed app clock anchors",
        "begin_ns": begin,
        "end_ns": end,
        "clock_offset_bound_ns": math.ceil(error),
        "excluded_start_ns": begin - math.floor(bounds[0][0]),
        "excluded_end_ns": math.ceil(bounds[1][1]) - end,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("time_info", type=Path)
    parser.add_argument("game_log", type=Path)
    args = parser.parse_args()
    try:
        print(json.dumps(calibrate(args.time_info, args.game_log), indent=2))
    except (OSError, AttributeError, TypeError, ValueError, ET.ParseError) as error:
        parser.exit(2, f"trace clock calibration refused: {error}\n")


if __name__ == "__main__":
    main()
