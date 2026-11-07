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

# Thresholds per statistic (median, p95, max of 32x18 cell mean absolute 8-bit RGB difference),
# fixed by the calibration rule max(1, 3 x worst valley-to-native), rounded up, on the first run that
# passed both controls (7 Nov 2026, worst 0.069, 0.482, 2.153). Changing them is a recorded decision.
THRESHOLDS = [1.0, 1.447, 6.461]
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


def identities(web):
    found = {'commit': git('rev-parse', 'HEAD'), 'dirty': bool(git('status', '--porcelain'))}
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
    deadline = time.monotonic() + 120
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise Failure(f'thrum-cook exited early (see {log_path})')
        text = log_path.read_text()
        origin = next((line.split()[1] for line in text.splitlines() if line.startswith('serving ')), None)
        if 'generation 1' in text and (origin or not serve):
            return process, origin
        time.sleep(0.1)
    process.terminate()
    raise Failure('thrum-cook did not publish generation 1 within 120 s')


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
              'builds': identities(web=False), 'preset': stops[args.stop]}
    write_json(run / 'run.json', record)
    watcher, _ = watch(run, copy_source(run), serve=False)
    env = dict(os.environ, THRUM_ASSET_STORE=str(run / 'store'), THRUM_SCENE='valley',
               THRUM_PARAMS=str(PARAMS / f'showcase-{args.stop}.params.json'))
    if args.frames:
        env['THRUM_V1_FRAMES'] = str(args.frames)
    print(f'mac: {args.stop}; edits to {run / "source"} cook live; L re-applies the preset. Run: {run}', flush=True)
    try:
        with open(run / 'game.log', 'w') as log:
            code = subprocess.run([str(GAME)], env=env, stdout=log, stderr=subprocess.STDOUT).returncode
    finally:
        stop_process(watcher)
    record['exit_code'] = code
    write_json(run / 'run.json', record)
    if code:
        raise Failure(f'v1-game exited {code} (see {run / "game.log"})')


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass


def chrome(args):
    route()
    run = new_run('chrome')
    seconds = None if args.no_build else build(run, web=True)
    record = {'command': 'chrome', 'stop': args.stop, 'build_seconds': seconds, 'builds': identities(web=True)}
    watcher, assets = watch(run, copy_source(run), serve=True)
    # build.sh copies the presets into the web root, so the page fetches them beside index.html.
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), lambda *a: Quiet(*a, directory=str(WEB)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    url = (f'http://127.0.0.1:{server.server_address[1]}/index.html?assets={assets.replace(":", "%3A").replace("/", "%2F")}'
           f'&params=showcase-{args.stop}.params.json')
    record['url'] = url
    write_json(run / 'run.json', record)
    print(f'chrome: {url}\nRun: {run}; Ctrl-C stops both servers.', flush=True)
    if not args.no_open:
        webbrowser.get('open -a "Google Chrome" %s').open(url)
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt))
    try:
        while watcher.poll() is None:
            time.sleep(0.5)
        raise Failure('thrum-cook watch exited')
    except KeyboardInterrupt:
        pass
    finally:
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
        out[y * stride:(y + 1) * stride] = line
        previous = line
    return width, height, bpp, bytes(out)


def read_ppm(path):
    data = Path(path).read_bytes()
    fields, pos = [], 0
    while len(fields) < 4:
        while data[pos:pos + 1].isspace():
            pos += 1
        end = pos
        while not data[end:end + 1].isspace():
            end += 1
        fields.append(data[pos:end])
        pos = end
    if fields[0] != b'P6' or fields[3] != b'255':
        raise Failure(f'{path}: not an 8-bit binary PPM')
    return int(fields[1]), int(fields[2]), 3, data[pos + 1:]


