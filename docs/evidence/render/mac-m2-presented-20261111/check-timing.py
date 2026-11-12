import csv, json, statistics
from pathlib import Path

def table(p):
    return list(csv.DictReader(p.open(), delimiter='\t'))

def analyse(p):
    markers=table(p/'signposts.tsv'); controls=table(p/'controls.tsv'); req=table(p/'requests.tsv'); surface=table(p/'surface.tsv')
    byseq={int(r['sequence']):r for r in req}
    joined=[dict(r,yaw=float(byseq[int(r['sequence'])]['accepted_yaw_degrees'])) for r in surface]
    start,end=float(markers[0]['media_before']),float(markers[-1]['media_after'])
    changing=[r for r in controls if 0<float(r['requested_yaw_degrees'])<36]
    origins=[float(r['media_seconds'])-float(r['requested_yaw_degrees'])/18 for r in changing]
    zero=[r for r in joined if abs(r['yaw'])<.001]; final=[r for r in joined if abs(r['yaw']-36)<.001]
    hold0=float(zero[-1]['media_seconds'])-float(zero[0]['media_seconds'])
    hold1=float(final[-1]['media_seconds'])-float(final[0]['media_seconds'])
    changing_submitted=[r for r in joined if .001<r['yaw']<35.999]
    sweep_span=float(final[0]['media_seconds'])-float(zero[-1]['media_seconds'])
    initial_marked_hold=float(changing_submitted[0]['media_seconds'])-start
    final_marked_hold=end-float(final[0]['media_seconds'])
    source={}
    if (p/'window-manifest.json').exists():
        manifest=json.loads((p/'window-manifest.json').read_text()); records=manifest['records']
        pairs=[(a,b) for a,b in zip(records,records[1:]) if .001<a['accepted_yaw_degrees']<35.999]
        source={'original_count':len(records),'ticks':sorted({r['tick'] for r in records}),'generation_tuples':sorted({(r['device_generation'],r['presenter_generation'],r['lease_generation']) for r in records}),'yaw_reversals':sum(b['accepted_yaw_degrees']<a['accepted_yaw_degrees']-.001 for a,b in zip(records,records[1:])),'missing_submissions':manifest['missing_submissions'],
            'repeated_nonendpoint_pairs':[{'from':a['submission_id'],'to':b['submission_id'],'duration_ms':(b['submit_ns']-a['submit_ns'])/1e6} for a,b in pairs if a['rgba_sha256']==b['rgba_sha256']],
            'nonendpoint_source_gaps_over_16ms':[{'from':a['submission_id'],'to':b['submission_id'],'duration_ms':(b['submit_ns']-a['submit_ns'])/1e6} for a,b in pairs if b['submit_ns']-a['submit_ns']>16000000],
            'max_source_gap_ms':max((b['submit_ns']-a['submit_ns'])/1e6 for a,b in zip(records,records[1:]))}
    steps=[(float(b['media_seconds'])-float(a['media_seconds']))*1000 for a,b in zip(joined,joined[1:])]
    return {'source_observations':source,'marker_durations_s':[float(b['media_before'])-float(a['media_before']) for a,b in zip(markers,markers[1:])],
        'marker_brackets':markers,'submitted_initial_yaw0_hold_s':hold0,'submitted_final_yaw36_hold_s':hold1,
        'last_yaw0_to_first_yaw36_submission_span_s':sweep_span,'whole_begin_to_first_changing_submission_s':initial_marked_hold,'first_final_submission_to_whole_end_s':final_marked_hold,
        'submitted_yaw_range':[min(r['yaw'] for r in joined),max(r['yaw'] for r in joined)],
        'requested_nonendpoint_count':len(changing),'inferred_nominal_18_degree_per_second_origin_range_s':[min(origins),max(origins)],
        'max_surface_call_gap_ms':max(steps),'marked_surface_submissions':sum(start<=float(r['media_seconds'])<=end for r in joined),
        'interpretation':'The diagnostic computes yaw=clamp((elapsed-4)/2)*36, sampled by AppKit actions. Markers are emitted on the first step crossing elapsed thresholds; their durations include polling quantization and are reported, not rounded to exact seconds. Initial warmup at yaw0 supplies >1 second of settled output; final submitted yaw36 hold is measured separately. The nominal two-second schedule and submitted endpoint coverage are distinct from exact marker duration.'}
root=Path('tmp/mac-m2/presented-deferred-20261111')
(root/'timing-review.json').write_text(json.dumps({p:analyse(root/p) for p in ('off-traced','on-untraced','on-traced') if (root/p/'surface.tsv').exists()},indent=2)+'\n')
