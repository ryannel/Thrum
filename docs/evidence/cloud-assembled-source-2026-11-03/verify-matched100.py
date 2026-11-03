#!/usr/bin/env python3
"""Optional CPU-only original PNG comparison after --with-observations restore. Requires Pillow."""
from pathlib import Path
import argparse,hashlib,json
from PIL import Image
P=Path(__file__).resolve().parent
ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--restored',type=Path,required=True);a=ap.parse_args();r=json.loads((P/'matched100-recovery.json').read_text());old=a.restored/r['oldReference']['restoreDestination'];new=a.restored/r['newImage']
with Image.open(old)as x,Image.open(new)as y:
 x=x.convert('RGBA');y=y.convert('RGBA');xb=x.tobytes();yb=y.tobytes();assert x.size==y.size and xb==yb
 print(json.dumps({'pass':True,'shape':[x.height,x.width,4],'decodedRGBASHA256':hashlib.sha256(xb).hexdigest(),'decodedPixelsExact':True,'scope':'Only matched100pair8km original PNG pixels; noGPU/rerender/performance/artqualification'}))
