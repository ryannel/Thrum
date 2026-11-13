"""Bounded local native-size movie review, run under the GPU lock."""
import functools
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import sys
import time

sys.path.insert(0, 'scripts')
from qualify_mac_m2 import panel_witness

root = Path('tmp/mac-m2/current-20261113').resolve()
observe = panel_witness(root)
stopped = False

class Handler(SimpleHTTPRequestHandler):
    def do_POST(self):
        global stopped
        if self.path == '/release':
            stopped = True
        elif self.path == '/report':
            report = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
            if report['arm'] not in ('motion', 'on-untraced', 'on-traced', 'grid-replacement', 'valley-replacement'):
                self.send_error(400)
                return
            (root / ('playback-' + report['arm'] + '.json')).write_text(json.dumps(report, indent=2) + '\n')
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.end_headers()

page = '''<!doctype html><title>Current Mac M2 motion review</title>
<style>body{margin:0;background:#171717;color:white;font:16px sans-serif}video{width:1920px;height:1080px}pre{white-space:pre-wrap}</style>
<label>Sequence <select id="arm">
<option value="motion">Offscreen elapsed-time pan</option>
<option value="on-untraced">Window ON untraced</option>
<option value="on-traced">Window ON traced</option>
<option value="grid-replacement">Window pattern replacement</option>
<option value="valley-replacement">Window valley replacement</option>
</select></label> <button id="play">Play complete sequence at 1×</button>
<button onclick="fetch('/release',{method:'POST'})">Finish review</button><pre id="status">Ready</pre>
<video id="movie" controls preload="metadata"></video>
<script>
const movie=document.querySelector('video'), status=document.querySelector('#status'), selector=document.querySelector('#arm');
let frames=0, started=0, selected='motion';
const paths={motion:'motion/session/preview-1x.mp4','on-untraced':'presented/on-untraced/preview-1x.mp4','on-traced':'presented/on-traced/preview-1x.mp4','grid-replacement':'window/grid-replacement/preview-1x.mp4','valley-replacement':'window/valley-replacement/preview-1x.mp4'};
function track(){frames++;if(!movie.ended&&!movie.paused)movie.requestVideoFrameCallback(track)}
document.querySelector('#play').onclick=()=>{selected=selector.value;movie.src=paths[selected];movie.playbackRate=1;frames=0;started=performance.now();movie.play();movie.requestVideoFrameCallback(track);status.textContent='Playing '+selected+' at 1×'};
movie.onended=()=>{const report={arm:selected,rate:movie.playbackRate,duration_s:movie.duration,wall_ms:performance.now()-started,callbacks:frames,quality:movie.getVideoPlaybackQuality().toJSON?.()??{totalVideoFrames:movie.getVideoPlaybackQuality().totalVideoFrames,droppedVideoFrames:movie.getVideoPlaybackQuality().droppedVideoFrames},width:movie.videoWidth,height:movie.videoHeight};status.textContent=JSON.stringify(report);fetch('/report',{method:'POST',body:JSON.stringify(report)})};
</script>'''
(root / 'review.html').write_text(page)
server = ThreadingHTTPServer(('127.0.0.1', 8842), functools.partial(Handler, directory=str(root)))
server.timeout = .5
deadline = time.monotonic() + 600
print('REVIEW_READY http://127.0.0.1:8842/review.html', flush=True)
try:
    while not stopped and time.monotonic() < deadline:
        observe()
        server.handle_request()
finally:
    server.server_close()
print('REVIEW_RELEASED', flush=True)
