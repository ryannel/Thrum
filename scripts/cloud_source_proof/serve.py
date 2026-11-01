"""Loopback-only cloud proof server; captures stay in the owned scratch tree."""
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
CAPTURES = ROOT/'tmp/cloud-source-proof/captures'


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def end_headers(self):
        self.send_header('Cache-Control','no-store')
        super().end_headers()

    def do_GET(self):
        if not self.path.startswith(('/scripts/cloud_source_proof/','/tmp/cloud-source-proof/')):
            self.send_error(404)
            return
        super().do_GET()

    def do_POST(self):
        name = self.headers.get('X-Capture-Name','')
        length = int(self.headers.get('Content-Length','0'))
        if self.path != '/capture' or not re.fullmatch(r'[A-Za-z0-9_.-]{1,100}',name) or not 0 < length <= 50*1024**2:
            self.send_error(400)
            return
        CAPTURES.mkdir(parents=True,exist_ok=True)
        target = CAPTURES/name
        if target.exists():
            self.send_error(409,'Capture already exists')
            return
        data=self.rfile.read(length)
        if len(data)!=length:
            self.send_error(400,'Truncated capture')
            return
        target.write_bytes(data)
        self.send_response(201)
        self.end_headers()
        self.wfile.write(str(target.relative_to(ROOT)).encode())


if __name__=='__main__':
    print('Cloud proof: http://localhost:8141/scripts/cloud_source_proof/viewer.html',flush=True)
    ThreadingHTTPServer(('127.0.0.1',8141),Handler).serve_forever()
