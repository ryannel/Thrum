"""Run the unmodified M2 checks, also locking GPU for the profile Check."""
import importlib.util
from pathlib import Path
import sys

LOOP=Path('/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py')
spec=importlib.util.spec_from_file_location('mac_m2_agent_loop',LOOP)
loop=importlib.util.module_from_spec(spec)
sys.modules[spec.name]=loop
spec.loader.exec_module(loop)
original_try_check=loop.try_check

def gpu_check(tree,command,lock,timeout_s):
    # Item M2.3 already names gpu; the profile Check otherwise passes lock=None.
    # Retain every command, timeout and exit judgement, strengthening only its lock.
    result=original_try_check(tree,command,lock or 'gpu',timeout_s)
    result['execution_lock']=lock or 'gpu'
    return result

loop.try_check=gpu_check
sys.argv=[str(LOOP),'check','--module','PLATFORM-RUNTIME-MAC','--milestone','M2','--worktree','.', '--timeout','1800','--json']
loop.main()
