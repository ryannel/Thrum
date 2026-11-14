"""Qualify SEA's integration source under the caller's shared GPU lock.

The builder saves the proof and commits its evidence through a bounded file
handshake, then the helper invokes agent_loop.py merge under the same GPU lock.
An existing proof is reused only when the repository's gate accepts it.
"""

from pathlib import Path
import json
import shutil
import subprocess
import sys
import time
import uuid


ROOT = Path(__file__).resolve().parents[3]
LOOP = Path.home() / ".claude/plugins/cache/ryannel-skills/orchestrate/2.2.2/skills/orchestration/build/scripts/agent_loop.py"


def run(*command):
    return subprocess.run(command, cwd=ROOT, check=True)


def git_output(*arguments):
    return subprocess.check_output(["git", *arguments], cwd=ROOT, text=True).strip()


def finish_with_evidence():
    sys.path.insert(0, str(ROOT / "scripts"))
    import exact_baselines as baselines

    proof = baselines.proof_path(ROOT)
    report = json.loads(proof.read_text())
    if not report.get("passed") or report["content_key"] != baselines.content_key(ROOT):
        raise RuntimeError("No successful proof for the current covered source")
    scratch = ROOT / "tmp/sea-m2"
    scratch.mkdir(parents=True, exist_ok=True)
    token = uuid.uuid4().hex
    ticket = scratch / f"evidence-commit-{token}.json"
    ready = scratch / "qualification-ready.json"
    retained = ROOT / "docs/evidence/SEA/M2-baselines.json"
    state = {
        "stage": "await_evidence_commit",
        "token": token,
        "proof_file": str(proof.relative_to(ROOT)),
        "retained_proof_file": str(retained.relative_to(ROOT)),
        "ticket_file": str(ticket.relative_to(ROOT)),
        "proof_revision": report["revision"],
        "source_head": git_output("rev-parse", "HEAD"),
        "deadline_unix_seconds": time.time() + 900,
    }
    ready.write_text(json.dumps(state, indent=2) + "\n")
    print(f"AWAIT_EVIDENCE: {ready.relative_to(ROOT)}", flush=True)
    deadline = time.monotonic() + 900
    while not ticket.exists():
        if time.monotonic() >= deadline:
            raise RuntimeError("Bounded evidence-commit wait expired; GPU lock is released")
        time.sleep(0.5)
    acknowledgement = json.loads(ticket.read_text())
    if acknowledgement.get("token") != token or acknowledgement.get("commit") != git_output("rev-parse", "HEAD"):
        raise RuntimeError("Evidence ticket does not name the current committed work")
    if git_output("status", "--porcelain", "--untracked-files=all"):
        raise RuntimeError("Evidence must be committed before the integration gate")
    if json.loads(retained.read_text()) != report:
        raise RuntimeError("The retained baseline record differs from the target proof")
    done = json.loads((ROOT / "docs/plans/SEA/DONE.json").read_text())
    milestone = next(m for m in done["milestones"] if m["id"] == "M2")
    if milestone["status"] != "in_progress" or any(i["builder_result"] != "pass" for i in milestone["items"]):
        raise RuntimeError("M2 must have its builder proofs and remain in_progress")
    run(sys.executable, "scripts/exact_baselines.py", "gate")
    run(sys.executable, str(LOOP), "merge", "--worktree", str(ROOT))
    state.update(stage="merged", merged_commit=git_output("rev-parse", "HEAD"))
    ready.write_text(json.dumps(state, indent=2) + "\n")


def main():
    dirty = subprocess.check_output(
        ["git", "status", "--porcelain", "--untracked-files=all"], cwd=ROOT, text=True
    )
    if dirty:
        raise RuntimeError("Commit this worktree before qualifying its integration source")
    run("git", "fetch", "origin")
    try:
        run("git", "rebase", "origin/main")
    except subprocess.CalledProcessError:
        subprocess.run(["git", "rebase", "--abort"], cwd=ROOT, check=False)
        raise
    run("git", "rev-parse", "HEAD")
    gate = subprocess.run(
        [sys.executable, "scripts/exact_baselines.py", "gate"], cwd=ROOT, check=False
    )
    if gate.returncode == 0:
        print("REUSED: the exact-baseline gate accepts this integration source", flush=True)
    else:
        # The native observer requires 16 GiB after the release and Wasm builds.
        # Those builds consumed the initial guard's small margin in the recorded
        # failed attempt; reclaim private caches with 2 GiB of build headroom.
        if shutil.disk_usage(ROOT).free < 18 * 1024**3:
            target = ROOT / "target"
            if target.resolve() != target:
                raise RuntimeError("Refusing to clean a linked or shared target directory")
            clean = (
                "rustup", "run", "1.99.1", "cargo", "clean", "--locked", "--offline",
                "--target-dir", str(target),
            )
            print("Low disk space: inspecting this worktree's rebuildable output", flush=True)
            run(*clean, "--dry-run")
            run(*clean)
            print(f"Free bytes after private target cleanup: {shutil.disk_usage(ROOT).free}", flush=True)
        run(sys.executable, "scripts/exact_baselines.py", "verify")
    finish_with_evidence()


if __name__ == "__main__":
    main()
