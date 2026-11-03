from pathlib import Path
import json,subprocess,sys,statistics
p=Path(sys.argv[1]);x=json.loads(p.read_text());m=x['motion'];clip=p.with_name(m['file'])
f=json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-show_entries','frame=best_effort_timestamp_time','-of','json',str(clip)]));pts=[float(a['best_effort_timestamp_time']) for a in f['frames'] if 'best_effort_timestamp_time' in a]
def describe(v):
 v=sorted(v)
 if not v:return None
 return {'count':len(v),'p50':statistics.median(v),'p95':v[min(len(v)-1,int((len(v)-1)*.95))],'p99':v[min(len(v)-1,int((len(v)-1)*.99))],'max':max(v)}
start=m['start']['wallPerformanceMs'];end=m['endAtStopRequest']['wallPerformanceMs'];tr=[v for v in x['submissionTrace'] if start<=v['submitted']<=end];v=[v for v in tr if v['sourceTime']>=m['sourceWindow'][0]-.001]
result={'build':x['build'],'clip':m['file'],'sourceWindow':m['sourceWindow'],'recorderWallMs':m['actualRecorderWallMs'],'decodedFrames':len(pts),'encodedPTSRangeSeconds':[min(pts),max(pts)],'encodedFrameGapMs':describe([1000*(b-a) for a,b in zip(pts,pts[1:])]),'retainedSubmissionRecordsAll':len(x['submissionTrace']),'recordingWindowSubmissions':len(tr),'submissionIntervalMs':describe([b['submitted']-a['submitted'] for a,b in zip(tr,tr[1:])]),'queueFenceElapsedMs':describe([a['completed']-a['submitted'] for a in tr]),'sourceBacklogSeconds':describe([max(0,a['requestedTime']-a['committedTime']) for a in tr]),'traceOverwrittenSinceStart':x['traceOverwrittenSinceStart'],'overwrittenNotAllowedToInferFullWindow':x['traceOverwrittenSinceStart']>0,'droppedEncodedBytes':m['droppedEncodedBytes'],'scope':'Recorded/observer-on run, source simulations and queue-fence timing; no quiet GPU execution, scanout, process-residency or target-device admission. All decoded frame PTS contribute to this summary; individual PTS are not retained here. No continuous watched-motion claim.'}
out=p.with_suffix('.analysis.json');out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
