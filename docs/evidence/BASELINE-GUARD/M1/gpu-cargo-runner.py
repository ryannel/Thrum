#!/usr/bin/env python3
"""Serialize the repository Check under one GPU lease.

Set CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER to this Python runner for
agent_loop check. The verifier item takes its own GPU lock. The xtask Check
gets one lease and runs native Cargo tests directly within that lease.
"""
from pathlib import Path
import os
import sys

loop = '/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py'
args = sys.argv[1:]
if Path(args[0]).name == 'xtask':
    if args[1:2] == ['check']:
        environment = dict(os.environ)
        environment.pop('CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER', None)
        os.execvpe('python3', ['python3', loop, 'lock', 'gpu', '--', *args], environment)
    os.execv(args[0], args)
os.execvp('python3', ['python3', loop, 'lock', 'gpu', '--', *args])
