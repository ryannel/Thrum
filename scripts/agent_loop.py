#!/usr/bin/env python3
"""Run one Codex generator on one Thrum milestone, with model and effort pinned, until a stop rule fires.

Used by the build-parallel lead (.agents/skills/build-parallel) and runnable by hand. Subcommands:

  run   --module WATER --worktree PATH [--milestone M2] [--fresh] [--budget-hours 4] [--pace-per-day 10]
        Starts (or resumes) a Codex session on the milestone and drives it one turn per `codex exec resume` until:
        CLAIMED   every item of the milestone has status pass in the worktree's done-when file
        DONE / BLOCKED / NEEDS_CONTEXT   the agent's final report line
        BUDGET    no new passing item or commit for --budget-hours
        FLOOR     two failed turns in a row, or the goal is usage- or budget-limited (likely quota or outage)
        PIN       a turn ran on a model or effort other than the pinned ones
        PACE      the account's weekly Codex use is ahead of --pace-per-day (checked at the start of every run)
        PAUSED    the goal is paused; only the Codex app's play button resumes it
        CLAIMED also needs HEAD to be on origin/main, so the milestone was pushed. A run on the same milestone resumes
        its session (with --message as the first prompt, e.g. the evaluator's findings) unless --fresh is given.
        Prints one JSON line with the result as the last line of output.

  next  --module WATER --worktree PATH --milestone M2
        Reads the milestone's `fails` count (written by the evaluator) and prints the escalation:
        same-session (1-2), fresh-session (3), replan (4), park (5+), or none (0).

  status --module WATER --worktree PATH
        Prints each milestone's state, fails and passing items.

  gpu -- COMMAND...
        Runs COMMAND under a lock shared by every worktree of this repository, so device timing and captures from
        parallel streams never overlap on the GPU.

State lives in tmp/streams/<module>/state.json under the worktree, so re-running the same command resumes.
"""

from __future__ import annotations

import argparse
import fcntl
import glob
import json
import os
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

MODEL = "gpt-6.1-sol"
EFFORT = "medium"
CODEX = os.environ.get("THRUM_CODEX", "/Applications/ChatGPT.app/Contents/Resources/codex-cli/bin/codex")
CODEX_HOME = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
REPORT_WORDS = ("DONE", "BLOCKED", "NEEDS_CONTEXT")

GOAL_TEMPLATE = """Model {model}, effort {effort}. Call your create_goal tool with this objective, then work it:

Milestone {milestone} of module {module} is done: every item for {milestone} in docs/design/modules/{module}-DONE.json \
has status pass with evidence on its named target; the change is rebased on origin/main, a fresh `./dev check` passes \
on that tip, and it is pushed to main without force. Use $build-milestone, which covers method, constraints and \
permissions. Work only in this worktree. Keep the plan's status lines and log current. If you are blocked (probed \
directly) or missing context, stop and end with `BLOCKED: ...` or `NEEDS_CONTEXT: ...` and the evidence. When finished, \
end with `DONE` and the evidence."""

FINDINGS_TEMPLATE = """The evaluator failed {milestone}: see docs/evidence/{module_lower}/{milestone}-review.md. Add its \
findings to the plan as work items, fix them, re-verify the affected items, push, and end with DONE, BLOCKED or \
NEEDS_CONTEXT."""


def codex_flags() -> list[str]:
    return [
        "-m", MODEL,
        "-c", f"model_reasoning_effort={EFFORT}",
        "-c", "approval_policy=on-request",
        "-c", "approvals_reviewer=auto_review",
        "-c", "sandbox_workspace_write.network_access=true",
    ]


def load_done(worktree: Path, module: str) -> dict:
    path = worktree / "docs" / "design" / "modules" / f"{module}-DONE.json"
    return json.loads(path.read_text())


def pick_milestone(done: dict, wanted: str | None) -> dict:
    milestones = done["milestones"]
    if wanted:
        for m in milestones:
            if m["id"] == wanted:
                if m.get("state") in ("done", "parked"):
                    print(f"warning: {wanted} is {m['state']}", file=sys.stderr)
                return m
        raise SystemExit(f"error: milestone {wanted} not in done-when file")
    for state in ("in_progress", "open"):
        for m in milestones:
            closed_by_oracle = not m.get("review") and passing(m) == len(m["items"])
            if m.get("state") == state and not closed_by_oracle:
                return m
    raise SystemExit("error: no open milestone")


def passing(milestone: dict) -> int:
    return sum(1 for item in milestone["items"] if item.get("status") == "pass")


def escalation(fails: int) -> str:
    if fails <= 0:
        return "none"
    if fails <= 2:
        return "same-session"
    if fails == 3:
        return "fresh-session"
    if fails == 4:
        return "replan"
    return "park"


