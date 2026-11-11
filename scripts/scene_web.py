"""Build, cook, watch and serve a kit scene on loopback. Called by ./dev scene ... --web."""
import functools
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
from scene_package import resolve, workspace_metadata


def main(folder):
    root = Path.cwd()
    package, binary = resolve(root, folder, workspace_metadata(root))
    target = Path(os.environ.get('CARGO_TARGET_DIR', 'target')).resolve()
    scene_output = Path(os.environ.get('THRUM_SCENE_OUTPUT', str(target / 'scenes' / package))).resolve()
    web = scene_output / 'web'
    web.mkdir(parents=True, exist_ok=True)
    cargo = ['rustup', 'run', '1.99.1', 'cargo']
    subprocess.run(cargo + ['build', '--locked', '--offline', '--release', '-p', package,
                           '--bin', binary, '--target', 'wasm32-unknown-unknown'], check=True)
    bindgen = shutil.which('wasm-bindgen') or str(Path.home() / '.cargo/bin/wasm-bindgen')
    subprocess.run([bindgen, '--target', 'web', '--out-name', 'scene', '--out-dir', str(web),
                    str(target / 'wasm32-unknown-unknown/release' / (binary + '.wasm'))], check=True)
    for name in ['index.html', 'scene.js']:
        shutil.copy(root / 'crates/thrum-app/web' / name, web / name.replace('scene.js', 'page.js'))
    shutil.copy(root / 'crates/thrum-controls/web/thrum-controls.js', web)
    for params in (root / folder).glob('*.params.json'):
        shutil.copy(params, web)
    subprocess.run(cargo + ['build', '--locked', '--offline', '--release', '-p', 'thrum-cook'], check=True)
    source = Path(os.environ.get('THRUM_SCENE_ASSETS', str(root / folder / 'assets')))
    store = web.parent / 'store'
    cook = str(target / 'release/thrum-cook')
    subprocess.run([cook, 'cook', str(source), str(store)], check=True)
    watcher = subprocess.Popen([cook, 'watch', str(source), str(store), '--serve', '127.0.0.1:0'],
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    try:
        origin = None
        for line in watcher.stdout:
            print(line, end='', flush=True)
            if 'http://127.0.0.1:' in line:
                origin = line[line.index('http://127.0.0.1:'):].strip().split()[0]
                break
        if origin is None:
            raise RuntimeError('asset watcher exited before serving; fix its error and rerun')
        (web / 'config.json').write_text(json.dumps({'store': origin}))
        handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=str(web))
        with http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler) as server:
            def logs():
                for line in watcher.stdout:
                    print(line, end='', flush=True)
            threading.Thread(target=logs, daemon=True).start()
            print(f'SCENE_URL=http://127.0.0.1:{server.server_port}/', flush=True)
            server.serve_forever()
    finally:
        watcher.terminate()
        watcher.wait()


if __name__ == '__main__':
    try:
        main(sys.argv[1])
    except KeyboardInterrupt:
        pass
