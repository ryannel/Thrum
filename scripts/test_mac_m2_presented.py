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
EVIDENCE=ROOT/'docs/evidence'

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
