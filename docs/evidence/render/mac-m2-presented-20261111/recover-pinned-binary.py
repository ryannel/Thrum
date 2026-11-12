import gzip, hashlib, json, os, subprocess, time
from pathlib import Path
root=Path.cwd(); out=root/'tmp/mac-m2/recover-pinned-20261112'; out.mkdir(exist_ok=False)
manifest=json.load(gzip.open(root/'docs/evidence/render/mac-m2-20261110/manifest.json.gz','rt'))
files=manifest['build_source_files']; saved={}; source={}
for rel,digest in files.items():
    data=subprocess.check_output(['git','show','10287802:'+rel])
    assert hashlib.sha256(data).hexdigest()==digest, rel
    source[rel]=data
    p=root/rel
    saved[rel]=p.read_bytes() if p.exists() else None
    if saved[rel] is not None:
        backup=out/'current-source'/rel; backup.parent.mkdir(parents=True,exist_ok=True); backup.write_bytes(saved[rel])
(out/'restore.json').write_text(json.dumps({'head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'files':{p:hashlib.sha256(data).hexdigest() if data is not None else None for p,data in saved.items()}},indent=2)+'\n')
result={'source_commit':'102878022239bedc5c4e0894cfd860909921b5a9','recorded_source_files':len(files),'all_recorded_source_files_match':True,'expected_binary_sha256':manifest['binary_sha256'],'command':['cargo','build','--locked','--offline','--release','-p','v1-game','--bin','v1-game'],'started_unix_ns':time.time_ns()}
try:
    for rel,data in source.items():
        p=root/rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(data)
    env=dict(os.environ); env['RUSTC_WRAPPER']=''
    with (out/'build.log').open('wb') as log:
        result['build_exit']=subprocess.run(result['command'],cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT).returncode
    binary=root/'target/release/v1-game'
    if result['build_exit']==0:
        data=binary.read_bytes(); result['actual_binary_sha256']=hashlib.sha256(data).hexdigest()
        pin=out/'v1-game'; pin.write_bytes(data); pin.chmod(0o755)
        result['binary_pin']=str(pin.relative_to(root)); result['exact_binary_recovered']=result['actual_binary_sha256']==result['expected_binary_sha256']
finally:
    for rel,data in saved.items():
        p=root/rel
        if data is None: p.unlink(missing_ok=True)
        else: p.write_bytes(data)
    result['current_sources_restored']=all((root/rel).read_bytes()==data if data is not None else not (root/rel).exists() for rel,data in saved.items())
    result['finished_unix_ns']=time.time_ns()
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result),flush=True)
