"""CPU-only complete 1x movie decode; this is an observer trace, not display cadence."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('campaign',type=Path)
parser.add_argument('output',type=Path)
args = parser.parse_args()
campaign = json.loads((args.campaign/'campaign.json').read_text())
assert campaign['complete']
args.output.mkdir(parents=True,exist_ok=False)
report = {'complete':False,'scope':'CPU software decode with real-time input pacing at 1x; no GPU, compositor or photon claim',
          'ffmpeg':subprocess.check_output(['ffmpeg','-version'],text=True).splitlines()[0],'arms':[]}
try:
    for arm in campaign['arms']:
        name = arm['case']+'-'+arm['reconstruction']
        session = args.campaign/name
        if campaign['target']=='mac-metal':session /= 'session'
        movie = session/'preview-1x.mp4'
        probe = json.loads(subprocess.check_output(['ffprobe','-v','error','-show_entries',
            'stream=width,height,nb_frames,duration,avg_frame_rate','-select_streams','v:0','-of','json',str(movie)],text=True))['streams'][0]
        hashes = args.output/(name+'.framemd5')
        command = ['ffmpeg','-nostdin','-hide_banner','-loglevel','info','-re','-readrate_initial_burst','0',
                   '-readrate_catchup','1','-hwaccel','none','-threads','4',
                   '-i',str(movie),'-map','0:v:0','-an','-fps_mode','passthrough','-f','framemd5',str(hashes)]
        start = time.monotonic()
        with (args.output/(name+'.log')).open('w') as log:
            subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=60)
        elapsed = time.monotonic()-start
        raw = hashes.read_bytes()
        frames = sum(bool(line.strip()) and not line.startswith('#') for line in raw.decode().splitlines())
        expected = int(probe['nb_frames'])
        assert frames==expected,(name,frames,expected)
        (hashes.with_suffix(hashes.suffix+'.gz')).write_bytes(gzip.compress(raw,mtime=0))
        hashes.unlink()
        report['arms'].append({'name':name,'movie_sha256':hashlib.sha256(movie.read_bytes()).hexdigest(),
            'probe':probe,'command':command,'input_rate':1,'wall_seconds':elapsed,'decoded_frames':frames,
            'all_movie_frames_decoded':True,'terminal_duplicate_is_submission':False})
    report['complete'] = True
finally:
    (args.output/'run.json').write_text(json.dumps(report,indent=2)+'\n')
