"""Known-answer missed refreshes and witness refusals; no recordings or GPU work."""
import copy
from decimal import Decimal as D
from pathlib import Path
import unittest
import tempfile
import json
import sys
from qualify_mac_m2 import bounded
import mac_m2_presented as witness

ROOT=Path(__file__).resolve().parents[1]
EVIDENCE=ROOT/'.orchestrate/evidence'

class PresentedWitness(unittest.TestCase):
    def retained(self, arm):
        path=EVIDENCE/'render/mac-m2-20261111'
        if arm=='off':
            path=path/'off-traced'
        pid,low,high,markers=witness.calibration(path)
        rows=witness.presented_rows(EVIDENCE/f'PLATFORM-RUNTIME-MAC/m2-presented-{arm}-traced.tsv.gz',pid)
        surfaces=witness.table(path/'surface.tsv')
        requests=witness.table(path/'requests.tsv')
        return rows,surfaces,requests,low,high,markers

    def test_known_missed_refreshes_and_truncated_refusal(self):
        for arm, expected in [('off',[(526,527),(544,545)]),('on',[(558,559)])]:
            result=witness.analyse(*self.retained(arm))
            self.assertEqual([(r['from'],r['to']) for r in result['missed_refreshes']],expected)
            self.assertTrue(result['conversion_validated'])
            self.assertFalse(result['coverage'])
            self.assertFalse(result['qualified'])
            if arm=='off':
                self.assertAlmostEqual(result['missed_refreshes'][1]['present_call_gap_ms'],15.470792,places=6)

    def test_callback_time_and_delta_are_not_presented_time(self):
        args=list(self.retained('off'))
        original=witness.analyse(*args)
        for row in args[0]:
            row['delta_ns']='1'
            row['handler_trace_ns']=str(int(row['handler_trace_ns'])+1000000)
        changed=witness.analyse(*args)
        self.assertEqual(original['intervals'],changed['intervals'])

    def test_zero_absent_unknown_and_duplicate_are_unmatched(self):
        for field,value in [('requested_time_raw','0'),('requested_time_raw',None),('status','99')]:
            args=list(self.retained('off'))
            args[0][4][field]=value
            result=witness.analyse(*args)
            self.assertTrue(any(r['drawable']==500 for r in result['unmatched']))
        args=list(self.retained('off'))
        args[0][4]['requested_time_raw']=str(-(args[3]+args[4])/2)
        self.assertTrue(any(r['drawable']==500 for r in witness.analyse(*args)['unmatched']))
        args=list(self.retained('off'))
        args[0].append(copy.deepcopy(args[0][0]))
        self.assertTrue(any(r['reason']=='duplicate ordinal' for r in witness.analyse(*args)['unmatched']))

    def test_missing_ordinal_and_wrong_conversion_refused(self):
        args=list(self.retained('off'))
        del args[0][4]
        result=witness.analyse(*args)
        self.assertTrue(any(r['drawable']==500 for r in result['unmatched']))
        self.assertFalse(witness.analyse(*args,numer=1,denom=1)['conversion_validated'])

    def test_unmatched_scope_uses_os_presented_bounds(self):
        args=list(self.retained('off'))
        args[5]=[5700000000,5800000000,5900000000,6000000000]
        args[0][0]['status']='99'
        result=witness.analyse(*args)
        self.assertTrue(result['qualified'])
        self.assertTrue(result['unmatched'][0]['outside_marked_presented_interval'])
        args[0][15]['status']='99'
        result=witness.analyse(*args)
        self.assertFalse(result['qualified'])
        self.assertTrue(result['unmatched_affecting_marked_interval'])

    def test_xml_references_and_pid_attribution(self):
        source = """<trace-query-result><node><schema name="ca-client-presented-handler">
        <col><mnemonic>timestamp</mnemonic></col><col><mnemonic>drawable-id</mnemonic></col>
        <col><mnemonic>status</mnemonic></col><col><mnemonic>at-time</mnemonic></col>
        <col><mnemonic>delta</mnemonic></col><col><mnemonic>process</mnemonic></col></schema>
        <row><t>1</t><d>4</d><s id="status">2</s><t>100</t><v>9</v><p id="process" fmt="v1-game (7)"/></row>
        <row><t>2</t><d>5</d><s ref="status"/><t>200</t><v>9</v><p ref="process"/></row>
        <row><t>2</t><d>6</d><s ref="status"/><t>300</t><v>9</v><p fmt="v1-game (8)"/></row>
        </node></trace-query-result>"""
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'rows.xml'
            path.write_text(source)
            rows=witness.presented_rows(path,7)
            self.assertEqual([r['drawable_id'] for r in rows],['4','5'])
            self.assertEqual(rows[1]['status'],'2')

    def test_measured_interval_uses_retained_clock_markers(self):
        source = """<trace-query-result><node>
        <row><process fmt="v1-game (7)"/><event-time>21000000000</event-time>
        <event-type>End</event-type><signpost-name>MacM2Pan</signpost-name></row>
        <row><process fmt="v1-game (7)"/><event-time>22000000000</event-time>
        <event-type>End</event-type><signpost-name>MacM2Whole</signpost-name></row>
        </node></trace-query-result>"""
        with tempfile.TemporaryDirectory() as directory:
            arm = Path(directory)
            (arm / 'window.txt').write_text('pid=7\n')
            (arm / 'signposts.xml').write_text(source)
            (arm / 'signposts.tsv').write_text(
                'stage\tmedia_before\tmedia_after\n'
                '0\t100.25\t100.250000002\n1\t101\t101.000000002\n'
                '2\t121\t121.000000002\n3\t122\t122.000000002\n'
            )
            (arm / 'refresh-interval.tsv').write_text(
                'start_media_seconds\tend_media_seconds\n101.000000001\t121.000000001\n'
            )
            with self.assertRaisesRegex(ValueError, 'missing same-process clock markers'):
                witness.calibration(arm)
            pid, low, high, markers = witness.calibration(arm, measured_interval=True)
            self.assertEqual((pid, low, high), (7, 100000000000, 100000000002))
            self.assertEqual(markers, [1000000000, 1000000000, 21000000000, 21000000000])
            (arm / 'signposts.xml').write_text(source.replace('v1-game (7)', 'v1-game (8)', 1))
            with self.assertRaisesRegex(ValueError, 'missing same-process clock markers'):
                witness.calibration(arm, measured_interval=True)

    def test_replayed_handlers_use_the_live_trace_clock(self):
        columns = ''.join(f'<col><mnemonic>{name}</mnemonic></col>'
                          for name in ('time', 'thread', 'class', 'subclass', 'code', 'arg1', 'arg2'))
        template = '<row><t>{}</t><thread><process fmt="v1-game (7)"/></thread><c>{}</c><sc>{}</sc><code>{}</code><a>{}</a><b>2</b></row>'
        source = (
            f'<trace-query-result><node><schema name="kdebug">{columns}</schema>'
            + template.format(998, 31, 5, 81, 4)
            + '</node><node>'
            + template.format(1501, 49, 202, 81, 4)
            + template.format(1700, 49, 202, 81, 5)
            + '</node></trace-query-result>'
        )
        rows = [{'drawable_id': '4', 'status': '2', 'handler_trace_ns': '1300'},
                {'drawable_id': '5', 'status': '2', 'handler_trace_ns': '1500'}]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'live.xml'
            path.write_text(source)
            self.assertEqual(witness.replay_origin_shift(path, 7, rows), (201, 2))
            rows[1]['handler_trace_ns'] = '1505'
            with self.assertRaisesRegex(ValueError, 'one trace-origin shift'):
                witness.replay_origin_shift(path, 7, rows)

    def test_finalization_cap_starts_at_recording_stop(self):
        with tempfile.TemporaryDirectory() as directory:
            log=Path(directory)/'process.log'
            command=[sys.executable,'-c',"import time; print('Target app exited, ending recording...',flush=True); time.sleep(30)"]
            with self.assertRaises(RuntimeError):
                bounded(command,log,2,finalization_seconds=.2)
            timing=json.loads(log.with_suffix('.timing.json').read_text())
            self.assertAlmostEqual(timing['deadline_monotonic_s']-timing['recording_stop_observed_monotonic_s'],.2,places=5)
            self.assertLess(timing['deadline_monotonic_s']-timing['start_monotonic_s'],2)

    def test_complete_synthetic_interval_is_accepted(self):
        args=list(self.retained('off'))
        args[5]=[5700000000,5800000000,5900000000,6000000000]
        result=witness.analyse(*args)
        self.assertTrue(result['coverage'])
        self.assertTrue(result['qualified'])

if __name__=='__main__':unittest.main()
