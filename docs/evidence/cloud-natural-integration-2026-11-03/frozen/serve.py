from http.server import ThreadingHTTPServer,SimpleHTTPRequestHandler
from pathlib import Path
import argparse,hashlib,json,mimetypes,threading
ROOT=Path(__file__).resolve().parent
LOCK=threading.Lock();CACHE={}
def payload():
    paths=sorted(p for p in ROOT.rglob('*') if p.is_file() and (str(p.relative_to(ROOT)) in ('index.html','style.css') or (p.relative_to(ROOT).parts[0]=='src' and p.suffix=='.js') or (p.relative_to(ROOT).parts[0]=='shaders' and p.suffix=='.wgsl') or (p.relative_to(ROOT).parts[0]=='assets' and p.suffix in ('.r16f','.json'))))
    stamp=tuple((str(p.relative_to(ROOT)),p.stat().st_mtime_ns,p.stat().st_size) for p in paths)
    with LOCK:
        if CACHE.get('stamp')!=stamp:
            data={str(p.relative_to(ROOT)):p.read_bytes() for p in paths};files={k:hashlib.sha256(v).hexdigest() for k,v in data.items()}
            CACHE.update(stamp=stamp,data=data,identity={'build':hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest()[:16],'files':files})
        return CACHE['data'],CACHE['identity']
class Handler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control','no-store');self.send_header('Cross-Origin-Opener-Policy','same-origin');self.send_header('Cross-Origin-Embedder-Policy','require-corp');super().end_headers()
    def do_GET(self):
        data,identity=payload()
        if self.path in ('/','/index.html'):
            self.send_response(307);self.send_header('Location',f"/build/{identity['build']}/index.html");self.end_headers();return
        parts=self.path.split('/',3)
        if len(parts)!=4 or parts[1]!='build':self.send_error(404);return
        if parts[2]!=identity['build']:self.send_error(409,'Frozen proof changed; reload root');return
        name=parts[3]
        if name=='identity.json':body=json.dumps(identity).encode();kind='application/json'
        elif name in data:body=data[name];kind='text/javascript' if name.endswith('.js') else mimetypes.guess_type(name)[0] or 'application/octet-stream'
        else:self.send_error(404);return
        self.send_response(200);self.send_header('Content-Type',kind);self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
    def do_POST(self):
        if self.path not in ('/receipt','/capture','/motion'):self.send_error(404);return
        size=int(self.headers.get('Content-Length','0'))
        if size>12*1024**2:self.send_error(413);return
        if sum(p.stat().st_size for p in ROOT.rglob('*') if p.is_file())+size>512*1024**2:self.send_error(413,'Owned512MiBbudgetexceeded');return
        body=self.rfile.read(size)
        if self.path=='/receipt':record=json.loads(body);name=record.get('name','');suffix='.json';body=(json.dumps(record,indent=2)+'\n').encode()
        else:name=self.headers.get('X-Capture-Name','');suffix='.webm' if self.path=='/motion' else '.png'
        if not name.replace('-','').replace('_','').isalnum():self.send_error(400);return
        dst=ROOT/'captures';dst.mkdir(exist_ok=True);(dst/(name+suffix)).write_bytes(body)
        self.send_response(200);self.end_headers();self.wfile.write(b'ok')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--port',type=int,default=8158);args=p.parse_args();print(f'IntegratedCloudCPUProof http://127.0.0.1:{args.port}/',flush=True);ThreadingHTTPServer(('127.0.0.1',args.port),Handler).serve_forever()
