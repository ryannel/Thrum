#!/usr/bin/env python3
"""Export and qualify OS-reported drawable presented times; never substitutes callback times."""
from __future__ import annotations
import argparse
import csv
import ctypes
from decimal import Decimal as D
import gzip
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

NS = D(10**9)


def text_file(path):
    if not path.exists() and path.with_suffix(path.suffix + '.gz').exists():
        path = path.with_suffix(path.suffix + '.gz')
    return gzip.open(path, 'rt') if str(path).endswith('.gz') else path.open()


def table(path):
    with text_file(path) as stream:
        return list(csv.DictReader(stream, delimiter='\t'))


def xml(path):
    with text_file(path) as stream:
        root = ET.parse(stream).getroot()
    ids = {e.get('id'): e for e in root.iter() if e.get('id')}
    def resolve(e):
        seen = set()
        while e.get('ref'):
            key = e.get('ref')
            if key in seen or key not in ids:
                raise ValueError('unresolved or cyclic XML reference')
            seen.add(key)
            e = ids[key]
        return e
    return root, resolve


def calibration(arm):
    pid = int(re.search(r'^pid=(\d+)$', (arm / 'window.txt').read_text(), re.M)[1])
    root, resolve = xml(arm / 'signposts.xml')
    points = {}
    for row in root.iter('row'):
        f = {e.tag: resolve(e) for e in row}
        if f.get('signpost-name') is None or f.get('process') is None:
            continue
        if f['process'].get('fmt') != f'v1-game ({pid})':
            continue
        name = f['signpost-name'].text
        if name not in ('MacM2Whole', 'MacM2Pan') or 'event-type' not in f:
            continue
        key = (name, f['event-type'].text)
        value = int(f['event-time'].text)
        if key in points and points[key] != value:
            raise ValueError('ambiguous marker')
        points[key] = value
    keys = [('MacM2Whole', 'Begin'), ('MacM2Pan', 'Begin'), ('MacM2Pan', 'End'), ('MacM2Whole', 'End')]
    markers = table(arm / 'signposts.tsv')
    if len(markers) != 4:
        raise ValueError('four marker brackets required')
    bounds = [(D(r['media_before']) * NS - points[k], D(r['media_after']) * NS - points[k]) for r, k in zip(markers, keys, strict=True)]
    low, high = max(b[0] for b in bounds), min(b[1] for b in bounds)
    if low > high:
        raise ValueError('inconsistent marker clock brackets')
    return pid, low, high, [points[k] for k in keys]


def presented_rows(path, pid):
    if path.suffix != '.xml':
        return table(path)
    root, resolve = xml(path)
    rows = []
    for node in root.findall('node'):
        schema = node.find('schema')
        if schema is None or schema.get('name') != 'ca-client-presented-handler':
            continue
        names = [c.findtext('mnemonic') for c in schema.findall('col')]
        for row in node.findall('row'):
            f = dict(zip(names, [resolve(e) for e in row], strict=True))
            if f['process'].get('fmt') != f'v1-game ({pid})':
                continue
            rows.append({'drawable_id': f['drawable-id'].text, 'handler_trace_ns': f['timestamp'].text,
                         'status': f['status'].text, 'requested_time_raw': f['at-time'].text, 'delta_ns': f['delta'].text})
    return rows


def timebase():
    class Timebase(ctypes.Structure):
        _fields_ = [('numer', ctypes.c_uint32), ('denom', ctypes.c_uint32)]
    value = Timebase()
    lib = ctypes.CDLL('/usr/lib/libSystem.B.dylib')
    if lib.mach_timebase_info(ctypes.byref(value)) or not value.numer or not value.denom:
        raise ValueError('mach timebase unavailable')
    return value.numer, value.denom


