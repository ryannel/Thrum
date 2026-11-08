"""Exercise clock fallback against the exported schema, without a live trace."""
from pathlib import Path
import sys
import tempfile
import xml.etree.ElementTree as ET

from pan_clock import ANCHOR, calibrate, trace_clock


schema, anchor_log = map(Path, sys.argv[1:])
origin, scale, trace_unix = trace_clock(schema)

with tempfile.TemporaryDirectory() as scratch:
    log = Path(scratch) / "game.log"

    def write(skew):
        lines = []
        for i, event in enumerate(("start", "end")):
            center = origin + 120_000_000 + i * 48_000_000
            wall = round((trace_unix + (center - origin) * scale + i * skew) / 1000) * 1000
            lines.append(f"m2 pan {event} mach_before={center-2} mach_after={center+2} unix_ns={wall} sequence={i+1}")
        log.write_text("\n".join(lines) + "\n")

    write(0)
    result = calibrate(schema, log)
    assert result["begin_ns"] > 5_000_000_000 and result["end_ns"] < 7_000_000_000
    assert 0 < result["clock_offset_bound_ns"] <= 2000
    print("exported schema accepts matching synthetic anchors and excludes both edges")

    write(10_000)
    try:
        calibrate(schema, log)
    except ValueError as error:
        assert str(error) == "clock anchor brackets disagree"
    else:
        raise AssertionError("accepted inconsistent calibration")
    print("rejects a discontinuous synthetic wall clock")

    # Keep the exported structure, but supply a synthetic epoch at the real
    # command-line probe's first Mach/UTC bracket. This checks the actual C/Rust
    # samples against the 125/3 timebase without asserting a current trace.
    _, before, _, unix, _ = ANCHOR.findall(anchor_log.read_text())[0]
    root = ET.parse(schema)
    row = root.find("node/row")
    row.find("mach-absolute-time").text = before
    row.find("time-since-epoch").text = f"{int(unix)//10**9}.{int(unix)%10**9:09d}"
    probe_schema = Path(scratch) / "probe-epoch.xml"
    root.write(probe_schema)
    real_anchors = calibrate(probe_schema, anchor_log)
    assert real_anchors["end_ns"] > 0
    print("real Rust/C anchor pair is consistent with a synthetic epoch")
