"""Check raw M1 logs, queue identities, and paused endpoint image equality."""
import json
from pathlib import Path
import re
import subprocess
import sys

root = Path(sys.argv[1]).resolve()
rows = {row: root / row for row in ("control", "paused", "running")}

def only(pattern, lines):
    matches = [re.search(pattern, line) for line in lines]
    found = [match for match in matches if match]
    assert len(found) == 1, (pattern, len(found))
    return found[0]

def image(generation, phase, log):
    pattern = rf"m1 image phase={phase} generation={generation} endpoint=SourceTick "
    return only(pattern + r"\{ epoch: (\d+), tick: (\d+), timeline: (\d+) \}$", log)

meta = {row: json.loads((path / "inputs.json").read_text()) for row, path in rows.items()}
for key in ("revision", "binary_sha256", "params_sha256", "recorder_sha256", "store_root",
            "cooked_generation"):
    assert len({value[key] for value in meta.values()}) == 1, key

report = {}
for row, path in rows.items():
    lines = (path / "game.log").read_text().splitlines()
    video = subprocess.check_output([
        "ffprobe", "-v", "error", "-select_streams", "v:0",
        "-show_entries", "stream=duration,width,height", "-of", "json", str(path / "window.mp4")
    ], text=True)
    stream, = json.loads(video)["streams"]
    assert float(stream["duration"]) > 0 and stream["width"] > 0
    recording = (path / "recorder.log").read_text().splitlines()
    recorder = only(r"^pid=(\d+) window=(\d+) ", recording)
    recorded_from = int(only(r"m1 recording start unix_ms=(\d+)$", recording).group(1))
    recorded_to = int(only(r"m1 recording stop-request unix_ms=(\d+)$", recording).group(1))
    finalized = int(only(r"m1 recording finish unix_ms=(\d+)$", recording).group(1))
    assert recorded_from < recorded_to <= finalized
    first = only(r"m1 app presented generation=1 .* window=(.*) pid=(\d+)$", lines)
    assert first.group(2) == recorder.group(1)
    before = image(1, "before", lines)
    pause = only(r"m1 app pause-ack request=Pause endpoint=SnapshotId "
                 r"\{ epoch: WorldEpoch\((\d+)\), tick: (\d+) \}", lines)
    assert pause.groups() == before.groups()[:2], (pause.group(0), before.group(0))
    assert lines.index(pause.string) < lines.index(before.string)
    generation = meta[row]["cooked_generation"]
    cooked = rf"m1 game time=\S+ drawn=Some\({generation}\) swaps=(\d+)"
    warmed = [int(match.group(1)) for line in lines[:lines.index(before.string)]
              if (match := re.search(cooked, line))]
    assert warmed and warmed[-1] > 0, warmed
    report[row] = {"pid": int(first.group(2)), "window_server_id": int(recorder.group(2)),
                   "before_endpoint": before.groups(), "video": stream}
    if row == "control":
        assert not any("m1 destroying" in line or "m1 published" in line for line in lines)
        assert any("m1 witness finish armed=false fired=false before=true" in line for line in lines)
        continue
    destroy = only(r"m1 destroying generation=1 sequence=(\d+).*?(QueueId\(\d+,\d+\))", lines)
    acquire = only(rf"m1 acquired generation=1 sequence={destroy.group(1)} surface=(\d+)$", lines)
    callback = only(rf"m1 loss callback generation=1 sequence={destroy.group(1)}$", lines)
    abandon = only(rf"m1 abandoned generation=1 sequence={destroy.group(1)}$", lines)
    published = only(r"m1 published generation=2 .*?(QueueId\(\d+,\d+\))", lines)
    first_new = only(r"m1 first present generation=2 sequence=(\d+)$", lines)
    assert published.group(1) != destroy.group(2)
    start = lines.index(destroy.string)
    assert lines.index(acquire.string) < start < lines.index(callback.string) < lines.index(abandon.string)
    assert lines.index(abandon.string) < lines.index(published.string) < lines.index(first_new.string)
    first_submit = next(i for i in range(lines.index(published.string) + 1, len(lines))
                        if lines[i].startswith(f"m1 queue submit id={published.group(1)} ")
                        and "count=0" not in lines[i])
    resumed_submit = only(rf"m1 frame submit generation=2 surface=(\d+) sequence={first_new.group(1)}$", lines)
    assert lines.index(resumed_submit.string) <= first_submit < lines.index(first_new.string)
    calls = [line for line in lines[start + 1:] if f"id={destroy.group(2)} " in line]
    assert not calls, calls
    assert any(f"m1 queue present id={destroy.group(2)} " in line for line in lines[:start])
    restored = only(rf"m1 app presented generation=2 sequence={first_new.group(1)} .* window=(.*) pid=(\d+)$", lines)
    assert restored.groups() == first.groups()
    after = image(2, "after", lines)
    restored_cooked = [int(match.group(1)) for line in lines[lines.index(published.string):lines.index(after.string)]
                       if (match := re.search(cooked, line))]
    assert restored_cooked and restored_cooked[-1] > warmed[-1], restored_cooked
    camera = only(r"m1 injected input key=KeyE tick=\d+ window=(.*)$", lines)
    assert camera.group(1) == first.group(1)
    assert lines.index(after.string) < lines.index(camera.string)
    present_ms = int(only(r"m1 event first-present-2 unix_ms=(\d+)$", lines).group(1))
    camera_ms = int(only(r"m1 event camera unix_ms=(\d+)$", lines).group(1))
    assert recorded_from <= present_ms < camera_ms < recorded_to
    assert any(line.startswith("m1 app presented generation=2 ") for line in lines[lines.index(camera.string)+1:])
    assert any("m1 witness finish armed=true fired=true before=true after=true" in line for line in lines)
    report[row].update({"after_endpoint": after.groups(), "old_queue_calls_after_destroy": len(calls),
                        "surface_before": int(acquire.group(1)), "surface_after": int(resumed_submit.group(1)),
                        "recovered_present_recording_offset_ms": present_ms - recorded_from,
                        "camera_input_recording_offset_ms": camera_ms - recorded_from})
    if row == "paused":
        assert before.groups() == after.groups(), (before.group(0), after.group(0))
        assert before.groups() == report["control"]["before_endpoint"]
        assert not any("m1 app pause-ack request=Resume" in line for line in lines)
        pre = (path / "pixels/before.rgba").read_bytes()
        post = (path / "pixels/after.rgba").read_bytes()
        control = (rows["control"] / "pixels/before.rgba").read_bytes()
        assert len(pre) == 1920 * 1080 * 4
        assert pre == post == control
        report[row]["exact_scene_pixels"] = len(pre) // 4
    else:
        # An independent authority sample during lost presentation is required.
        observed = next((match for line in lines[start:] if
                         (match := re.search(r"m1 app loss-observed generation=1 tick=(\d+)", line))), None)
        assert observed is not None
        resume = only(r"m1 app pause-ack request=Resume", lines)
        resume_input = only(r"m1 injected input key=KeyP tick=\d+ window=(.*)$", lines)
        assert resume_input.group(1) == first.group(1)
        assert lines.index(resume_input.string) < lines.index(resume.string) < start
        assert start < lines.index(observed.string) < first_submit
        samples = [int(match.group(1)) for line in lines[lines.index(observed.string)+1:first_submit]
                   if (match := re.search(r"m1 app authority tick=(\d+) state=Running", line))]
        assert samples and max(samples) > int(observed.group(1)), samples
        report[row]["authority_ticks_during_loss"] = samples
print(json.dumps(report, indent=2))
