#!/usr/bin/env python3
"""Launch the cooked valley on the Mac or in Chrome along the G7 showcase route, or check it.

Design: docs/design/decisions/G7-SHOWCASE-ROUTE.md. Each command works in a new run
directory under tmp/g7-showcase/runs/ and records the commit, build identities and arguments
in run.json there. The route's values live only in examples/v1-game/params/showcase-*.params.json.

  mac [--stop NAME] [--frames N]   windowed game on a watched copy of the valley source
  chrome [--stop NAME] [--no-open] Chrome page on a watched, served copy; Ctrl-C stops it
  check                            visible check: the cooked valley on both targets, not a fallback
"""

import argparse
import base64
import hashlib
import http.server
import json
import os
import shutil
import signal
import struct
import subprocess
import sys
import threading
import time
import traceback
import urllib.parse
import webbrowser
import zlib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PARAMS = REPO / 'examples/v1-game/params'
SOURCE = REPO / 'examples/v1-game/assets/showcase'
RUNS = REPO / 'tmp/g7-showcase/runs'
COOK = REPO / 'target/release/thrum-cook'
GAME = REPO / 'target/release/v1-game'
WEB = REPO / 'target/v1-game-web'
WASM = WEB / 'v1_game_bg.wasm'
CHECKER = REPO / 'scripts/check_v1_browser.mjs'
BASELINE = REPO / 'docs/evidence/v1-workload/g7-showcase-baseline.json'
TOOLCHAIN = '1.99.1'

# Route order and each stop's matching capture on either target. Values come from the presets.
ROUTE = ['sunrise', 'noon', 'sunset-clear-sun', 'sunset-sun', 'twilight', 'night-upslope']
NATIVE_FILE = {
    'sunrise': 'sunrise.ppm', 'noon': 'noon.ppm', 'sunset-clear-sun': 'sunset-sun-clear.ppm',
    'sunset-sun': 'sunset-sun.ppm', 'twilight': 'twilight.ppm', 'night-upslope': 'night-upslope.ppm',
}
# The 17.4 h clear-Sun stop has no Chrome counterpart until the checker route seam lands.
CHROME_PHASE = {
    'sunrise': 'phase-scene-sunrise', 'noon': 'phase-paused-scene-noon', 'sunset-sun': 'phase-sunset',
    'twilight': 'phase-scene-twilight', 'night-upslope': 'phase-upslope-midnight',
}
KEYS = {'world.paused', 'sky.solar_time', 'camera.yaw', 'camera.pitch'}

# Comparator version: 2 maps both extents onto the grid by exact area coverage (version 1 used integer cells,
# which misaligned cell edges between 1252 and 1280 pixel frames by up to a pixel).
COMPARATOR = 2
# Thresholds per statistic (median, p95, max of 32x18 cell mean absolute 8-bit RGB difference), fixed by the
# calibration rule max(1, 3 x worst valley-to-native), rounded up, on the first run of this comparator and source
# root that passes both controls. None means that calibration run. Changing them is a recorded decision.
# Comparator 2, source root 59f0484f (authored ground, 80e519ec): worst valley 0.027, 0.062, 0.161 -> the 1.0 floor.
THRESHOLDS = [1.0, 1.0, 1.0]
# The source root the thresholds were calibrated on. Another root needs a reviewed recalibration, so a parity
# regression that arrives with a content change is not absorbed.
CALIBRATED_ROOT = '59f0484f6f05e5c1a0cf33f62755f1083a3802a7b32631191d6d88afff08c367'
CONTROL_RATIO = 10.0


class Failure(Exception):
    pass


def route():
    found = sorted(path.name for path in PARAMS.glob('showcase-*.params.json'))
    expected = sorted(f'showcase-{stop}.params.json' for stop in ROUTE)
    if found != expected:
        raise Failure(f'route presets {found} differ from the route {expected}')
    stops = {}
    for stop in ROUTE:
        values = json.loads((PARAMS / f'showcase-{stop}.params.json').read_text())
        if set(values) != KEYS or values['world.paused'] is not True:
            raise Failure(f'{stop}: a route preset holds exactly {sorted(KEYS)} with world.paused true')
        stops[stop] = values
    return stops


