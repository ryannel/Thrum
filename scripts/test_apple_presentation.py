"""Regression checks for the Mac display and render clock join."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import call, patch

from apple_governor_proof import (
    InvalidDisplayEnvironment, awake_display, compare_live_load, measure_window, require_unlocked_display,
)
from apple_presentation import annotate_display_ticks
from apple_paired_proof import verify_replayed_scale


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


class PairedReplayTests(unittest.TestCase):
    def test_repeated_step_tick_and_skipped_ticks_use_the_exact_trace(self):
        samples = [
            dict(submission=1, tick=323, render_extent=[1286, 724]),
            dict(submission=2, tick=324, render_extent=[960, 540]),
            dict(submission=3, tick=324, render_extent=[960, 540]),
            dict(submission=4, tick=326, render_extent=[960, 540]),
        ]
        frames = [dict(submission=i, presented=True) for i in range(1, 5)]
        verify_replayed_scale("apple", samples, frames, 180, [1920, 1080])
        with self.assertRaisesRegex(ValueError, "missing scale samples for presents \\[4\\]"):
            verify_replayed_scale("apple", samples[:-1], frames, 180, [1920, 1080])
        samples[2]["render_extent"] = [1286, 724]
        with self.assertRaisesRegex(ValueError, "submission 3 at routed tick 144"):
            verify_replayed_scale("apple", samples, frames, 180, [1920, 1080])


class GovernorProofTests(unittest.TestCase):
    def test_idle_display_is_woken_and_cycled_before_the_timed_hold(self):
        locked = '"IOConsoleLocked" = No\n"CGSSessionScreenIsLocked" = Yes'
        unlocked = '"IOConsoleLocked" = No\n"CGSSessionScreenIsLocked" = No'
        wake = call(["/usr/bin/caffeinate", "-u", "-t", "3"], check=True, timeout=6)
        with (
            patch("apple_governor_proof.subprocess.check_output", side_effect=[locked, locked, unlocked]),
            patch("apple_governor_proof.subprocess.run") as run,
            patch("apple_governor_proof.subprocess.Popen") as hold,
            patch("apple_governor_proof.sleep") as sleep,
        ):
            with awake_display():
                self.assertEqual(run.call_args_list, [
                    wake,
                    call(["/usr/bin/pmset", "displaysleepnow"], check=True, timeout=6),
                    wake,
                ])
                sleep.assert_called_once_with(3)
                hold.assert_called_once()
                hold.return_value.terminate.assert_not_called()
            hold.return_value.terminate.assert_called_once()

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

    def test_steady_retries_only_when_fixed_quality_control_cannot_keep_up(self):
        route = dict(origin_tick=180, keys=[dict(tick=60)], tick_hz=60)
        failed = dict(directory="steady", display_hz=120, changes=[
            dict(tick=180 + index, quality=dict(scale=scale))
            for index, scale in enumerate((100, 85, 75, 67))
        ])
        passed = dict(failed, changes=failed["changes"][:1])
        for misses in (0, 3):
            with self.subTest(misses=misses), tempfile.TemporaryDirectory() as directory:
                control = dict(directory="steady-off", display_hz=120, rows=[
                    dict(world_tick=tick, sequence=tick,
                         governor=dict(missed_present_proxy=min(tick - 180, misses)))
                    for tick in range(180, 240)
                ])
                with (
                    patch("apple_governor_proof.require_unlocked_display"),
                    patch("apple_governor_proof.digest", return_value="binary"),
                    patch("apple_governor_proof.window_arm", side_effect=[failed, control, passed]) as arm,
                ):
                    output = Path(directory)
                    args = (output, Path("v1-game"), {}, route)
                    if misses == 0:
                        with self.assertRaisesRegex(ValueError, "steady route changed scale 3 times"):
                            measure_window(*args, steady=True)
                        self.assertEqual(arm.call_count, 2)
                    else:
                        report = measure_window(*args, steady=True)
                        self.assertEqual(report["scale_changes"], 0)
                        self.assertEqual(len(report["invalid_display_attempts"]), 1)
                        self.assertIn("steady-off", report["invalid_display_attempts"][0])
                        self.assertEqual(arm.call_count, 3)
                    self.assertEqual(arm.call_args_list[1].kwargs, {"enabled": False})


if __name__ == "__main__":
    unittest.main()
