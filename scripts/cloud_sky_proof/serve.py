"""Loopback cloud-sky proof host with one immutable startup build and owned captures."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import hashlib
import json
import mimetypes
import re
from urllib.parse import urlsplit, unquote

ROOT = Path(__file__).resolve().parents[2]
CAPTURES = ROOT / 'tmp/cloud-sky-2026-11-01/captures'
OWNED = ROOT / 'scripts/cloud_sky_proof'
SOURCES = [p for p in OWNED.rglob('*') if p.is_file() and p.suffix in {'.html', '.js', '.wgsl', '.json', '.py', '.mjs', '.f32'}]
SOURCES += [ROOT/'scripts/cloud_coupled_proof/source.js', ROOT/'scripts/cloud_source_proof/source.js']
SOURCES += [ROOT/'docs/evidence/cloud-source-2026-11-01/selected-clip'/name for name in ['cloudclip.json','density.bin']]
SNAPSHOT = {p.relative_to(ROOT).as_posix(): p.read_bytes() for p in SOURCES}
HASHES = {name: hashlib.sha256(data).hexdigest() for name, data in SNAPSHOT.items()}
BUILD_ID = hashlib.sha256(json.dumps(HASHES, sort_keys=True).encode()).hexdigest()
IDENTITY = json.dumps({'build_id': BUILD_ID, 'files': HASHES}, indent=2).encode()

class Handler(BaseHTTPRequestHandler):
    def send_bytes(self, body, content_type):
        self.send_response(200)
        self.send_header('Content-Type', content_type)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = unquote(urlsplit(self.path).path).lstrip('/')
        if path == 'identity':
            self.send_bytes(IDENTITY, 'application/json')
            return
        if path in SNAPSHOT:
            body = SNAPSHOT[path]
            if path.endswith('.html'):
                body = body.replace(b'<title>', f'<meta name="proof-build" content="{BUILD_ID}"><title>'.encode(), 1)
            self.send_bytes(body, mimetypes.guess_type(path)[0] or 'application/octet-stream')
            return
        if path.startswith('tmp/cloud-sky-2026-11-01/captures/'):
            target = (ROOT/path).resolve()
            if target.is_relative_to(CAPTURES.resolve()) and target.is_file():
                self.send_bytes(target.read_bytes(), mimetypes.guess_type(target)[0] or 'application/octet-stream')
                return
        self.send_error(404)

    def do_POST(self):
        name = self.headers.get('X-Capture-Name', '')
        try:
            length = int(self.headers.get('Content-Length', '0'))
        except ValueError:
            self.send_error(400)
            return
        if self.path != '/capture' or not re.fullmatch(r'[A-Za-z0-9_.-]{1,120}', name) or not 0 < length <= 80*1024**2:
            self.send_error(400)
            return
        CAPTURES.mkdir(parents=True, exist_ok=True)
        target = CAPTURES/name
        if target.exists():
            self.send_error(409)
            return
        data = self.rfile.read(length)
        if len(data) != length:
            self.send_error(400)
            return
        target.write_bytes(data)
        self.send_response(201)
        self.end_headers()
        self.wfile.write(str(target.relative_to(ROOT)).encode())

if __name__ == '__main__':
    print(f'Frozen build {BUILD_ID}: http://localhost:8143/scripts/cloud_sky_proof/packing.html', flush=True)
    ThreadingHTTPServer(('127.0.0.1', 8143), Handler).serve_forever()
