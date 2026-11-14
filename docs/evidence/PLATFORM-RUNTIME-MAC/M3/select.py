"""Select native motion sources and unscaled crops; never replace the originals."""
import json
from pathlib import Path
import shutil
import subprocess
import sys

session, output = map(Path, sys.argv[1:])
manifest = json.loads((session / 'manifest.json').read_text())
runtime = json.loads((session / 'runtime.json').read_text())
opportunities = {row['submission_id']: row for row in runtime['opportunities'] if row['submitted']}
output.mkdir(parents=True, exist_ok=True)
selected = []
for seconds in (0, 1.5, 2, 2.5, 3.5):
    row = min(manifest['records'], key=lambda row: abs(row['submit_ns'] / 1e9 - seconds))
    image = session / 'png' / f"{row['submission_id']:06}.png"
    full, crop = output / f'full-{seconds}.png', output / f'crop-{seconds}.png'
    shutil.copy2(image, full)
    subprocess.run(['ffmpeg', '-v', 'error', '-i', str(image), '-vf',
                    'crop=960:540:0:360', '-frames:v', '1', str(crop)], check=True)
    selected.append(dict(row, yaw=opportunities[row['submission_id']]['accepted_yaw_degrees'],
                         full=full.name, crop=crop.name, crop_xywh=[0, 360, 960, 540]))
(output / 'selected.json').write_text(json.dumps(selected, indent=2) + '\n')
