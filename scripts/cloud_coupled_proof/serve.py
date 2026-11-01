"""Loopback Q2/Q3 proof. Static owned inputs and bounded local evidence capture."""
from http.server import SimpleHTTPRequestHandler,ThreadingHTTPServer
from pathlib import Path
import re
ROOT=Path(__file__).resolve().parents[2]
CAPTURES=ROOT/'tmp/cloud-coupled-proof/captures'
class Handler(SimpleHTTPRequestHandler):
    def __init__(self,*args,**kwargs):super().__init__(*args,directory=str(ROOT),**kwargs)
    def end_headers(self):self.send_header('Cache-Control','no-store');super().end_headers()
    def do_GET(self):
        if not self.path.startswith(('/scripts/cloud_coupled_proof/','/scripts/cloud_source_proof/','/docs/evidence/cloud-source-2026-11-01/','/docs/evidence/cloud-coupled-2026-11-01/','/tmp/cloud-coupled-proof/')):self.send_error(404);return
        super().do_GET()
    def do_POST(self):
        name=self.headers.get('X-Capture-Name','')
        try:length=int(self.headers.get('Content-Length','0'))
        except ValueError:self.send_error(400);return
        if self.path!='/capture' or not re.fullmatch(r'[A-Za-z0-9_.-]{1,120}',name) or not 0<length<=80*1024**2:self.send_error(400);return
        CAPTURES.mkdir(parents=True,exist_ok=True);target=CAPTURES/name
        if target.exists():self.send_error(409);return
        data=self.rfile.read(length)
        if len(data)!=length:self.send_error(400);return
        target.write_bytes(data);self.send_response(201);self.end_headers();self.wfile.write(str(target.relative_to(ROOT)).encode())
if __name__=='__main__':
    print('http://localhost:8142/scripts/cloud_coupled_proof/',flush=True)
    ThreadingHTTPServer(('127.0.0.1',8142),Handler).serve_forever()
