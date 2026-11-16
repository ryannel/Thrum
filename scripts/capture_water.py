#!/usr/bin/env python3
"""Package water-scene's submitted fixed or elapsed-time T2 output for review."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from capture_spool import decoded_frames

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode', choices=['fixed', 'realtime'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    stages = parser.add_mutually_exclusive_group()
    stages.add_argument('--capture-only', action='store_true')
    stages.add_argument('--package-only', action='store_true')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, THRUM_WATER_PROOF=args.mode, THRUM_WATER_OUTPUT=str(output))
    env['THRUM_WATER_COMMIT'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    if not args.package_only:
        subprocess.run(['cargo', 'test', '--release', '--locked', '--offline', '-p', 'water-scene', '--test', 'proof_gpu'], cwd=ROOT, env=env, check=True)
    if args.capture_only:
        print(f'Captured {args.mode} submitted sources at {output}; package outside the GPU lock')
        return
    record = json.loads((output / f'{args.mode}.json').read_text())
    record['tap'] = 'FrameHost submitted SDR output; fixed synthetic clock or actual monotonic elapsed driver, no desktop capture'
    width, height = record['extent']
    files = []
    if args.mode == 'fixed':
        for image in record['images']:
            raw = output / image['file']
            png = raw.with_suffix('.png')
            subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'rawvideo', '-pixel_format', 'rgba', '-video_size', f'{width}x{height}', '-i', str(raw), '-frames:v', '1', str(png)], check=True)
            image['sha256_rgba'] = hashlib.sha256(raw.read_bytes()).hexdigest()
            files.append(png)
            raw.unlink()
    else:
        raw_path = output / 'motion.spool.raw'
        if record['encoding'] == 'Raw':
            compressed = output / 'motion.spool.zst'
            encoder = subprocess.Popen(['zstd', '-q', '-f', '-1', '-T2', '-o', str(compressed)], stdin=subprocess.PIPE)
            previous = 0
            for raw in decoded_frames(raw_path, 'Raw', record['hashes'], width*height*4):
                current = int.from_bytes(raw, 'little')
                encoder.stdin.write((previous ^ current).to_bytes(width*height*4, 'little'))
                previous = current
            encoder.stdin.close()
            if encoder.wait():
                raise RuntimeError('offline lossless encoder failed')
            record['encoding'] = 'ZstdDelta'
        sources = output / 'frames'
        sources.mkdir(exist_ok=True)
        concat = []
        for index, raw in enumerate(decoded_frames(output / 'motion.spool.zst', record['encoding'], record['hashes'], width*height*4)):
            png = sources / f'{index:04}.png'
            subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'rawvideo', '-pixel_format', 'rgba', '-video_size', f'{width}x{height}', '-i', 'pipe:0', '-frames:v', '1', str(png)], input=raw, check=True)
            concat.append(f"file '{png.name}'\noption framerate 1000\n")
            if index + 1 < len(record['frames']):
                duration = record['frames'][index+1]['elapsed'] - record['frames'][index]['elapsed']
                concat.append(f'duration {duration:.9f}\n')
        (sources / 'frames.txt').write_text(''.join(concat))
        movie = output / 'motion.mp4'
        subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', str(sources/'frames.txt'), '-fps_mode', 'vfr', '-enc_time_base', '1/1000', '-c:v', 'libx264', '-crf', '16', '-pix_fmt', 'yuv420p', str(movie)], check=True)
        probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-count_frames', '-select_streams', 'v:0', '-show_entries', 'stream=nb_read_frames', '-of', 'json', str(movie)]))
        if int(probe['streams'][0]['nb_read_frames']) != len(record['frames']):
            raise RuntimeError('review movie omitted submitted source frames')
        record['movie_timebase'] = '1/1000 s; source elapsed records retain their original precision'
        files.append(movie)
        raw_path.unlink(missing_ok=True)
    record['files'] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    (output / f'{args.mode}.json').write_text(json.dumps(record, indent=2)+'\n')
    print(f'PASS {args.mode}: {len(files)} review outputs at {output}')


if __name__ == '__main__':
    main()
