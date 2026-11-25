"""Regression checks for the Mac display and render clock join."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from apple_governor_proof import InvalidDisplayEnvironment, compare_live_load, require_unlocked_display
from apple_presentation import annotate_display_ticks


def callback(sequence, media, dispatched, busy=False):
    return dict(sequence=sequence, callback=sequence, media=media,
                target=media + 0.008, dispatched=dispatched, render_busy=busy)


def frame(sequence, started, ended, present=None):
    return dict(sequence=sequence, presented=present is not None, present_media=present,
                submission=sequence if present is not None else None,
                status="Presented" if present is not None else "FrameSlotsOccupied",
                work=[["encoding the scene", started, ended]])


class DisplayTickTests(unittest.TestCase):
    def receipt(self, callbacks, frames, calls=()):
        presented = sum(row["presented"] for row in frames)
        with tempfile.TemporaryDirectory() as directory:
            arm = Path(directory)
            (arm / "display.jsonl").write_text(
                "".join(json.dumps(tick) + "\n" for tick in callbacks))
            (arm / "game.log").write_text(
                f"game; presented={presented}; callbacks={len(frames)}; "
                f"display_link_callbacks={len(callbacks)};")
            return annotate_display_ticks(arm, frames, calls, expected_conflict=False)

    def test_dispatched_target_whose_frame_starts_late(self):
        # The review's valley case dispatched promptly, started work 0.11 ms
        # after the next interval opened, and presented in that interval.
        ticks = self.receipt(
            [callback(1, 0, 0.00098), callback(2, 0.00833, 0.00886, busy=True)],
            [frame(1, 0.00844, 0.012, present=0.01040)],
        )
        self.assertEqual(ticks[0]["outcome"], "target frame work started after its interval")
        self.assertEqual(ticks[0]["target_frames"][0]["submission"], 1)
        self.assertNotIn("superseded_by", ticks[0])
        self.assertEqual(ticks[1]["presented_submissions"], [1])

    def test_callback_dispatched_late_still_owns_its_frame(self):
        # The review's ordinary window case also started late, but its
        # callback was already past the interval when the host received it.
        ticks = self.receipt(
            [callback(1, 0, 0.041), callback(2, 0.0409, 0.042)],
            [frame(1, 0.0411, 0.046, present=0.045)],
        )
        self.assertEqual(ticks[0]["outcome"], "display callback dispatched after its interval")
        self.assertGreater(ticks[0]["target_frames"][0]["work_started"], ticks[0]["interval_end"])
        self.assertNotIn("superseded_by", ticks[0])
        self.assertEqual(ticks[1]["presented_submissions"], [1])

    def test_unmatched_target_requires_a_successor_for_coalescing(self):
        ticks = self.receipt(
            [callback(1, 0, 0.001, busy=True), callback(2, 0.008, 0.009),
             callback(3, 0.016, 0.017)],
            [frame(2, 0.010, 0.015, present=0.014)],
        )
        self.assertEqual(ticks[0]["outcome"], "display target superseded before frame work")
        self.assertEqual(ticks[0]["superseded_by"], 2)
        self.assertEqual(ticks[0]["target_frames"], [])
        self.assertEqual(ticks[2]["outcome"], "run ended with the last display target pending")
        self.assertNotIn("superseded_by", ticks[2])

    def test_configure_and_frame_deferral_keep_their_witnesses(self):
        call = dict(sequence=1, started=0.002, ended=0.010, wall_ms=8.0)
        ticks = self.receipt(
            [callback(1, 0, 0.001), callback(2, 0.008, 0.009),
             callback(3, 0.016, 0.017)],
            [frame(1, 0.0015, 0.012, present=0.011), frame(3, 0.018, 0.020)],
            [call],
        )
        self.assertEqual(ticks[0]["outcome"], "configure")
        self.assertEqual(ticks[0]["configure_calls"], [call])
        self.assertEqual(ticks[1]["presented_submissions"], [1])
        self.assertEqual(ticks[2]["outcome"], "FrameSlotsOccupied")

    def test_miss_uses_the_stage_at_interval_close(self):
        late = frame(1, 0.001, 0.011, present=0.010)
        late["work"] = [["encoding the scene", 0.001, 0.007],
                        ["acquiring a drawable", 0.007, 0.010],
                        ["collecting frame completions and reply", 0.010, 0.011]]
        call = dict(sequence=1, started=0.001, ended=0.002, wall_ms=1.0)
        ticks = self.receipt(
            [callback(1, 0, 0.001), callback(2, 0.008, 0.009)], [late], [call])
        self.assertEqual(ticks[0]["outcome"], "acquiring a drawable")
        self.assertEqual(ticks[0]["configure_calls"], [call])


class GovernorProofTests(unittest.TestCase):
    def test_locked_console_session_takes_precedence_over_unlocked_display_flag(self):
        console = '"IOConsoleLocked" = No\n"IOConsoleUsers" = ({"CGSSessionScreenIsLocked"=Yes})'
        with patch("apple_governor_proof.subprocess.check_output", return_value=console):
            with self.assertRaisesRegex(InvalidDisplayEnvironment, "console session is locked"):
                require_unlocked_display()

    def test_missed_presents_in_unloaded_baseline_invalidate_attempt_before_load(self):
        for miss_start, miss_end in ((200, 230), (260, 320)):
            with self.subTest(miss_start=miss_start):
                rows = [
                    dict(world_tick=tick, sequence=tick, governor={
                        "missed_present_proxy": min(max(tick - miss_start, 0), miss_end - miss_start),
                    })
                    for tick in range(200, 320)
                ]
                baseline = dict(directory="steady", rows=rows)
                with patch("apple_governor_proof.window_arm", return_value=baseline) as arm:
                    with self.assertRaisesRegex(InvalidDisplayEnvironment, "unloaded baseline"):
                        compare_live_load(Path("."), Path("v1-game"), {}, {})
                arm.assert_called_once_with(Path("."), Path("v1-game"), {}, {}, "steady")


if __name__ == "__main__":
    unittest.main()