def allowed_used_percent(resets_at: float, now: float, pace_per_day: float) -> float:
    days_left = max(0.0, (resets_at - now) / 86400.0)
    return max(0.0, 100.0 - pace_per_day * days_left)


def rollout_for(thread_id: str) -> Path | None:
    hits = glob.glob(str(CODEX_HOME / "sessions" / "**" / f"rollout-*{thread_id}.jsonl"), recursive=True)
    return Path(max(hits, key=os.path.getmtime)) if hits else None


def pin_mismatches(rollout: Path) -> list[str]:
    bad = []
    for line in rollout.read_text().splitlines():
        if '"turn_context"' not in line:
            continue
        payload = json.loads(line).get("payload", {})
        model, effort = payload.get("model"), payload.get("effort")
        if (model, effort) != (MODEL, EFFORT):
            bad.append(f"{payload.get('turn_id')}: {model}/{effort}")
    return bad


def latest_rate_limits() -> dict | None:
    files = sorted(glob.glob(str(CODEX_HOME / "sessions" / "**" / "rollout-*.jsonl"), recursive=True),
                   key=os.path.getmtime, reverse=True)[:5]
    for f in files:
        for line in reversed(Path(f).read_text().splitlines()):
            if '"token_count"' in line:
                limits = json.loads(line).get("payload", {}).get("rate_limits") or {}
                primary = limits.get("primary") or {}
                if limits.get("limit_id") == "codex" and primary.get("window_minutes") == 10080:
                    return primary
    return None


def goal_status(thread_id: str) -> str | None:
    db = CODEX_HOME / "goals_1.sqlite"
    if not db.exists():
        return None
    with sqlite3.connect(f"file:{db}?mode=ro", uri=True) as conn:
        row = conn.execute("select status from thread_goals where thread_id=?", (thread_id,)).fetchone()
    return row[0] if row else None