def sha256(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            digest.update(block)
    return digest.hexdigest()


def git(*args):
    return subprocess.run(['git', *args], cwd=REPO, capture_output=True, text=True, check=True).stdout.strip()


def new_run(command):
    RUNS.mkdir(parents=True, exist_ok=True)
    run = RUNS / f'{command}-{time.strftime("%Y%m%d-%H%M%S")}'
    run.mkdir()
    return run


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def build(run, web):
    """Build release thrum-cook and v1-game, and the default web page if asked; return build seconds."""
    started = time.monotonic()
    with open(run / 'build.log', 'w') as log:
        steps = [['rustup', 'run', TOOLCHAIN, 'cargo', 'build', '--release', '--locked',
                  '-p', 'thrum-cook', '-p', 'v1-game', '--bin', 'thrum-cook', '--bin', 'v1-game']]
        if web:
            steps.append(['sh', 'examples/v1-game/web/build.sh'])
        for step in steps:
            if subprocess.run(step, cwd=REPO, stdout=log, stderr=subprocess.STDOUT).returncode:
                raise Failure(f'build failed: {" ".join(step)} (see {run / "build.log"})')
    return time.monotonic() - started


def identities(web, built):
    # Under --no-build the binaries may predate the commit; only a run that builds proves they match it.
    found = {'commit': git('rev-parse', 'HEAD'), 'dirty': bool(git('status', '--porcelain')), 'built_by_this_run': built}
    for name, path in [('thrum_cook', COOK), ('v1_game', GAME)] + ([('wasm', WASM)] if web else []):
        if not path.exists():
            raise Failure(f'{path} is not built; run without --no-build')
        found[f'{name}_sha256'] = sha256(path)
    return found


def copy_source(run):
    source = run / 'source'
    shutil.copytree(SOURCE, source)
    return source


def watch(run, source, serve):
    """Start thrum-cook watch on the copy; wait for generation 1 (and the origin when serving)."""
    log_path = run / 'cook.log'
    log = open(log_path, 'w')
    command = [str(COOK), 'watch', str(source), str(run / 'store')] + (['--serve', '127.0.0.1:0'] if serve else [])
    process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    log.close()
    try:
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise Failure(f'thrum-cook exited early (see {log_path})')
            text = log_path.read_text()
            origin = next((line.split()[1] for line in text.splitlines() if line.startswith('serving ')), None)
            if 'generation 1' in text and (origin or not serve):
                return process, origin
            time.sleep(0.1)
        raise Failure('thrum-cook did not publish generation 1 within 120 s')
    except BaseException:
        stop_process(process)
        raise


def stop_process(process):
    if process and process.poll() is None:
        process.terminate()
        try:
            process.wait(10)
        except subprocess.TimeoutExpired:
            process.kill()


def mac(args):
    stops = route()
    run = new_run('mac')
    seconds = None if args.no_build else build(run, web=False)
    record = {'command': 'mac', 'stop': args.stop, 'frames': args.frames, 'build_seconds': seconds,
              'builds': identities(web=False, built=not args.no_build), 'preset': stops[args.stop],
              'store': str(run / 'store'), 'scene': 'valley'}
    write_json(run / 'run.json', record)
    watcher, _ = watch(run, copy_source(run), serve=False)
    env = dict(os.environ, THRUM_ASSET_STORE=str(run / 'store'), THRUM_SCENE='valley',
               THRUM_PARAMS=str(PARAMS / f'showcase-{args.stop}.params.json'))
    if args.frames:
        env['THRUM_V1_FRAMES'] = str(args.frames)
    print(f'mac: {args.stop}; edits to {run / "source"} cook live; L re-applies the preset. Run: {run}', flush=True)
    game = None
    try:
        with open(run / 'game.log', 'w') as log:
            game = subprocess.Popen([str(GAME)], env=env, stdout=log, stderr=subprocess.STDOUT)
            code = game.wait()
    except KeyboardInterrupt:
        code = 'interrupted'
    finally:
        stop_process(game)
        stop_process(watcher)
    record['exit_code'] = code
    write_json(run / 'run.json', record)
    if code:
        raise Failure(f'v1-game exited {code} (see {run / "game.log"})')


class Logged(http.server.SimpleHTTPRequestHandler):
    """Static handler that appends each request line and status to the run's requests.log."""
    log_path = None

    def log_message(self, *_):
        pass

    def log_request(self, code='-', size='-'):
        with open(self.log_path, 'a') as log:
            log.write(f'{time.time():.3f} {code} {self.requestline}\n')


def chrome(args):
    route()
    run = new_run('chrome')
    seconds = None if args.no_build else build(run, web=True)
    record = {'command': 'chrome', 'stop': args.stop, 'build_seconds': seconds, 'builds': identities(web=True, built=not args.no_build)}
    watcher = server = None
    try:
        watcher, assets = watch(run, copy_source(run), serve=True)
        # build.sh copies the presets into the web root, so the page fetches them beside index.html.
        handler = type('Handler', (Logged,), {'log_path': run / 'requests.log'})
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), lambda *a: handler(*a, directory=str(WEB)))
        threading.Thread(target=server.serve_forever, daemon=True).start()
        query = urllib.parse.urlencode({'assets': assets, 'params': f'showcase-{args.stop}.params.json'})
        url = f'http://127.0.0.1:{server.server_address[1]}/index.html?{query}'
        record['url'] = url
        write_json(run / 'run.json', record)
        print(f'chrome: {url}\nRun: {run}; Ctrl-C stops both servers.', flush=True)
        if not args.no_open:
            webbrowser.get('open -a "Google Chrome" %s').open(url)
        while watcher.poll() is None:
            time.sleep(0.5)
        raise Failure('thrum-cook watch exited')
    except KeyboardInterrupt:
        pass
    finally:
        if server:
            server.shutdown()
        stop_process(watcher)


