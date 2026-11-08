"""Read one attributed compositor swap stream inside an explicit trace window."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
import xml.etree.ElementTree as ET

NARRATIVE = re.compile(r"^Surface (\d+) \((.+) \((\d+)\):Frame (\d+):Surface (\d+)\b")


def select(xml: Path, pid: int, display: str, begin: int, end: int):
    root = ET.parse(xml).getroot()
    by_id = {element.attrib["id"]: element for element in root.iter() if "id" in element.attrib}

    def resolve(element):
        if element is not None and "ref" in element.attrib:
            return by_id[element.attrib["ref"]]
        return element

    def formatted(element):
        resolved = resolve(element)
        return "" if resolved is None else resolved.get("fmt", "")

    swaps = []
    unmatched = 0
    for row in root.iter("row"):
        if formatted(row.find("display-name")) != display:
            continue
        if formatted(row.find("display-event-name")) != "Display":
            continue
        time = resolve(row.find("start-time"))
        if time is None or time.text is None:
            raise ValueError("display row lacks numeric Swap Time")
        instant = int(time.text)
        if not begin <= instant <= end:
            continue
        narrative = resolve(row.find("narrative"))
        match = NARRATIVE.match(formatted(narrative))
        if not match:
            unmatched += 1
            continue
        compositor, app, labelled_pid, frame, surface = match.groups()
        if int(labelled_pid) != pid:
            continue
        process = formatted(narrative.find("process"))
        if process != f"{app} ({pid})":
            raise ValueError(f"narrative process disagrees with nested process {process!r}")
        swaps.append((instant, int(frame), int(compositor), int(surface)))

    swaps.sort()
    if len(swaps) < 2:
        raise ValueError("fewer than two attributable swaps in the pan window")
    if any(a[0] >= b[0] or a[1] >= b[1] for a, b in zip(swaps, swaps[1:])):
        raise ValueError("duplicate swap time or competing frame sequence")
    return swaps, unmatched


def summary(swaps, unmatched):
    intervals = sorted(b[0] - a[0] for a, b in zip(swaps, swaps[1:]))

    def percentile(fraction):
        index = (len(intervals) * fraction + 99) // 100 - 1
        return round(intervals[index] / 1_000_000, 6)

    return {
        "swaps": len(swaps),
        "first_swap_ns": swaps[0][0],
        "last_swap_ns": swaps[-1][0],
        "first_label": swaps[0][1],
        "last_label": swaps[-1][1],
        "p50_ms": percentile(50),
        "p95_ms": percentile(95),
        "p99_ms": percentile(99),
        "longest_ms": intervals[-1] / 1_000_000,
        "unlabelled_display_rows": unmatched,
        "surface_pairs": sorted({(row[2], row[3]) for row in swaps}),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("xml", type=Path)
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--display", default="Built-In Display 1")
    parser.add_argument("--begin-ns", type=int, required=True)
    parser.add_argument("--end-ns", type=int, required=True)
    args = parser.parse_args()
    if not 0 <= args.begin_ns < args.end_ns:
        parser.error("ordered, nonnegative trace boundaries required")
    try:
        result = summary(*select(args.xml, args.pid, args.display, args.begin_ns, args.end_ns))
    except (OSError, ValueError, KeyError, ET.ParseError) as error:
        parser.exit(2, f"swap attribution refused: {error}\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
