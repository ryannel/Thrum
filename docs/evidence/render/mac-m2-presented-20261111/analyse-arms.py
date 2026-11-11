"""Reproduce marked-interval observer costs; host durations include waits, GPU queries are not presentation."""
import argparse
import csv
import json
from pathlib import Path
import re
import statistics


def table(path):
    with path.open() as stream:
        return list(csv.DictReader(stream, delimiter='\t'))


def summary(values):
    values=sorted(values)
    return {'count':len(values),'min':values[0],'median':statistics.median(values),
            'p95':values[min(len(values)-1,int(.95*len(values)))],'max':values[-1]} if values else {'count':0}


def analyse(arm):
    surface=table(arm/'surface.tsv')
    markers=table(arm/'signposts.tsv')
    start,end=float(markers[0]['media_before']),float(markers[-1]['media_after'])
    by_id={int(r['submission_id']):r for r in surface}
    wanted={i for i,r in by_id.items() if start<=float(r['media_seconds'])<=end}
    metrics={}
    for block in (arm/'gpu-samples.txt').read_text().split('FrameSample {')[1:]:
        ordinal=int(re.search(r'submission_order: (\d+)',block)[1])
        if ordinal in metrics:raise ValueError('duplicate GPU/CPU sample ordinal')
        row={}
        for field in ('render_cpu','encode_cpu'):
            m=re.search(r'\b'+field+r': ([\d.]+)(ns|µs|ms|s),',block)
            row[field+'_elapsed_ms']=float(m[1])*{'ns':1e-6,'µs':.001,'ms':1,'s':1000}[m[2]] if m else None
        m=re.search(r'gpu_pass_ns: Some\(\s*\[([^]]*)\]',block)
        row['gpu_scene_query_ms']=sum(map(float,re.findall(r'[\d.]+',m[1])))/1e6 if m else None
        for field in ('gpu_present_ns','gpu_contribution_ns'):
            m=re.search(field+r': Some\(\s*([\d.]+)',block)
            row[field+'_query_ms']=float(m[1])/1e6 if m else None
        metrics[ordinal]=row
    fields=next(iter(metrics.values())).keys()
    selected=[metrics[i] for i in wanted if i in metrics]
    result={'marked_surface_submissions':len(wanted),'marked_host_media_interval_seconds':[start,end],
            'sample_scope':'samples selected by joined game present-call time within Whole markers',
            'statistics':{f:summary([r[f] for r in selected if r[f] is not None]) for f in fields},
            'missing_sample_ordinals':sorted(wanted-metrics.keys()),
            'missing_gpu_scene_query_ordinals':sorted(i for i in wanted if i in metrics and metrics[i]['gpu_scene_query_ms'] is None),
            'load':json.loads((arm/'load.json').read_text())}
    if (arm/'witness.json').exists():
        witness=json.loads((arm/'witness.json').read_text())
        drawables={r['drawable']:r for r in witness['drawables']}
        result['marked_missed_refreshes']=[]
        for missed in witness['missed_refreshes']:
            if not missed['overlaps_whole']:continue
            record=dict(missed)
            a,b=drawables[missed['from']],drawables[missed['to']]
            record['opportunity_gap_ms']=(float(b['opportunity_media_seconds'])-float(a['opportunity_media_seconds']))*1000
            record['callback_target_gap_ms']=(float(b['callback_target_media_seconds'])-float(a['callback_target_media_seconds']))*1000
            record['from_sample']=metrics.get(missed['from'])
            record['to_sample']=metrics.get(missed['to'])
            record['host_load_scope']='see run-wide load/iostat; no exact instantaneous CPU attribution or isolated cause'
            result['marked_missed_refreshes'].append(record)
    return result


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    report={arm:analyse(args.root/arm) for arm in ('off-traced','on-untraced','on-traced') if (args.root/arm/'surface.tsv').exists()}
    report['limits']='Shared CPU load and run order confound observer comparisons. Host method durations include waits; GPU query durations, callback targets, image time, movie PTS and OS presented time are separate measurements. No causal overhead percentage or physical panel timing claim.'
    args.output.write_text(json.dumps(report,indent=2)+'\n')