def grid(path, columns=32, rows=18):
    width, height, bpp, pixels = read_ppm(path) if str(path).endswith('.ppm') else read_png(path)
    sums = [[0, 0, 0, 0] for _ in range(columns * rows)]
    cell_x = [x * columns // width for x in range(width)]
    for y in range(height):
        row = (y * rows // height) * columns
        base = y * width * bpp
        for x in range(width):
            cell = sums[row + cell_x[x]]
            o = base + x * bpp
            cell[0] += pixels[o]
            cell[1] += pixels[o + 1]
            cell[2] += pixels[o + 2]
            cell[3] += 1
    return [[s[0] / s[3], s[1] / s[3], s[2] / s[3]] for s in sums]


def difference(a, b):
    cells = sorted(sum(abs(p - q) for p, q in zip(x, y)) / 3 for x, y in zip(a, b))
    return [cells[len(cells) // 2], cells[int(len(cells) * 0.95)], cells[-1]]


def close(a, b, tolerance):
    return abs(float(a) - float(b)) <= tolerance


def check(args):
    stops = route()
    run = new_run('check')
    report = {'command': 'check', 'route': stops, 'passed': False, 'failures': []}
    report['build_seconds'] = None if args.no_build else build(run, web=True)
    report['builds'] = identities(web=True)
    write_json(run / 'run.json', report)
    failures = report['failures']
    timings = report['timings_seconds'] = {}
    try:
        source = copy_source(run)
        started = time.monotonic()
        with open(run / 'cook.log', 'w') as log:
            if subprocess.run([str(COOK), 'cook', str(source), str(run / 'store')], stdout=log, stderr=subprocess.STDOUT).returncode:
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
            code = subprocess.run([str(GAME)], env=env, stdout=log, stderr=subprocess.STDOUT).returncode
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
                code = subprocess.run(['node', str(CHECKER), '--case', case, '--cook', str(COOK), '--web-root', str(WEB),
                                       '--assets-source', str(source), '--output', str(run / name)],
                                      cwd=REPO, stdout=log, stderr=subprocess.STDOUT).returncode
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
                if (not phase or any(not close(phase['requested'][k], want[k], 1e-9) for k in want)
                        or not (close(got['hours'], want['hours'], 0.0051) and close(got['yaw'], want['yaw'], 0.5001)
                                and close(got['pitch'], want['pitch'], 0.5001))):
                    failures.append(f'Chrome {case} pose drift at {stop}: {phase and phase["requested"]} / {got} vs {want}')
        if failures:
            raise Failure('pose drift')

        # Compare matched stops.
        native_grids = {stop: grid(native / NATIVE_FILE[stop]) for stop in CHROME_PHASE}
        statistics = report['statistics'] = {}
        for stop, phase_name in CHROME_PHASE.items():
            statistics[stop] = {
                'valley': difference(grid(run / 'chrome-valley' / f'valley-{phase_name}.png'), native_grids[stop]),
                'diagnostic': difference(grid(run / 'chrome-diagnostic' / f'sky-reflections-{phase_name}.png'), native_grids[stop]),
            }
        edit = difference(grid(run / 'chrome-valley/valley-before-edit-final.png'), grid(run / 'chrome-valley/valley-edited-final.png'))
        report['positive_control'] = edit
        worst = [max(values['valley'][i] for values in statistics.values()) for i in range(3)]
        rule = [max(1.0, 3 * value) for value in worst]
        report['calibration'] = {'worst_valley': worst, 'rule_thresholds': rule, 'fixed_thresholds': THRESHOLDS}
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
    except Failure as error:
        failures.append(str(error))
    report['passed'] = not failures
    write_json(run / 'run.json', report)
    for stop, values in report.get('statistics', {}).items():
        print(f'{stop:>14}  valley {" ".join(f"{v:7.3f}" for v in values["valley"])}  '
              f'diagnostic {" ".join(f"{v:7.2f}" for v in values["diagnostic"])}')
    if 'thresholds' in report:
        print(f'thresholds {report["thresholds"]}  positive control {report["positive_control"]}')
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
    commands.choices['chrome'].add_argument('--no-open', action='store_true', help='print the URL instead of opening Chrome')
    args = parser.parse_args()
    try:
        {'mac': mac, 'chrome': chrome, 'check': check}[args.command](args)
    except Failure as error:
        print(f'FAIL: {error}', file=sys.stderr)
        raise SystemExit(1)


if __name__ == '__main__':
    main()
