#!/usr/bin/env python3
"""Run the unchanged TRANSPORT M1 check commands concurrently through the build loop.

The loop's normal runner owns locks, logs and content-keyed check reuse. This does not
edit the plan, change commands or mark results in DONE.json. Follow with its `check`
command for the official milestone summary and project-wide gate.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--loop",type=Path,required=True)
    parser.add_argument("--items",nargs="+",help="approved item IDs to run; omitted runs all checks and the project gate")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    rustc = Path(subprocess.check_output(["rustup","which","--toolchain","1.99.1","rustc"],text=True).strip())
    os.environ["PATH"] = str(rustc.parent)+os.pathsep+os.environ["PATH"]
    os.environ["RUSTC_WRAPPER"] = ""
    spec = importlib.util.spec_from_file_location("transport_build_loop",args.loop)
    loop = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = loop
    spec.loader.exec_module(loop)
    loop.install_stop()
    document = json.loads((root / "docs/plans/TRANSPORT/DONE.json").read_text())
    milestone, = [row for row in document["milestones"] if row["id"] == "M1"]
    items = [item for item in milestone["items"] if item.get("check")]
    if args.items:
        unknown = set(args.items)-{item["id"] for item in items}
        if unknown:
            parser.error("unknown check IDs: "+", ".join(sorted(unknown)))
        items = [item for item in items if item["id"] in args.items]
    results = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        pending = {pool.submit(loop.try_check,root,item["check"],item.get("lock"),7200,"docs/plans"):item for item in items}
        for future in as_completed(pending):
            item = pending[future]
            result = {"id":item["id"],"check":item["check"],**future.result()}
            results.append(result)
            print(json.dumps(result),flush=True)
    if not args.items:
        result = {"id":"Test command","check":"./dev check",**loop.try_check(root,"./dev check","gpu",7200,"docs/plans")}
        results.append(result)
        print(json.dumps(result),flush=True)
    return int(any(result["result"] != "pass" for result in results))


if __name__ == "__main__":
    raise SystemExit(main())