# --- Pixel comparison (stdlib PNG and PPM readers) ---

def read_png(path):
    data = Path(path).read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise Failure(f'{path}: not a PNG')
    pos, idat = 8, []
    while pos < len(data):
        length, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b'IHDR':
            width, height, depth, colour, _, _, interlace = struct.unpack('>IIBBBBB', body)
            if depth != 8 or colour not in (2, 6) or interlace:
                raise Failure(f'{path}: unsupported PNG layout')
            bpp = 3 if colour == 2 else 4
        elif kind == b'IDAT':
            idat.append(body)
        pos += 12 + length
    raw = zlib.decompress(b''.join(idat))
    stride = width * bpp
    out = bytearray(height * stride)
    previous = bytearray(stride)
    offset = 0
    for y in range(height):
        kind = raw[offset]
        line = bytearray(raw[offset + 1:offset + 1 + stride])
        offset += 1 + stride
        if kind == 1:
            for x in range(bpp, stride):
                line[x] = (line[x] + line[x - bpp]) & 255
        elif kind == 2:
            line = bytearray((a + b) & 255 for a, b in zip(line, previous))
        elif kind == 3:
            for x in range(stride):
                left = line[x - bpp] if x >= bpp else 0
                line[x] = (line[x] + (left + previous[x]) // 2) & 255
        elif kind == 4:
            for x in range(stride):
                a = line[x - bpp] if x >= bpp else 0
                b = previous[x]
                c = previous[x - bpp] if x >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        elif kind != 0:
            raise Failure(f'{path}: unknown PNG filter {kind} on row {y}')
        out[y * stride:(y + 1) * stride] = line
        previous = line
    return width, height, bpp, bytes(out)


def read_ppm(path):
    data = Path(path).read_bytes()
    fields, pos = [], 0
    while len(fields) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b'#':
            pos = data.index(b'\n', pos)
            continue
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        fields.append(data[pos:end])
        pos = end
    width, height = int(fields[1]), int(fields[2])
    pixels = data[pos + 1:]
    if fields[0] != b'P6' or fields[3] != b'255' or len(pixels) != width * height * 3:
        raise Failure(f'{path}: not a complete 8-bit binary PPM')
    return width, height, 3, pixels


def coverage(length, cells):
    """Each pixel's (cell, weight) parts when `length` pixels are mapped onto `cells` equal cells.

    Weights are exact area fractions, so frames of different extents average over the same regions
    of the view. Integer cell assignment would move cell edges by up to a pixel between extents.
    """
    parts = []
    for p in range(length):
        start, end = p * cells / length, (p + 1) * cells / length
        first = int(start)
        if first + 1 < end and first + 1 < cells:
            parts.append(((first, first + 1 - start), (first + 1, end - first - 1)))
        else:
            parts.append(((min(first, cells - 1), end - start),))
    return parts


def grid(path, columns=32, rows=18, crop=None):
    width, height, bpp, pixels = read_ppm(path) if str(path).endswith('.ppm') else read_png(path)
    x0, y0, x1, y1 = crop or (0, 0, width, height)
    if not (0 <= x0 < x1 <= width and 0 <= y0 < y1 <= height):
        raise Failure(f'{path}: crop {crop} outside {width}x{height}')
    across, down = coverage(x1 - x0, columns), coverage(y1 - y0, rows)
    sums = [[0.0, 0.0, 0.0, 0.0] for _ in range(columns * rows)]
    for y in range(y1 - y0):
        # Sum each source row into per-column totals first, then spread those over the rows it covers.
        line = [[0.0, 0.0, 0.0, 0.0] for _ in range(columns)]
        base = (y + y0) * width * bpp
        for x in range(x1 - x0):
            o = base + (x + x0) * bpp
            r, g, b = pixels[o], pixels[o + 1], pixels[o + 2]
            for cell, weight in across[x]:
                total = line[cell]
                total[0] += r * weight
                total[1] += g * weight
                total[2] += b * weight
                total[3] += weight
        for row, weight in down[y]:
            for column, total in enumerate(line):
                cell = sums[row * columns + column]
                cell[0] += total[0] * weight
                cell[1] += total[1] * weight
                cell[2] += total[2] * weight
                cell[3] += total[3] * weight
    return [[s[0] / s[3], s[1] / s[3], s[2] / s[3]] for s in sums]


def difference(a, b):
    cells = sorted(sum(abs(p - q) for p, q in zip(x, y)) / 3 for x, y in zip(a, b))
    return [cells[len(cells) // 2], cells[int(len(cells) * 0.95)], cells[-1]]


def close(a, b, tolerance):
    return abs(float(a) - float(b)) <= tolerance


def pack(cells):
    """A grid as base64 little-endian uint16 hundredths of an 8-bit level (0.005 rounding)."""
    return base64.b64encode(b''.join(struct.pack('<H', round(v * 100)) for cell in cells for v in cell)).decode()


def unpack(text):
    raw = base64.b64decode(text)
    values = [v / 100 for (v,) in struct.iter_unpack('<H', raw)]
    return [values[i:i + 3] for i in range(0, len(values), 3)]


def compare_baseline(grids, identity, thresholds):
    """Each stop's difference from the checked-in baseline, when the baseline is for this identity."""
    if not BASELINE.exists():
        return {'status': 'absent'}
    saved = json.loads(BASELINE.read_text())
    differs = sorted(key for key in identity if saved['identity'].get(key) != identity[key])
    if differs:
        return {'status': 'other identity', 'differs': differs, 'baseline_identity': saved['identity']}
    result = {'status': 'compared', 'stops': {}, 'over': []}
    for target, stops in grids.items():
        for stop, cells in stops.items():
            values = difference(cells, unpack(saved['grids'][target][stop]))
            result['stops'][f'{target}/{stop}'] = values
            if any(value > limit for value, limit in zip(values, thresholds)):
                result['over'].append(f'{target}/{stop}')
    return result


def save_baseline(grids, identity, report, run):
    BASELINE.write_text(json.dumps({
        'about': 'G7 showcase route baseline: per-stop 32x18 grids consumed by scripts/run_v1_showcase.py check. '
                 'Reproduce with `check`; replace only with `check --save-baseline` after reviewing the frames, and '
                 'record why in docs/evidence/v1-workload/G7-CHROME-VALLEY-2026-11-07.md.',
        'identity': identity,
        'builds': report['builds'],
        'thresholds': report['thresholds'],
        'run': run.name,
        'grids': {target: {stop: pack(cells) for stop, cells in stops.items()} for target, stops in grids.items()},
    }, indent=1) + '\n')


def bounded(command, log, seconds, **options):
    """Run one check step with a deadline, so a hung step fails the check instead of holding it."""
    try:
        return subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=seconds, **options).returncode
    except subprocess.TimeoutExpired:
        raise Failure(f'{Path(command[0]).name} exceeded {seconds} s') from None


def check(args):
    stops = route()
    run = new_run('check')
    report = {'command': 'check', 'route': stops, 'comparator': COMPARATOR, 'passed': False, 'failures': []}
    report['build_seconds'] = None if args.no_build else build(run, web=True)
    report['builds'] = identities(web=True, built=not args.no_build)
    write_json(run / 'run.json', report)
    failures = report['failures']
    timings = report['timings_seconds'] = {}
    try:
        source = copy_source(run)
        started = time.monotonic()
        with open(run / 'cook.log', 'w') as log:
            if bounded([str(COOK), 'cook', str(source), str(run / 'store')], log, 120):
                raise Failure('thrum-cook cook failed')
        timings['cook'] = time.monotonic() - started
        head = (run / 'store/HEAD').read_bytes()
        if head[:8] != b'THRMHEAD' or struct.unpack('<Q', head[16:24])[0] != 1:
            raise Failure('the cooked store is not at generation 1')
        root = head[24:56].hex()
        report['generation_1_root'] = root

        # Native: THRUM_SCENE=valley makes capture require the store and refuse unless it is drawn.
        native = run / 'native'
        env = dict(os.environ, THRUM_CAPTURE_SCENE=str(native), THRUM_ASSET_STORE=str(run / 'store'), THRUM_SCENE='valley')
        launched = time.time()
        started = time.monotonic()
        with open(run / 'native.log', 'w') as log:
            code = bounded([str(GAME)], log, 300, env=env)
        timings['native_capture_run'] = time.monotonic() - started
        if code:
            raise Failure(f'native capture refused or failed (exit {code}; see {run / "native.log"})')
        manifest = json.loads((native / 'captures.json').read_text())
        first = min((native / record['file']).stat().st_mtime for record in manifest['captures'] if 'file' in record)
        timings['native_cold_start_proxy'] = first - launched
        poses = {record['file']: record for record in manifest['captures'] if 'file' in record}
        for stop, values in stops.items():
            pose = poses.get(NATIVE_FILE[stop])
            if not pose or not (close(pose['hours'], values['sky.solar_time'], 1e-4) and close(pose['yaw_degrees'], values['camera.yaw'], 1e-3)
                                and close(pose['pitch_degrees'], values['camera.pitch'], 1e-3)):
                failures.append(f'native pose drift at {stop}: {pose} vs {values}')

        # Chrome: the cooked valley, then the same procedure without ?assets as the negative control.
        chrome_runs = {}
        for case, name in [('valley', 'chrome-valley'), ('sky-reflections', 'chrome-diagnostic')]:
            started = time.monotonic()
            with open(run / f'{name}.log', 'w') as log:
                code = bounded(['node', str(CHECKER), '--case', case, '--cook', str(COOK), '--web-root', str(WEB),
                                       '--assets-source', str(source), '--output', str(run / name)],
                               log, 900, cwd=REPO)
            timings[f'{name}_checker_run'] = time.monotonic() - started
            if code:
                raise Failure(f'checker --case {case} failed (see {run / f"{name}.log"})')
            result = json.loads((run / name / 'run.json').read_text())['results'][0]
            if result['actualWasmSha256'] != report['builds']['wasm_sha256']:
                raise Failure(f'{case}: Chrome loaded Wasm {result["actualWasmSha256"]}, not the build')
            chrome_runs[case] = {phase['name']: phase for phase in result['sky']['phases']}
        if not (run / 'chrome-valley/store/cas' / root[:2] / root).is_file():
            failures.append(f'same cooked input: generation-1 root {root} is absent from the checker store')
        for stop, phase_name in CHROME_PHASE.items():
            values = stops[stop]
            for case, phases in chrome_runs.items():
                phase = phases.get(phase_name)
                want = {'hours': values['sky.solar_time'], 'yaw': values['camera.yaw'], 'pitch': values['camera.pitch']}
                got = phase and phase['edited']['controls']
                # world.paused: the checker asserts the paused tick around every capture; check it was recorded.
                if (not phase or phase['settled'].get('paused') is not True or any(not close(phase['requested'][k], want[k], 1e-9) for k in want)
                        or not (close(got['hours'], want['hours'], 0.0051) and close(got['yaw'], want['yaw'], 0.5001)
                                and close(got['pitch'], want['pitch'], 0.5001))):
                    failures.append(f'Chrome {case} pose drift at {stop}: {phase and phase["requested"]} / {got} vs {want}')
        if failures:
            raise Failure('pose drift')

        # Compare matched stops.
        grids = {'native': {stop: grid(native / NATIVE_FILE[stop]) for stop in ROUTE},
                 'chrome': {stop: grid(run / 'chrome-valley' / f'valley-{phase}.png') for stop, phase in CHROME_PHASE.items()}}
        statistics = report['statistics'] = {}
        for stop, phase_name in CHROME_PHASE.items():
            statistics[stop] = {
                'valley': difference(grids['chrome'][stop], grids['native'][stop]),
                'diagnostic': difference(grid(run / 'chrome-diagnostic' / f'sky-reflections-{phase_name}.png'), grids['native'][stop]),
            }
        # The edit frames are full-page shots; compare only the canvas, located by the checker's own clip.
        clip = chrome_runs['valley'][CHROME_PHASE['noon']]['images'][0]['clip']
        canvas = (round(clip['x']), round(clip['y']), round(clip['x'] + clip['width']), round(clip['y'] + clip['height']))
        report['positive_control_crop'] = canvas
        edit = difference(grid(run / 'chrome-valley/valley-before-edit-final.png', crop=canvas),
                          grid(run / 'chrome-valley/valley-edited-final.png', crop=canvas))
        report['positive_control'] = edit
        worst = [max(values['valley'][i] for values in statistics.values()) for i in range(3)]
        rule = [max(1.0, 3 * value) for value in worst]
        report['calibration'] = {'worst_valley': worst, 'rule_thresholds': rule, 'fixed_thresholds': THRESHOLDS}
        if THRESHOLDS and root != CALIBRATED_ROOT:
            raise Failure(f'thresholds were calibrated on source root {CALIBRATED_ROOT[:8]}, not {root[:8]}: '
                          'recalibrate by the design rule and record it')
        thresholds = THRESHOLDS or rule
        report['thresholds'] = thresholds
        names = ['median', 'p95', 'max']
        for stop, values in statistics.items():
            over = [f'{names[i]} {values["valley"][i]:.3f} > {thresholds[i]:.3f}' for i in range(3) if values['valley'][i] > thresholds[i]]
            if over:
                failures.append(f'Chrome fallback or divergence at {stop}: {", ".join(over)}')
            if not any(values['diagnostic'][i] >= CONTROL_RATIO * thresholds[i] for i in range(3)):
                failures.append(f'negative control: the diagnostic scene at {stop} is within {CONTROL_RATIO}x of the thresholds')
        if not any(edit[i] > thresholds[i] for i in range(3)):
            failures.append('positive control: the retinted texture did not exceed any threshold')
        identity = {'comparator': COMPARATOR, 'source_root': root,
                    'route': {stop: sha256(PARAMS / f'showcase-{stop}.params.json') for stop in ROUTE}}
        baseline = report['baseline'] = compare_baseline(grids, identity, thresholds)
        if baseline['status'] == 'compared' and baseline['over']:
            failures.append(f'baseline drift beyond the thresholds at {", ".join(baseline["over"])}')
        if args.save_baseline and not failures:
            save_baseline(grids, identity, report, run)
            baseline['saved'] = str(BASELINE.relative_to(REPO))
    except Failure as error:
        failures.append(str(error))
    except Exception:  # Record any other fault in run.json before failing.
        failures.append(traceback.format_exc())
    except KeyboardInterrupt:
        failures.append('interrupted')
    report['passed'] = not failures
    write_json(run / 'run.json', report)
    for stop, values in report.get('statistics', {}).items():
        print(f'{stop:>14}  valley {" ".join(f"{v:7.3f}" for v in values["valley"])}  '
              f'diagnostic {" ".join(f"{v:7.2f}" for v in values["diagnostic"])}')
    if 'thresholds' in report:
        print(f'thresholds {report["thresholds"]}  positive control {report["positive_control"]}')
    baseline = report.get('baseline', {})
    if baseline.get('status') == 'compared':
        worst = [max(values[i] for values in baseline['stops'].values()) for i in range(3)]
        print(f'baseline: worst difference {" ".join(f"{v:.3f}" for v in worst)} over {len(baseline["stops"])} frames')
    elif baseline:
        print(f'baseline: {baseline["status"]} {baseline.get("differs", "")}')
    if baseline.get('saved'):
        print(f'baseline saved: {baseline["saved"]}')
    print(f'{"PASS" if report["passed"] else "FAIL"}: {run / "run.json"}')
    for failure in failures:
        print(f'  {failure}')
    if failures:
        raise SystemExit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest='command', required=True)
    for name in ('mac', 'chrome', 'check'):
        command = commands.add_parser(name)
        command.add_argument('--no-build', action='store_true', help='use the existing release and web builds')
        if name != 'check':
            command.add_argument('--stop', choices=ROUTE, default=ROUTE[0])
    commands.choices['mac'].add_argument('--frames', type=int, help='bound the run (THRUM_V1_FRAMES)')
    commands.choices['check'].add_argument('--save-baseline', action='store_true',
                                           help='write the baseline after a passing run (a reviewed, recorded act)')
    commands.choices['chrome'].add_argument('--no-open', action='store_true', help='print the URL instead of opening Chrome')
    args = parser.parse_args()
    # A terminated launcher stops its watcher, servers and game the same way Ctrl-C does.
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt))
    try:
        {'mac': mac, 'chrome': chrome, 'check': check}[args.command](args)
    except Failure as error:
        print(f'FAIL: {error}', file=sys.stderr)
        raise SystemExit(1)


if __name__ == '__main__':
    main()
