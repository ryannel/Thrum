"""Checks for agent_loop's mechanical decisions: milestone choice, escalation, pace, pins and report lines."""

import argparse
import json
import os
import sqlite3
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import agent_loop


def done(*milestones):
    return {"module": "WATER", "milestones": list(milestones)}


def milestone(mid, state, statuses, fails=0):
    items = [{"id": f"{mid}.{i}", "status": s} for i, s in enumerate(statuses, 1)]
    return {"id": mid, "name": mid, "state": state, "fails": fails, "items": items}


class Decisions(unittest.TestCase):
    def test_picks_in_progress_before_open(self):
        d = done(milestone("M1", "done", ["pass"]), milestone("M2", "open", ["open"]),
                 milestone("M3", "in_progress", ["open"]))
        self.assertEqual(agent_loop.pick_milestone(d, None)["id"], "M3")

    def test_named_milestone_must_exist(self):
        with self.assertRaises(SystemExit):
            agent_loop.pick_milestone(done(milestone("M1", "open", [])), "M9")

    def test_escalation_ladder(self):
        self.assertEqual([agent_loop.escalation(n) for n in range(7)],
                         ["none", "same-session", "same-session", "fresh-session", "replan", "park", "park"])

    def test_pace_allows_more_as_reset_nears(self):
        now = 1_000_000.0
        self.assertAlmostEqual(agent_loop.allowed_used_percent(now + 6 * 86400, now, 10), 40.0)
        self.assertEqual(agent_loop.allowed_used_percent(now - 10, now, 10), 100.0)

    def test_report_word_reads_last_report_line(self):
        self.assertEqual(agent_loop.report_word("work\nBLOCKED: no Metal device\nevidence"), "BLOCKED")
        self.assertEqual(agent_loop.report_word("`DONE`"), "DONE")
        self.assertIsNone(agent_loop.report_word("still working"))

    def test_pin_mismatch_lists_wrong_turns(self):
        with tempfile.TemporaryDirectory() as tmp:
            rollout = Path(tmp) / "rollout.jsonl"
            lines = [
                {"type": "turn_context", "payload": {"turn_id": "a", "model": "gpt-6.1-sol", "effort": "medium"}},
                {"type": "turn_context", "payload": {"turn_id": "b", "model": "gpt-6.1-sol", "effort": "xhigh"}},
                {"type": "event_msg", "payload": {}},
            ]
            rollout.write_text("\n".join(json.dumps(l) for l in lines))
            self.assertEqual(agent_loop.pin_mismatches(rollout), ["b: gpt-6.1-sol/xhigh"])


    def test_oracle_closed_milestone_is_skipped(self):
        d = done(milestone("M1", "open", ["pass"]), milestone("M2", "open", ["open"]))
        self.assertEqual(agent_loop.pick_milestone(d, None)["id"], "M2")
        reviewed = milestone("M1", "open", ["pass"])
        reviewed["review"] = True
        self.assertEqual(agent_loop.pick_milestone(done(reviewed), None)["id"], "M1")


FAKE_CODEX = """#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
with open(os.environ["FAKE_LOG"], "a") as log:
    log.write(json.dumps(sys.argv[1:]) + "\\n")
path = Path(os.environ["FAKE_WT"]) / "docs/design/modules/WATER-DONE.json"
if os.environ.get("FAKE_PASS") == "1":
    d = json.loads(path.read_text())
    for m in d["milestones"]:
        for i in m["items"]:
            i["status"] = "pass"
    path.write_text(json.dumps(d))
print(json.dumps({"type": "thread.started", "thread_id": "t1"}))
print(json.dumps({"type": "item.completed", "item": {"type": "agent_message", "text": os.environ.get("FAKE_REPLY", "DONE")}}))
"""


