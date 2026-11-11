#!/usr/bin/env python3
"""Run the 1.7.1 milestone checker with GPU locking on its profile pass too."""
import importlib.util
from pathlib import Path
import sys

LOOP = Path('/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py')


def main():
    if sys.argv[1:2] != ['check']:
        raise SystemExit('this adapter accepts the check command only')
    spec = importlib.util.spec_from_file_location('locked_milestone_checker', LOOP)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    check = module.try_check

    def locked_check(tree, command, lock, timeout_s):
        resource = lock or 'gpu'
        return {**check(tree, command, resource, timeout_s), 'lock': resource}

    module.try_check = locked_check
    module.main()


if __name__ == '__main__':
    main()