def head_commit(worktree: Path) -> str:
    return subprocess.run(["git", "-C", str(worktree), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()


def on_main(worktree: Path) -> bool:
    subprocess.run(["git", "-C", str(worktree), "fetch", "--quiet", "origin", "main"], capture_output=True)
    return subprocess.run(["git", "-C", str(worktree), "merge-base", "--is-ancestor", "HEAD", "origin/main"],
                          capture_output=True).returncode == 0


def git_common_dir(worktree: Path) -> str:
    out = subprocess.run(["git", "-C", str(worktree), "rev-parse", "--git-common-dir"], capture_output=True, text=True)
    return str((worktree / out.stdout.strip()).resolve())


def run_turn(args: list[str], timeout_s: float) -> tuple[int, str | None, str]:
    """Runs one codex exec call; returns (exit code, thread id, last agent message). A timeout counts as failed."""
    try:
        proc = subprocess.run([CODEX, *args], capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=timeout_s)
        code, stdout = proc.returncode, proc.stdout
    except subprocess.TimeoutExpired as e:
        code, stdout = 124, e.stdout.decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
    thread_id, last_message = None, ""
    for line in stdout.splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if event.get("type") == "thread.started":
            thread_id = event.get("thread_id")
        item = event.get("item") or {}
        if item.get("type") == "agent_message" and item.get("text"):
            last_message = item["text"]
    return code, thread_id, last_message


def report_word(message: str) -> str | None:
    for line in reversed([l.strip() for l in message.splitlines() if l.strip()]):
        word = line.split(":")[0].strip("`* ")
        if word in REPORT_WORDS:
            return word
    return None


def finish(state_path: Path, state: dict, result: str, **extra) -> None:
    state.update(result=result, finished=time.time(), **extra)
    state_path.write_text(json.dumps(state, indent=2))
    print(json.dumps({"result": result, "module": state["module"], "milestone": state["milestone"],
                      "thread": state.get("thread"), **extra}))
    sys.exit(0)


def cmd_run(a: argparse.Namespace) -> None:
    worktree = Path(a.worktree).resolve()
    state_dir = worktree / "tmp" / "streams" / a.module
    state_dir.mkdir(parents=True, exist_ok=True)
    state_path = state_dir / "state.json"
    milestone = pick_milestone(load_done(worktree, a.module), a.milestone)
    state = json.loads(state_path.read_text()) if state_path.exists() else {}
    fresh = a.fresh or state.get("milestone") != milestone["id"] or state.get("result") == "PIN"
    if fresh:
        state = {"module": a.module, "milestone": milestone["id"], "thread": None, "turns": 0}
    # Every run starts its own counters, so a resume after BUDGET or a failed review isn't judged on old progress.
    state.update(module=a.module, milestone=milestone["id"], failed_turns=0, reports=0, progress_at=time.time(),
                 passing=passing(milestone), head=head_commit(worktree))
    state.pop("result", None)
    limits = latest_rate_limits()
    if limits:
        allowed = allowed_used_percent(limits["resets_at"], time.time(), a.pace_per_day)
        if limits["used_percent"] > allowed:
            finish(state_path, state, "PACE", used=limits["used_percent"], allowed=round(allowed, 1))
    flags = codex_flags()
    timeout_s = a.turn_timeout_hours * 3600

    while True:
        if state.get("thread") is None:
            prompt = GOAL_TEMPLATE.format(model=MODEL, effort=EFFORT, milestone=milestone["id"], module=a.module)
            if a.message:
                prompt += "\n\n" + a.message
                a.message = None
            code, thread_id, message = run_turn(["exec", *flags, "--add-dir", git_common_dir(worktree),
                                                 "--thread-source", "vscode", "-C", str(worktree), "--json", prompt],
                                                timeout_s)
            state["thread"] = thread_id
        else:
            prompt = a.message or "Continue the goal."
            a.message = None
            code, _, message = run_turn(["exec", "resume", state["thread"], *flags, "--json", prompt], timeout_s)
        state["turns"] += 1
        state["failed_turns"] = state["failed_turns"] + 1 if code != 0 else 0
        state_path.write_text(json.dumps(state, indent=2))

        if state["thread"] is None or state["failed_turns"] >= 2:
            finish(state_path, state, "FLOOR", reason="two failed turns or no thread; likely quota or outage")
        rollout = rollout_for(state["thread"])
        if rollout and (bad := pin_mismatches(rollout)):
            finish(state_path, state, "PIN", mismatches=bad[:5])
        status = goal_status(state["thread"])
        if status in ("usage_limited", "budget_limited"):
            finish(state_path, state, "FLOOR", reason=f"goal {status}")
        if status == "paused":
            finish(state_path, state, "PAUSED")

        current = pick_milestone(load_done(worktree, a.module), milestone["id"])
        now_passing, head = passing(current), head_commit(worktree)
        if now_passing > state["passing"] or head != state["head"]:
            state.update(progress_at=time.time(), passing=now_passing, head=head)
        if now_passing == len(current["items"]) and on_main(worktree):
            finish(state_path, state, "CLAIMED", review=bool(current.get("review")))
        word = report_word(message)
        state["reports"] = state.get("reports", 0) + 1 if word else 0
        if word and (status != "active" or state["reports"] >= 2):
            finish(state_path, state, word, report=message[-1500:])
        if status in ("complete", "blocked"):
            finish(state_path, state, "BLOCKED" if status == "blocked" else "DONE", report=message[-1500:])
        if time.time() - state["progress_at"] > a.budget_hours * 3600:
            finish(state_path, state, "BUDGET", hours=a.budget_hours)


def cmd_next(a: argparse.Namespace) -> None:
    milestone = pick_milestone(load_done(Path(a.worktree), a.module), a.milestone)
    fails = int(milestone.get("fails", 0))
    message = FINDINGS_TEMPLATE.format(milestone=milestone["id"], module_lower=a.module.lower())
    print(json.dumps({"milestone": milestone["id"], "fails": fails, "action": escalation(fails),
                      "resume_message": message}))


def cmd_status(a: argparse.Namespace) -> None:
    for m in load_done(Path(a.worktree), a.module)["milestones"]:
        print(f"{m['id']}\t{m.get('state')}\tfails={m.get('fails', 0)}\t{passing(m)}/{len(m['items'])} pass\t{m['name']}")


def cmd_gpu(a: argparse.Namespace) -> None:
    command = a.command_args[1:] if a.command_args[:1] == ["--"] else a.command_args
    if not command:
        raise SystemExit("error: gpu needs a command after --")
    lock = Path(git_common_dir(Path.cwd())) / "thrum-gpu.lock"
    with open(lock, "w") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        sys.exit(subprocess.run(command).returncode)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("--module", required=True)
    run.add_argument("--worktree", required=True)
    run.add_argument("--milestone")
    run.add_argument("--fresh", action="store_true", help="start a new Codex session even if one exists")
    run.add_argument("--message", help="first prompt on resume, e.g. the evaluator's findings message")
    run.add_argument("--budget-hours", type=float, default=4.0)
    run.add_argument("--pace-per-day", type=float, default=10.0)
    run.add_argument("--turn-timeout-hours", type=float, default=2.0)
    run.set_defaults(func=cmd_run)
    for name, func in (("next", cmd_next), ("status", cmd_status)):
        p = sub.add_parser(name)
        p.add_argument("--module", required=True)
        p.add_argument("--worktree", required=True)
        if name == "next":
            p.add_argument("--milestone", required=True)
        p.set_defaults(func=func)
    gpu = sub.add_parser("gpu")
    gpu.add_argument("command_args", nargs=argparse.REMAINDER)
    gpu.set_defaults(func=cmd_gpu)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
