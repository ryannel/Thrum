"""Checks for agent_loop's mechanical decisions: milestone choice, escalation, pace, pins and report lines."""

import json
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
