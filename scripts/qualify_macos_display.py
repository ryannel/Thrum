#!/usr/bin/env python3
"""Check actual built-in-display swaps from an Xcode Instruments XML export.

Capture and export are intentionally separate. Game Performance can create
multi-gigabyte temporary traces even for a short app run.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

TARGET_PERIOD_NS = 1_000_000_000 / 120


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def displayed_swaps(xml_path: Path, app: str, display: str) -> list[tuple[int, int, int]]:
    root = ET.parse(xml_path).getroot()
    by_id = {element.get("id"): element for element in root.iter() if element.get("id")}
    frame_label = re.compile(rf"\({re.escape(app)} \((\d+)\):Frame (\d+):")

    def fmt(element: ET.Element | None) -> str:
        if element is None:
            return ""
        if "ref" in element.attrib:
            element = by_id.get(element.attrib["ref"])
        return "" if element is None else element.get("fmt", "")

    swaps = []
    for row in root.iter("row"):
        if fmt(row.find("display-name")) != display:
            continue
        match = frame_label.search(fmt(row.find("narrative")))
        start = row.find("start-time")
        if match is None or start is None:
            continue
        if "ref" in start.attrib:
            start = by_id[start.attrib["ref"]]
        if start.text is None:
            continue
        swaps.append((int(start.text), int(match.group(1)), int(match.group(2))))
    return sorted(swaps)


def summarize(swaps: list[tuple[int, int, int]], minimum_seconds: float) -> dict[str, object]:
    if len(swaps) < 2:
        raise ValueError("fewer than two labelled V1 swaps on the built-in display")
    processes = {pid for _, pid, _ in swaps}
    if len(processes) != 1:
        raise ValueError(f"multiple V1 process IDs in the displayed rows: {sorted(processes)}")
    times = [time for time, _, _ in swaps]
    if len(set(times)) != len(times):
        raise ValueError("duplicate swap timestamps; surface filtering is ambiguous")
    intervals = [float(right - left) for left, right in zip(times, times[1:])]
    span = (times[-1] - times[0]) / 1e9
    longest_gap_ms = max(intervals) / 1e6
    refresh_coverage = min(1.0, len(intervals) * TARGET_PERIOD_NS / (span * 1e9))
    p99_deviation_ms = percentile(
        [abs(interval - TARGET_PERIOD_NS) / 1e6 for interval in intervals], 0.99
    )
    result = {
        "display": "Built-In Display 1",
        "target_hz": 120,
        "process_id": next(iter(processes)),
        "observed_seconds": round(span, 6),
        "swaps": len(swaps),
        "first_frame_label": swaps[0][2],
        "last_frame_label": swaps[-1][2],
        "longest_gap_ms": round(longest_gap_ms, 6),
        "estimated_refresh_coverage": round(refresh_coverage, 6),
        "interval_ms": {
            f"p{int(fraction * 100)}": round(percentile(intervals, fraction) / 1e6, 6)
            for fraction in (0.5, 0.95, 0.99)
        },
        "absolute_period_error_p99_ms": round(p99_deviation_ms, 6),
        "intervals_over_1ms_error": sum(
            abs(interval - TARGET_PERIOD_NS) > 1_000_000 for interval in intervals
        ),
        "qualified": (
            span >= minimum_seconds
            and p99_deviation_ms <= 1.0
            and longest_gap_ms <= 100.0
            and refresh_coverage >= 0.99
        ),
    }
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--xml", required=True, type=Path, help="displayed-surfaces-interval XML")
    parser.add_argument("--seconds", type=float, default=600.0)
    parser.add_argument("--app", default="v1-game")
    args = parser.parse_args()
    if not math.isfinite(args.seconds) or args.seconds <= 0:
        parser.error("--seconds must be positive and finite")
    if args.seconds < 600:
        parser.error("the G3b display row requires at least 600 observed seconds")
    try:
        result = summarize(
            displayed_swaps(args.xml, args.app, "Built-In Display 1"), args.seconds
        )
    except (OSError, ValueError, ET.ParseError) as error:
        print(f"display qualification failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2))
    return 0 if result["qualified"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
