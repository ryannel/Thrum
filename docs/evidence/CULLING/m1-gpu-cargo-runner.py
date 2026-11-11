#!/usr/bin/env python3
"""Give the profile Check one GPU lease while item checks use their declared locks."""
import os
from pathlib import Path
import sys

LOOP = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py'

args = sys.argv[1:]
if Path(args[0]).name == 'xtask' and args[1:2] == ['check']:
    environment = dict(os.environ)
    environment.pop('CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER', None)
    os.execvpe('python3', ['python3', LOOP, 'lock', 'gpu', '--', *args], environment)
os.execv(args[0], args)
