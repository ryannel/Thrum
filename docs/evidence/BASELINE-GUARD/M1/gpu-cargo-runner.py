#!/usr/bin/env python3
"""Serialize native Cargo tests while allowing the CPU-only xtask dispatcher.

Set CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER='python3 <absolute path to this file>'
for agent_loop check/land. The verifier item's own gpu lock remains unchanged.
"""
from pathlib import Path
import os
import sys
loop='/Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.1/skills/orchestration/build/scripts/agent_loop.py'
args=sys.argv[1:]
if Path(args[0]).name == 'xtask':
    os.execv(args[0],args)
os.execvp('python3',['python3',loop,'lock','gpu','--',*args])