class RunLoop(unittest.TestCase):
    """Drives cmd_run against a fake codex and a throwaway repo with an origin."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.wt, origin, self.home = root / "wt", root / "origin.git", root / "codex-home"
        self.home.mkdir()
        git = lambda *a, cwd=None: subprocess.run(["git", *a], cwd=cwd, check=True, capture_output=True)
        git("init", "--bare", "-b", "main", str(origin))
        git("init", "-b", "main", str(self.wt))
        (self.wt / "docs/design/modules").mkdir(parents=True)
        self.write_done([milestone("M1", "open", ["open"])])
        git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "--allow-empty", "-m", "init", cwd=self.wt)
        git("remote", "add", "origin", str(origin), cwd=self.wt)
        git("push", "-q", "origin", "main", cwd=self.wt)
        fake = root / "codex"
        fake.write_text(FAKE_CODEX)
        fake.chmod(0o755)
        self.log = root / "log"
        self.saved = (agent_loop.CODEX, agent_loop.CODEX_HOME)
        agent_loop.CODEX, agent_loop.CODEX_HOME = str(fake), self.home
        os.environ.update(FAKE_LOG=str(self.log), FAKE_WT=str(self.wt), FAKE_PASS="1", FAKE_REPLY="DONE")

    def tearDown(self):
        agent_loop.CODEX, agent_loop.CODEX_HOME = self.saved
        self.tmp.cleanup()

    def write_done(self, milestones):
        (self.wt / "docs/design/modules/WATER-DONE.json").write_text(json.dumps(done(*milestones)))

    def run_loop(self, **kw):
        args = dict(module="WATER", worktree=str(self.wt), milestone=None, fresh=False, message=None,
                    budget_hours=4.0, pace_per_day=10.0, turn_timeout_hours=0.1)
        args.update(kw)
        with self.assertRaises(SystemExit):
            agent_loop.cmd_run(argparse.Namespace(**args))
        return json.loads((self.wt / "tmp/streams/WATER/state.json").read_text())

    def calls(self):
        return [json.loads(l) for l in self.log.read_text().splitlines()]

    def test_claims_when_items_pass_and_head_is_on_main(self):
        self.assertEqual(self.run_loop()["result"], "CLAIMED")

    def test_rerun_after_failed_review_resumes_with_message(self):
        reviewed = milestone("M1", "open", ["open"])
        reviewed["review"] = True
        self.write_done([reviewed])
        self.run_loop()
        d = json.loads((self.wt / "docs/design/modules/WATER-DONE.json").read_text())
        d["milestones"][0]["items"][0]["status"] = "fail"
        (self.wt / "docs/design/modules/WATER-DONE.json").write_text(json.dumps(d))
        self.run_loop(message="FINDINGS")
        last = self.calls()[-1]
        self.assertEqual(last[:3], ["exec", "resume", "t1"])
        self.assertEqual(last[-1], "FINDINGS")

    def test_pace_is_checked_on_every_run(self):
        sessions = self.home / "sessions"
        sessions.mkdir()
        limits = {"limit_id": "codex", "primary": {"used_percent": 99.0, "window_minutes": 10080,
                                                   "resets_at": 4_000_000_000}}
        (sessions / "rollout-x.jsonl").write_text(json.dumps({"type": "event_msg", "payload": {
            "type": "token_count", "rate_limits": limits}}))
        self.assertEqual(self.run_loop()["result"], "PACE")
        self.assertEqual(self.run_loop()["result"], "PACE")
        self.assertFalse(self.log.exists())

    def test_rate_limits_ignore_other_limits(self):
        sessions = self.home / "sessions"
        sessions.mkdir()
        lines = [{"type": "event_msg", "payload": {"type": "token_count", "rate_limits": {
            "limit_id": "codex", "primary": {"used_percent": 47.0, "window_minutes": 10080, "resets_at": 1}}}},
                 {"type": "event_msg", "payload": {"type": "token_count", "rate_limits": {
            "limit_id": "premium", "primary": {"used_percent": 0.0, "window_minutes": 0, "resets_at": 1}}}}]
        (sessions / "rollout-x.jsonl").write_text("\n".join(json.dumps(l) for l in lines))
        self.assertEqual(agent_loop.latest_rate_limits()["used_percent"], 47.0)

    def test_second_report_without_progress_finishes(self):
        os.environ.update(FAKE_PASS="0", FAKE_REPLY="BLOCKED: no device")
        with sqlite3.connect(self.home / "goals_1.sqlite") as conn:
            conn.execute("create table thread_goals (thread_id text, status text)")
            conn.execute("insert into thread_goals values ('t1', 'active')")
        state = self.run_loop()
        self.assertEqual(state["result"], "BLOCKED")
        self.assertEqual(state["turns"], 2)


if __name__ == "__main__":
    unittest.main()
