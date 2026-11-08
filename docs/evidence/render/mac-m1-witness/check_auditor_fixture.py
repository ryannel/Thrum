"""Self-test the evidence checker without touching the real window."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile

out = Path(__file__).resolve().parent
with tempfile.TemporaryDirectory(dir=out) as tmp:
    root = Path(tmp)
    clip = root / "fixture.mp4"
    subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i", "color=s=32x32:d=1",
                    "-c:v", "libx264", str(clip)], check=True)
    early = """m1 app presented generation=1 sequence=1 tick=0 window=Some(WindowId(123)) pid=42
m1 app pause-ack request=Pause endpoint=SnapshotId { epoch: WorldEpoch(1), tick: 0 }
m1 game time=12.0 drawn=Some(1) swaps=1 submitted=60
m1 image phase=before generation=1 endpoint=SourceTick { epoch: 1, tick: 0, timeline: 0 }
m1 queue present id=QueueId(0,1) surface=SurfaceId(0,1)
"""
    failure = """m1 acquired generation=1 sequence=61 surface=1
m1 destroying generation=1 sequence=61 queue=Queue { id: QueueId(0,1) }
m1 loss callback generation=1 sequence=61
m1 abandoned generation=1 sequence=61
m1 app loss-observed generation=1 tick=0
"""
    resumed = """m1 published generation=2 queue=Queue { id: QueueId(1,1) }
m1 frame submit generation=2 surface=2 sequence=62
m1 queue submit id=QueueId(1,1) count=1
m1 queue present id=QueueId(1,1) surface=SurfaceId(0,1)
m1 first present generation=2 sequence=62
m1 event first-present-2 unix_ms=2000
m1 app presented generation=2 sequence=62 tick=0 window=Some(WindowId(123)) pid=42
m1 game time=12.0 drawn=Some(1) swaps=2 submitted=62
m1 image phase=after generation=2 endpoint=SourceTick { epoch: 1, tick: 0, timeline: 0 }
m1 app camera E tick=0 window=Some(WindowId(123))
m1 event camera unix_ms=3000
m1 app presented generation=2 sequence=63 tick=0 window=Some(WindowId(123)) pid=42
m1 witness finish armed=true fired=true before=true after=true
"""
    identities = {"revision": "fixture", "binary_sha256": "", "params_sha256": "",
                  "recorder_sha256": "", "store_root": "", "cooked_generation": 1}
    logs = {
        "control": early + "m1 witness finish armed=false fired=false before=true after=false\n",
        "paused": early + failure + resumed,
        "running": early + "m1 app pause-ack request=Resume\n" + failure
                   + "m1 app authority tick=1 state=Running\n" + resumed,
    }
    for row, log in logs.items():
        directory = root / row
        directory.mkdir()
        (directory / "pixels").mkdir()
        for phase in ("before", "after"):
            with (directory / "pixels" / f"{phase}.rgba").open("wb") as file:
                file.truncate(1920 * 1080 * 4)
        (directory / "window.mp4").symlink_to(clip)
        (directory / "recorder.log").write_text(
            "pid=42 window=1234 frame=(fixture)\nm1 recording start unix_ms=1000\n"
            "m1 recording finish unix_ms=4000\n"
        )
        (directory / "inputs.json").write_text(json.dumps(identities))
        (directory / "game.log").write_text(log)
    command = [sys.executable, str(out / "audit_live_witness.py"), str(root)]
    baseline = subprocess.run(command, capture_output=True, text=True)
    assert baseline.returncode == 0, baseline.stderr
    print("valid synthetic fixture passes")
    paused = root / "paused/game.log"
    running = root / "running/game.log"
    for name, path, old, new in (
        ("old empty submit", paused, "m1 abandoned", "m1 queue submit id=QueueId(0,1) count=0\nm1 abandoned"),
        ("world tick never advances", running, "authority tick=1", "authority tick=0"),
        ("changed acknowledged endpoint", paused, "timeline: 0 }\nm1 app camera", "timeline: 1 }\nm1 app camera"),
        ("cooked generation never redrawn", paused, "swaps=2", "swaps=1"),
        ("post-recovery camera input outside recording", paused, "camera unix_ms=3000", "camera unix_ms=5000"),
    ):
        original = path.read_text()
        assert old in original
        path.write_text(original.replace(old, new, 1))
        result = subprocess.run(command, capture_output=True, text=True)
        assert result.returncode != 0, name
        print(f"rejects {name}")
        path.write_text(original)
    with (root / "paused/pixels/after.rgba").open("r+b") as file:
        file.write(b"\xff")
    assert subprocess.run(command, capture_output=True).returncode != 0
    print("rejects one-byte paused pixel change")
