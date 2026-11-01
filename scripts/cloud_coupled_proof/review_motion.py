"""Sample real canvas movies and report callback/source cadence without claiming continuous viewing.

Requires ffmpeg, ffprobe, NumPy and Pillow. Movies/receipts are retained unchanged;
only labelled contact sheets are derived. Timestamps come from the capture receipt.
"""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from PIL import Image, ImageDraw


def stats(a):
    a = np.asarray(a, float)
    return {key: float(np.percentile(a, percentile)) for key, percentile in
            [('p50', 50), ('p95', 95), ('p99', 99), ('max', 100)]}


def review(movie, receipt, out):
    data = json.loads(receipt.read_text())
    samples = data['samples']
    media = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_entries',
                       'format=duration:stream=width,height,r_frame_rate', '-of', 'json', str(movie)]))
    packets = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries', 'packet=pts_time,duration_time', '-of', 'json', str(movie)]))['packets']
    duration = max(float(p['pts_time'])+float(p.get('duration_time', 0)) for p in packets)
    media['decoded_packet_duration_s'] = duration
    media['packet_count'] = len(packets)
    # Twelve overview witnesses; retained native frames resolve edge/detail doubts.
    times = np.linspace(0, max(0, duration-.15), 12)
    sheet = Image.new('RGB', (1280, 4*265), (17, 27, 35))
    draw = ImageDraw.Draw(sheet)
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as directory:
        for i, t in enumerate(times):
            path = Path(directory)/f'{i}.png'
            subprocess.run(['ffmpeg', '-v', 'error', '-threads', '1', '-ss', str(t), '-i', str(movie),
                            '-frames:v', '1', '-threads', '1', str(path)], check=True)
            im = Image.open(path).convert('RGB')
            if i in [2, 5, 8]:
                im.save(out/f'{movie.stem}-native-{i}.png')
            x, y = (i % 3)*426, (i//3)*265
            sheet.paste(im.resize((426, 240)), (x, y+25))
            witness = min(samples, key=lambda s: abs(s['wall_ms']-t*1000))
            draw.text((x+6, y+6), f"wall {t:.2f}s / source {witness['source_s']:.2f}s", fill='white')
    sheet.save(out/f'{movie.stem}-contact.png')
    wall = np.array([s['wall_ms'] for s in samples])
    age = np.array([s['source_s'] for s in samples])
    rate = data['start']['rate']
    unclamped = age[1:] < 240
    slope = np.diff(age)[unclamped]*1000/np.diff(wall)[unclamped]
    result = {'movie': str(movie), 'media': media, 'captured_callbacks': len(samples),
              'source_range': [float(age[0]), float(age[-1])], 'rate': rate,
              'callback_interval_ms': stats(np.diff(wall)),
              'queue_completion_ms': stats([s['queue_completion_ms'] for s in samples]),
              'source_rate_max_absolute_error': float(np.max(abs(slope-rate))),
              'scope': 'Actual recorded 1280x720 canvas, sampled overview/native frames and source callback timing. '
                       'Recorder load included; queue waits are not GPU execution. No continuous-viewing claim.'}
    (out/f'{movie.stem}-review.json').write_text(json.dumps(result, indent=2)+'\n')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--movie', required=True, type=Path)
    parser.add_argument('--receipt', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(review(args.movie, args.receipt, args.out), indent=2))
