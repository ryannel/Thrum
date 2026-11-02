from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
import argparse, json, hashlib, mimetypes
ROOT=Path(__file__).resolve().parent
class Handler(SimpleHTTPRequestHandler):
    def __init__(self,*a,**kw): super().__init__(*a,directory=str(ROOT),**kw)
    def end_headers(self):
        self.send_header('Cache-Control','no-store')
        self.send_header('Cross-Origin-Opener-Policy','same-origin')
        self.send_header('Cross-Origin-Embedder-Policy','require-corp')
        super().end_headers()
    def do_GET(self):
        payloads={str(p.relative_to(ROOT)):p.read_bytes() for p in sorted(ROOT.rglob('*')) if p.is_file() and p.suffix in ('.js','.wgsl','.html','.css')}
        files={name:hashlib.sha256(data).hexdigest() for name,data in payloads.items()}
        identity={'files':files,'build':hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest()[:16]}
        current=identity['build']
        if self.path in ('/','/index.html'):
            self.send_response(307);self.send_header('Location',f'/build/{current}/index.html');self.end_headers();return
        if self.path.startswith('/build/'):
            parts=self.path.split('/',3)
            if len(parts)!=4: self.send_error(404);return
            requested,relative=parts[2:]
            if requested!=current:
                if relative=='index.html':
                    self.send_response(307);self.send_header('Location','/');self.end_headers()
                else: self.send_error(409,'Build changed; reload the proof')
                return
            self.path='/'+relative
        else:
            self.send_error(404);return
        if self.path=='/identity.json':
            data=json.dumps(identity).encode()
            self.send_response(200);self.send_header('Content-Type','application/json');self.end_headers();self.wfile.write(data)
        else:
            name=self.path.lstrip('/')
            if name not in payloads: self.send_error(404);return
            # Serve the exact bytes hashed above; never reopen a live file here.
            data=payloads[name]
            self.send_response(200)
            self.send_header('Content-Type','text/javascript' if name.endswith('.js') else 'text/plain' if name.endswith('.wgsl') else mimetypes.guess_type(name)[0] or 'application/octet-stream')
            self.send_header('Content-Length',str(len(data)))
            self.end_headers();self.wfile.write(data)
    def do_POST(self):
        if self.path not in ('/receipt','/capture','/motion'):
            self.send_error(404);return
        size=int(self.headers.get('Content-Length','0'))
        if size>12*1024*1024: self.send_error(413);return
        data=self.rfile.read(size)
        dst=ROOT/'captures';dst.mkdir(exist_ok=True)
        if self.path=='/receipt':
            record=json.loads(data);name=record.get('name','latest')
            if not name.replace('-','').replace('_','').isalnum(): self.send_error(400);return
            (dst/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
        else:
            name=self.headers.get('X-Capture-Name','latest')
            if not name.replace('-','').replace('_','').isalnum(): self.send_error(400);return
            (dst/(name+('.webm' if self.path=='/motion' else '.png'))).write_bytes(data)
        self.send_response(200);self.end_headers();self.wfile.write(b'ok')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--port',type=int,default=8146);args=p.parse_args()
    print(f'Cloud proof http://127.0.0.1:{args.port}/',flush=True)
    ThreadingHTTPServer(('127.0.0.1',args.port),Handler).serve_forever()