def analyse(rows, surface, requests, low, high, markers, numer=125, denom=3, hz=120):
    # Instruments treats the already-nanosecond CA value as mach ticks, scales it,
    # then subtracts recording origin. Undo those operations using independently
    # bracketed OS signposts. The delta column is deliberately unused.
    offset = (low + high) / 2
    factor = D(denom) / D(numer)
    by_id = {int(r['submission_id']): r for r in surface if r['presented'] == 'true'}
    request_by_sequence = {int(r['sequence']): r for r in requests}
    yaw = {seq:r['accepted_yaw_degrees'] for seq,r in request_by_sequence.items()}
    values, unmatched, seen = [], [], set()
    for row in rows:
        ordinal = int(row['drawable_id'])
        raw = row.get('requested_time_raw')
        converted = (D(raw)+offset)*factor if raw else None
        reason = None
        if ordinal in seen:
            reason = 'duplicate ordinal'
        elif row.get('status') != '2':
            reason = 'unfamiliar status'
        elif converted is None or D(raw)==0 or abs(converted) <= (high-low)*factor/2+D(1):
            reason = 'zero or absent presented time'
        elif ordinal not in by_id:
            reason = 'missing submission ordinal'
        seen.add(ordinal)
        if reason:
            unmatched.append({'drawable': ordinal, 'reason': reason})
            continue
        submission = by_id[ordinal]
        media_ns = converted
        trace_ns = media_ns - offset
        handler = D(row['handler_trace_ns'])
        call = D(submission['media_seconds']) * NS
        values.append({'drawable': ordinal, 'presented_trace_ns': float(trace_ns),
                       'presented_media_ns': str(media_ns), 'handler_trace_ns': int(handler),
                       'handler_lag_ms': float((handler-trace_ns)/D(10**6)),
                       'present_call_media_seconds': submission['media_seconds'],
                       'call_to_present_ms': float((media_ns-call)/D(10**6)),
                       'sequence': int(submission['sequence']), 'yaw': yaw.get(int(submission['sequence'])),
                       'opportunity_media_seconds': request_by_sequence[int(submission['sequence'])]['media_seconds'],
                       'callback_target_media_seconds': request_by_sequence[int(submission['sequence'])]['target_media_seconds']})
    values.sort(key=lambda r:r['drawable'])
    period = NS / D(hz)
    intervals = []
    for a,b in zip(values,values[1:]):
        if b['drawable'] != a['drawable']+1:
            unmatched.extend({'drawable':i,'reason':'missing ordinal'} for i in range(a['drawable']+1,b['drawable']))
            continue
        dt = D(b['presented_media_ns'])-D(a['presented_media_ns'])
        intervals.append({'from':a['drawable'],'to':b['drawable'],'interval_ns':float(dt),
                          'refresh_periods':float(dt/period),'over_one_period':dt>period,'missed_refresh':int((dt/period).to_integral_value())>1,
                          'present_call_gap_ms':float((D(b['present_call_media_seconds'])-D(a['present_call_media_seconds']))*1000),
                          'presented_trace_ns':b['presented_trace_ns'],'yaw':b['yaw']})
    whole_start,pan_start,pan_end,whole_end = markers
    expected = [i for i,r in by_id.items() if whole_start <= D(r['media_seconds'])*NS-offset <= whole_end]
    for i in expected:
        if not any(v['drawable']==i for v in values):
            unmatched.append({'drawable':i,'reason':'whole-interval submission has no presented row'})
    retained = [int(r['handler_trace_ns']) for r in rows]
    coverage = bool(retained and values and min(retained)<=whole_start and max(retained)>=whole_end
                    and min(v['presented_trace_ns'] for v in values)<=whole_start
                    and max(v['presented_trace_ns'] for v in values)>=whole_end)
    affecting = []
    for missing in unmatched:
        previous = [v for v in values if v['drawable'] < missing['drawable']]
        following = [v for v in values if v['drawable'] > missing['drawable']]
        outside = ((previous and previous[-1]['presented_trace_ns'] > whole_end)
                   or (following and following[0]['presented_trace_ns'] < whole_start))
        missing['outside_marked_presented_interval'] = bool(outside)
        if not outside:
            affecting.append(missing)
    for interval in intervals:
        a = next(v for v in values if v['drawable']==interval['from'])
        interval['overlaps_whole'] = a['presented_trace_ns']<=whole_end and interval['presented_trace_ns']>=whole_start
    lag = [r['handler_lag_ms'] for r in values]
    # A bad scale/offset must not be accepted merely because intervals look regular.
    conversion_valid = bool(lag and min(lag)>=-float((high-low)/D(10**6))
                            and all(D(r['presented_media_ns']) >= D(r['opportunity_media_seconds'])*NS-(high-low) for r in values))
    return {'qualified':coverage and conversion_valid and not affecting and all(i['interval_ns']>0 for i in intervals),
            'coverage':coverage,'conversion_validated':conversion_valid,
            'timebase':{'numer':numer,'denom':denom},'reported_refresh_hz':hz,'refresh_period_ns':str(period),'offset_ns':[str(low),str(high)],
            'conversion_validation':'same-process marker brackets anchor trace origin; every converted time is between its game opportunity and OS handler, within clock error',
            'conversion':'presented_media_ns = (raw_at_time + marker_clock_offset_ns) * denom / numer',
            'marker_offset_bound_ns':str(high-low),'presented_media_error_bound_ns':str((high-low)*factor/2+D(1)),
            'markers_trace_ns':markers,'retained_handler_span_ns':[min(retained),max(retained)] if retained else [],
            'handler_lag_ms_range':[min(lag),max(lag)] if lag else [],
            'whole_submission_count':len(expected),'joined_rows':len(values),'unmatched':unmatched,
            'unmatched_affecting_marked_interval':affecting,
            'missed_refreshes':[i for i in intervals if i['missed_refresh']],
            'intervals_over_one_period':[i for i in intervals if i['over_one_period']],
            'intervals':intervals,'drawables':values,
            'limits':'OS-reported presented time; kernel display swaps, scan-out, photons and input-to-photon unmeasured'}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--input',type=Path,required=True,help='trace bundle, exported XML, or retained TSV.gz')
    p.add_argument('--arm',type=Path,required=True,help='game surface/requests/markers/window records')
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--refresh-hz',type=float,help='override only with separately retained display-mode evidence')
    a=p.parse_args()
    source=a.input
    if source.suffix=='.trace':
        source=a.output.with_suffix('.presented.xml')
        subprocess.run(['xcrun','xctrace','export','--input',str(a.input),'--output',str(source),'--xpath','/trace-toc/run/data/table[@schema="ca-client-presented-handler"]'],check=True,timeout=30)
    pid,low,high,markers=calibration(a.arm)
    numer,denom=timebase()
    mode = re.search(r'Some\((\d+)\)\)\)', (a.arm/'window.txt').read_text())
    hz = a.refresh_hz if a.refresh_hz is not None else int(mode[1])/1000 if mode else None
    if not hz or hz<=0:
        raise ValueError('reported display refresh unavailable; supply retained mode evidence')
    result=analyse(presented_rows(source,pid),table(a.arm/'surface.tsv'),table(a.arm/'requests.tsv'),low,high,markers,numer,denom,hz)
    result.update(pid=pid,input=str(a.input),arm=str(a.arm))
    a.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('intervals','drawables','unmatched','unmatched_affecting_marked_interval','intervals_over_one_period')},indent=2))
    return 0 if result['qualified'] else 2

if __name__=='__main__':
    raise SystemExit(main())
