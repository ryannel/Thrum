"""Acquire one official static cloud pack, retaining only two selected VDBs."""
import datetime,hashlib,html,json,pathlib,re,urllib.request,zipfile
ROOT=pathlib.Path(__file__).resolve().parent
PAGE='https://jangafx.com/software/embergen/download/free-vdb-animations'
DOWNLOAD='https://www.mediafire.com/file/879onggscuon7ew/CloudPackVDB.zip/file'
EXPECTED=109761538
MEMBERS=['LICENSE.txt','README.txt','CloudPack/cloud_pack_thumbnails.png','CloudPack/CloudPackVDB/cloud_03_variant_0000.vdb','CloudPack/CloudPackVDB/cloud_08_variant_0000.vdb']
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1048576),b''):h.update(b)
 return h.hexdigest()
start=datetime.datetime.now(datetime.timezone.utc).isoformat()
archive=ROOT/'CloudPackVDB.zip'
with urllib.request.urlopen(urllib.request.Request(DOWNLOAD,headers={'User-Agent':'Mozilla/5.0'}),timeout=30) as r:doc=r.read(2000000).decode('utf8','replace')
m=re.search(r'href=["\']([^"\']+)["\'][^>]*id=["\']downloadButton',doc) or re.search(r'id=["\']downloadButton["\'][^>]*href=["\']([^"\']+)',doc)
if not m:raise RuntimeError('Official linked download button not found')
url=html.unescape(m.group(1));total=0
if not archive.exists():
 with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'Mozilla/5.0'}),timeout=60) as r,(ROOT/'CloudPackVDB.zip.partial').open('wb') as f:
  for b in iter(lambda:r.read(1048576),b''):
   total+=len(b)
   if total>EXPECTED:raise RuntimeError('Unexpected archive size; stopped bounded acquisition')
   f.write(b)
 if total!=EXPECTED:raise RuntimeError(f'Archive byte mismatch {total}')
 (ROOT/'CloudPackVDB.zip.partial').rename(archive)
if archive.stat().st_size!=EXPECTED:raise RuntimeError('Unexpected retained archive size')
with zipfile.ZipFile(archive) as z:
 directory=[{'name':i.filename,'bytes':i.file_size,'compressed_bytes':i.compress_size,'crc32':f'{i.CRC:08x}'} for i in z.infolist()]
 if sum(z.getinfo(n).file_size for n in MEMBERS)>50*1024**2:raise RuntimeError('Selected extraction exceeds50MiB')
 files=[]
 for n in MEMBERS:
  target=ROOT/n;target.parent.mkdir(parents=True,exist_ok=True)
  target.write_bytes(z.read(n));files.append({'archive_member':n,'bytes':target.stat().st_size,'sha256':sha(target)})
meta={'schema':'thrum.static-cloud-master-acquisition.v1','package_id':'thrum.cloud-master.jangafx-03-08.v1','official_license_page':PAGE,'linked_archive_page':DOWNLOAD,'transfer_url':url,'download_started_utc':start,'completed_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'license':'CC0-1.0; exact LICENSE.txt retained','authorship':'Created with EmberGen by JangaFX LLC','archive':{'file':archive.name,'bytes':archive.stat().st_size,'sha256':sha(archive)},'archive_uncompressed_bytes':sum(x['bytes'] for x in directory),'directory':directory,'selected':files,'selection_basis':'Official thumbnail:03 asymmetric tallCu with medium flank relief;08 compact irregular Cu; actual density still requires inspection','limits':'Static density ingredients; no complete Cb, runtime weather, cloud-family or lifecycle acceptance'}
(ROOT/'provenance.json').write_text(json.dumps(meta,indent=2)+'\n')
print(json.dumps(meta['archive']),flush=True);print('SELECTED',json.dumps(files),flush=True)
