import json,shutil,sys,hashlib
from pathlib import Path
root=Path('tmp/g3t-m3-mf1').resolve();recipe=sys.argv[1];assert recipe in ['portable','apple-tile']
bundle=root/'ThrumM2.app'
if not bundle.exists():shutil.copytree(Path('tmp/g3t-m2/attended/ThrumM2.app'),bundle)
shutil.copy2(root/'bin/v1-game',bundle/'Contents/MacOS/v1-game')
out=root/'interactive';out.mkdir(exist_ok=True)
controls=out/(recipe+'-controls.json');shutil.copy2(Path('examples/v1-game/params/showcase-noon.params.json'),controls)
launcher=bundle/'Contents/MacOS/Launcher'
launcher.write_text(f'''#!/bin/sh
cd /Users/ryannel/Workspace/thrum/.claude/worktrees/apple-tile-m0
export THRUM_RECIPE={recipe} THRUM_SCENE=valley THRUM_ROUTE_OVERLAY=proof THRUM_STARS=on THRUM_CULLING=direct THRUM_V1_FRAMES=100000 THRUM_REPORT=1
export THRUM_ASSET_STORE=/Users/ryannel/Workspace/thrum/.claude/worktrees/apple-tile-m0/tmp/g3t-m0/valley
export THRUM_PARAMS={controls}
exec "{bundle}/Contents/MacOS/v1-game" > "{out}/{recipe}-game.log" 2>&1
''');launcher.chmod(0o755)
(out/(recipe+'-build.json')).write_text(json.dumps({'recipe':recipe,'bundle':str(bundle),'binary_sha256':hashlib.sha256((bundle/'Contents/MacOS/v1-game').read_bytes()).hexdigest(),'scope':'Existing game test-host bundle identity; original M2 bundle is preserved. Current M3 binary and controls.'},indent=2)+'\n')
print(bundle)
