#!/usr/bin/env python3
"""Measure BASE to supply absent quiet-load G5 history, without changing its encoder."""
import argparse
import gzip
import io
import json
from pathlib import Path
import subprocess
import tarfile

from cooked_draw_fixture import environment
from cooked_draw_parity import ROOT, digest


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument("--resume",action="store_true")
    args=parser.parse_args()
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=args.resume)
    base=(ROOT/'docs/evidence/render/cooked-draw/BASE').read_text().strip()
    tree=out/'base-source'
    if not args.resume:
        tree.mkdir()
        with tarfile.open(fileobj=io.BytesIO(subprocess.check_output(['git','archive',base]))) as archive:
            archive.extractall(tree,filter='data')
        overlays={}
        file=tree/'crates/thrum-cook/src/tripwire.rs'
        text=file.read_text();begin=text.index('    let revision =',text.index('pub fn this_run'));end=text.index('    Run {',begin)
        # A git archive has no .git; pin the measured source instead of inheriting its parent's HEAD.
        text=text[:begin]+f'    let revision = "{base}".to_owned();\n'+text[end:];file.write_text(text)
        overlays[str(file.relative_to(tree))]={'sha256':digest(file),'reason':'measurement provenance only: archived baseline revision'}
        file=tree/'examples/v1-game/benches/cooked_edit.rs';text=file.read_text()
        old='let mut host = FrameHost::new(&device, &queue, FrameConfig::default())'
        if old not in text:raise RuntimeError('baseline bench caller changed')
        text=text.replace(old,'''let capabilities = thrum_render::capabilities::DeviceCapabilities::capture(&adapter, &device, 1);
            assert!(adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION));
            let mut host = FrameHost::new_with_capabilities(&device, &queue, &capabilities, FrameConfig::default())''')
        file.write_text(text);overlays[str(file.relative_to(tree))]={'sha256':digest(file),'reason':'adapter-backed caller; baseline contribution is always indirect'}
    else:
        prior=json.loads((out/'report.json').read_text())
        if prior['base']!=base:raise RuntimeError('resume BASE changed')
        overlays=prior['overlays']
        if any(digest(tree/name)!=row['sha256'] for name,row in overlays.items()):
            raise RuntimeError('resumed baseline witness/caller changed')
    histories=['g5-edit-to-visible.tsv','g5-cooked-edit.tsv']
    original={name:(tree/'docs/evidence/integration/tripwires'/name).read_text() for name in histories}
    report={'base':base,'overlays':overlays,'runs':[],'scope':'BASE encoder unchanged; only benchmark caller and revision witness overlays; historical class retained; measurements retain their actual natural load class; quiet runs seed a new class, never reused as a speed claim'}
    if args.resume:report['runs']=prior['runs']
    env=dict(environment(),CARGO_TARGET_DIR=str(out/'target'),THRUM_TRIPWIRES_RECORD='1')
    try:
        start=len(report['runs'])//2
        def quiet_count(name):
            return sum('class=quiet' in row for row in (tree/'docs/evidence/integration/tripwires'/name).read_text().splitlines())
        for index in range(start,start+10):
            if all(quiet_count(name)>=3 for name in histories):break
            for package,bench in [('thrum-cook','edit_to_visible'),('v1-game','cooked_edit')]:
                path=out/f'{index}-{bench}.md'
                result=subprocess.run(['cargo','bench','--locked','--offline','-q','-p',package,'--bench',bench],cwd=tree,env=dict(env,THRUM_G5_BENCH_OUT=str(path)),text=True,capture_output=True)
                (out/f'{index}-{bench}.log').write_text(result.stdout+result.stderr)
                report['runs'].append({'index':index,'bench':bench,'exit':result.returncode,'report':path.name,'sha256':digest(path) if path.exists() else None})
                result.check_returncode()
        if not all(quiet_count(name)>=3 for name in histories):raise RuntimeError('natural quiet class still has fewer than three BASE measurements')
        for name in histories:
            path=tree/'docs/evidence/integration/tripwires'/name
            added=path.read_text().removeprefix(original[name])
            if any(f'rev={base}' not in line for line in added.splitlines() if line):raise RuntimeError('invalid recorded provenance')
            report[name]=added
            with (ROOT/'docs/evidence/integration/tripwires'/name).open('a') as stream:stream.write(added)
        report['pass']=True
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')


if __name__=='__main__':main()
