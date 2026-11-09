"""Before/after qualification. Only caller/witness overlays enter archived sources."""
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

from cooked_draw_fixture import sources, cook
from cooked_draw_parity import ROOT, VIEWS, digest, png


def prepare(out, base, env, target):
    first = subprocess.check_output(['git', 'log', '--format=%H', '--max-count=1', '--grep=^Checkpoint cooked direct fallback'], text=True).strip()
    expected = subprocess.check_output(['git', 'rev-parse', first+'^'], text=True).strip()
    base = subprocess.check_output(['git', 'rev-parse', base], text=True).strip()
    if base != expected:
        raise RuntimeError(f'BASE must be the parent of the first implementation: {expected}')
    report = {'target': target, 'base': base, 'claim': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'diff_sha256': __import__('hashlib').sha256(subprocess.check_output(['git', 'diff', 'HEAD'])).hexdigest(),
              'pass': False, 'views': [], 'builds': {}}
    source, store = out / 'source', out / 'store'
    sources(source, env)
    cook(source, store, env)
    report['content_hashes'] = {str(p.relative_to(store)): digest(p) for p in store.rglob('*') if p.is_file()}
    trees = {}
    for arm in ['base', 'claim']:
        tree = out / (arm + '-source')
        tree.mkdir()
        if arm == 'base':
            data = subprocess.check_output(['git', 'archive', base])
            with tarfile.open(fileobj=io.BytesIO(data)) as archive:
                archive.extractall(tree, filter='data')
        else:
            for name in subprocess.check_output(['git', 'ls-files', '-co', '--exclude-standard', '-z']).decode().split('\0'):
                if name and (ROOT / name).is_file():
                    dest = tree / name
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(ROOT / name, dest)
        trees[arm] = tree
        report['builds'][arm] = {'source_hashes': {str(p.relative_to(tree)): digest(p) for root in ['crates', 'examples'] for p in (tree/root).rglob('*') if p.suffix in ['.rs','.wgsl','.toml']}}
    return report, trees, store


def native(out, base, env):
    report, trees, store = prepare(out, base, env, 'mac-metal')
    try:
        for arm, tree in trees.items():
            source = (ROOT/'scripts/cooked_draw_indirect_probe.rs.in').read_text()
            witness = ('let mut report = report; report["path"] = serde_json::json!(format!("{:?}", stats.draw_path)); report["flags"] = serde_json::json!(format!("{:?}", capabilities.downlevel()));' if arm == 'claim' else 'let mut report = report; report["path"] = serde_json::json!("pre-change always-indirect");')
            source = source.replace('// CURRENT_WITNESS', witness)
            probe = tree/'examples/v1-game/src/bin/cooked_indirect_probe.rs'
            probe.write_text(source)
            fixture = probe.parent/'cooked_indirect_fixture/fixture.rs'
            fixture.parent.mkdir()
            shutil.copyfile(ROOT/'examples/v1-game/src/cooked.rs', fixture)
            report['builds'][arm]['caller_overlays'] = {str(p.relative_to(tree)): digest(p) for p in [probe,fixture]}
            build_target = out/(arm+'-target')
            subprocess.run(['cargo','build','--locked','--offline','-p','v1-game','--bin','cooked_indirect_probe'], cwd=tree, env=dict(env,CARGO_TARGET_DIR=str(build_target)),check=True)
            binary = out/(arm+'-probe')
            shutil.copyfile(build_target/'debug/cooked_indirect_probe', binary)
            binary.chmod(0o755)
            report['builds'][arm]['binary_sha256'] = digest(binary)
        for name, scene, yaw, solar in VIEWS:
            captures = {}
            for arm in ['base','claim','claim-repeat','base-repeat']:
                build=arm.split('-')[0]
                folder=out/name/arm
                run_env=dict(env,THRUM_SCENE=scene,THRUM_ASSET_STORE=str(store),THRUM_STARS='off',COOKED_DRAW_OUTPUT=str(folder),COOKED_DRAW_YAW=str(yaw),COOKED_DRAW_SOLAR=str(solar))
                subprocess.run([str(out/(build+'-probe'))], env=run_env, check=True)
                raw=(folder/'output.rgba').read_bytes()
                png(folder/'output.png',raw)
                row=json.loads((folder/'report.json').read_text())
                if row['drawn'] != 1 or row['visible'] is None or 'INDIRECT_EXECUTION' not in row['adapter_flags']:
                    raise RuntimeError(f'not drawn through indirect: {row}')
                if build == 'claim' and row['path'] != 'Some(Indirect)':
                    raise RuntimeError(f'wrong claim path: {row}')
                captures[arm]=(raw,row)
            exact=all(raw==captures['base'][0] for raw,_ in captures.values())
            counts=all(row['visible']==captures['base'][1]['visible'] for _,row in captures.values())
            report['views'].append({'view':[name,scene,yaw,solar],'exact_all_arms':exact,'counts_equal':counts,'arms':{arm:dict(row,rgba_sha256=digest(out/name/arm/'output.rgba')) for arm,(_,row) in captures.items()}})
            print(name,'base/claim/repeats exact',exact,'counts equal',counts,flush=True)
            if not exact or not counts:
                raise RuntimeError(f'before/after indirect mismatch: {name}')
        report['pass']=True
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')


def browser(out, base, env):
    report, trees, store = prepare(out, base, env, 'chrome-webgpu')
    try:
        for arm, tree in trees.items():
            if arm == 'base':
                file=tree/'examples/v1-game/src/browser.rs'
                text=file.read_text().replace('"drawn": stats.drawn,', '"drawn": stats.drawn, "drawPath":"pre-change always-indirect", "initialDownlevel":"pre-change unavailable", "visibleMeshlets":stats.visible_meshlets,')
                file.write_text(text)
                report['builds'][arm]['witness_overlay']={str(file.relative_to(tree)):digest(file)}
            build_target=out/(arm+'-target')
            subprocess.run(['sh','examples/v1-game/web/build.sh'],cwd=tree,env=dict(env,CARGO_TARGET_DIR=str(build_target)),check=True)
            report['builds'][arm]['artifacts']={p.name:digest(p) for p in (build_target/'v1-game-web').iterdir() if p.suffix in ['.js','.wasm']}
        subprocess.run(['node','scripts/cooked_draw_browser.mjs',str(out),str(store)],env=env,check=True)
        result=json.loads((out/'browser.json').read_text())
        report['browser']=result
        report['pass']=result['pass']
    finally:
        (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
