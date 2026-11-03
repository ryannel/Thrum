from pathlib import Path
import json,statistics,subprocess,hashlib
root=Path.cwd(); out=root/'docs/evidence/cloud-cohort-pilot-2026-11-03'; obs=out/'observations'
results=[]
for p in sorted(obs.glob('*.webm')):
 j=json.loads(p.with_suffix('.json').read_text());m=j['motion'];trace=j['submissionTrace']
 frames=json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-show_entries','frame=best_effort_timestamp_time','-of','json',str(p)],text=True))['frames']; ts=[float(f['best_effort_timestamp_time']) for f in frames]
 subprocess.run(['ffmpeg','-v','error','-err_detect','explode','-i',str(p),'-fps_mode','passthrough','-enc_time_base','demux','-f','null','-'],check=True)
 def dist(a):
  a=sorted(a);return {k:a[min(len(a)-1,int(v*len(a)))] for k,v in [('p50',.5),('p95',.95),('max',1)]}
 results.append({'file':p.name,'build':j['build'],'source_window':m['sourceWindow'],'wall_ms':m['actualRecorderWallMs'],'outcome':m['outcome'],'numerical_purpose_gate':m['purposeGate'],'decoded_frames':len(ts),'first_last_pts':ts[::max(1,len(ts)-1)],'encoded_fps_from_pts':(len(ts)-1)/(ts[-1]-ts[0]),'full_ffmpeg_decode_exit':0,'dropped_encoded_bytes':m['droppedEncodedBytes'],'retained_submission_count':len(trace),'overwritten_submission_records':j['overwrittenSubmissionTrace'],'retained_submission_wall_span_ms':trace[-1]['submittedMs']-trace[0]['submittedMs'],'retained_submission_cadence_ms':dist([b['submittedMs']-a['submittedMs'] for a,b in zip(trace,trace[1:])]),'retained_queue_fence_elapsed_ms':dist([f['completedMs']-f['submittedMs'] for f in trace]),'source_cpu_delta_ms':m['endAtStopRequest']['clock']['sourceCpuMs']-m['start']['clock']['sourceCpuMs'],'logical_gpu_bytes_at_end':m['endAtStopRequest']['lastCompletedFrame']['logicalGpuBytes'],'host':j['host'],'limits':'Only latest240 submissions retained, not full60s/90s timing campaign; elapsed fence is not timestamp GPU execution or display latency. MediaRecorder15fps is separate from callback cadence. Logical bytes are not process/device residency. Source endpoint is stop-request tuple, not proven encoded-frame source endpoint. No M2/5ms/512MiB admission.'})
(out/'motion-analysis.json').write_text(json.dumps(results,indent=2)+'\n')
