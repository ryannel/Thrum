#!/usr/bin/env python3
"""Verify every native simulator source and package its monotonic submission timeline."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from capture_spool import decoded_frames
from capture_v1_simulator import validate_session
from t2_writer_probe import package


def run(session):
    capture = session / 'capture'
    runtime = json.loads((capture / 'runtime.json').read_text())
    validate_session(runtime)
    images = runtime['images']
    width, height = images[0]['extent']
    if any(row['extent'] != [width, height] for row in images):
        raise ValueError('one capture interval changed source extent')
    if runtime['writer']['frames'] != len(images):
        raise ValueError('writer/image count mismatch')
    png = session / 'png'
    png.mkdir(exist_ok=False)
    start = images[0]['submit_ns']
    end = images[-1]['submit_ns']
    targets = [start + (end - start) * i / 16 for i in range(17)]
    selected = {min(images, key=lambda row: abs(row['submit_ns'] - t))['submission_id'] for t in targets}
    max_pair = max(zip(images, images[1:]), key=lambda pair: pair[1]['submit_ns'] - pair[0]['submit_ns'])
    selected.update(row['submission_id'] for row in max_pair)
    records = []
    hashes = runtime['writer']['hashes']
    for row, raw in zip(images, decoded_frames(capture / 'frames.spool', 'ZstdDelta', hashes, width * height * 4), strict=True):
        digest = hashlib.sha256(raw).hexdigest()
        path = png / f"{row['submission_id']:06}.png"
        if package(raw, path, width, height) != digest:
            raise ValueError('PNG round-trip mismatch')
        records.append(dict(row, rgba_sha256=digest, png_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                            selected=row['submission_id'] in selected))
        if row['submission_id'] in selected:
            # Native pixel crops, with coordinates in the source grid.
            crop_width, crop_height = min(512, width), min(512, height)
            x, y = (width - crop_width) // 2, (height - crop_height) // 2
            crop = b''.join(raw[((y+j)*width+x)*4:((y+j)*width+x+crop_width)*4] for j in range(crop_height))
            package(crop, png / f"{row['submission_id']:06}-crop.png", crop_width, crop_height)
    lines = ['ffconcat version 1.0']
    for index, row in enumerate(images):
        time_ns = row['submit_ns'] - start
        next_ns = images[index + 1]['submit_ns'] - start if index + 1 < len(images) else time_ns + 16_666_667
        duration_us = max(1, round(next_ns / 1000) - round(time_ns / 1000))
        lines += [f"file 'png/{row['submission_id']:06}.png'", 'option framerate 1000000', f'duration {duration_us / 1e6:.6f}']
    lines += [f"file 'png/{images[-1]['submission_id']:06}.png'", 'option framerate 1000000']
    (session / 'timeline.ffconcat').write_text('\n'.join(lines) + '\n')
    subprocess.run(['ffmpeg', '-v', 'error', '-f', 'concat', '-safe', '0', '-i', str(session / 'timeline.ffconcat'),
                    '-vf', 'scale=-2:960', '-fps_mode', 'vfr', '-c:v', 'libx264', '-preset', 'fast', '-crf', '16',
                    '-pix_fmt', 'yuv420p', str(session / 'preview-1x.mp4')], check=True)
    pts = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-select_streams', 'v:0',
        '-show_entries', 'frame=best_effort_timestamp_time', '-of', 'json', str(session / 'preview-1x.mp4')]))['frames']
    if len(pts) != len(images) + 1:
        raise ValueError('preview changed frame coverage')
    errors = [abs(float(p['best_effort_timestamp_time']) - (row['submit_ns'] - start) / 1e9)
              for p, row in zip(pts, images)]
    if max(errors) > .00001:
        raise ValueError('preview compressed or quantized elapsed time')
    subprocess.run(['ffmpeg', '-v', 'error', '-i', str(session / 'preview-1x.mp4'), '-vf',
                    'fps=2,scale=-2:240,tile=8x4', '-frames:v', '1', str(session / 'contact.png')], check=True)
    # Full lossless source remains in the spool. Keep a compact native selection after preview verification.
    for row in records:
        if not row['selected']:
            (png / f"{row['submission_id']:06}.png").unlink()
    with (capture / 'frames.spool').open('rb') as stream:
        spool_sha256 = hashlib.file_digest(stream, 'sha256').hexdigest()
    report = {'complete': True, 'verified_original_images': len(records), 'extent': [width, height],
              'preview_pts_max_error_seconds': max(errors), 'preview_terminal_duplicate_is_submission': False,
              'preview_scale_height': 960, 'source_spool_sha256': spool_sha256,
              'duration_ns': end - start, 'max_gap_ns': max_pair[1]['submit_ns'] - max_pair[0]['submit_ns'],
              'native_crop': {'x': (width-min(512,width))//2, 'y': (height-min(512,height))//2,
                              'width': min(512,width), 'height': min(512,height)}, 'records': records}
    (session / 'packaging.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'records'}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--session', required=True, type=Path)
    run(parser.parse_args().session)
