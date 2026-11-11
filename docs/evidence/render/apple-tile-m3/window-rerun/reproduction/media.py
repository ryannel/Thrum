from pathlib import Path
import json,subprocess,hashlib,math
r=Path('tmp/g3t-m3-mf1');out=Path('docs/evidence/render/apple-tile-m3/window-rerun');(out/'strips').mkdir(exist_ok=True);report={};strips={}
def probe(p):return json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-show_packets','-show_data_hash','sha256','-show_streams','-show_format','-show_entries','packet=pts_time,data_hash:stream=codec_name,width,height,time_base:format=duration','-of','json',str(p)]))
for recipe in ['portable','apple-tile']:
 src=r/f'{recipe}-interactive.mov';dst=out/'interactive'/f'{recipe}.mov';obs=json.loads((r/'interactive'/f'{recipe}-restored.json').read_text());start=(r/f'{recipe}-record-start').stat().st_mtime_ns;end=math.ceil((obs['unix_ns']-start)/1e9)+2
 assert end<75
 command=['ffmpeg','-v','error','-y','-copyts','-i',str(src),'-t',str(end),'-map','0:v:0','-c','copy','-avoid_negative_ts','disabled','-video_track_timescale','600',str(dst)]
 subprocess.run(command,check=True)
 a=probe(src);b=probe(dst);pa=a['packets'];pb=b['packets'];assert pb==pa[:len(pb)],(recipe,len(pa),len(pb),pa[:2],pb[:2])
 report[recipe]={'source':str(src),'source_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'source_bytes':src.stat().st_size,'retained':str(dst.relative_to(out)),'retained_sha256':hashlib.sha256(dst.read_bytes()).hexdigest(),'retained_bytes':dst.stat().st_size,'source_streams':a['streams'],'retained_streams':b['streams'],'source_duration':a['format']['duration'],'retained_duration':b['format']['duration'],'source_packets':len(pa),'retained_packets':len(pb),'all_retained_packets_equal_source_prefix':True,'requested_excerpt_end_seconds':end,'restored_observation_seconds_from_start_signal':(obs['unix_ns']-start)/1e9,'command':command,'scope':'Original-time prefix covering all input/resize/restoration, no video reencode; full OS original remains in scratch. Packet SHA-256 and PTS match the source prefix exactly.'}
 old=out/'interactive'/f'{recipe}.mp4'
 if old.exists():old.unlink()
 for label,p,fps in [('controls',r/'window'/recipe/'control-response.mov',10),('recovery',r/'window'/f'{recipe}-recovery-settled-video/window-response.mov',10),('interactive',src,2)]:
  key=f'{recipe}-{label}';pattern=out/'strips'/f'{key}-%02d.png'
  subprocess.run(['ffmpeg','-v','error','-y','-threads','1','-i',str(p),'-vf',f'fps={fps},scale=320:-1,tile=5x10','-fps_mode','vfr',str(pattern)],check=True)
  strips[key]={'source':str(p),'fps':fps,'layout':[5,10],'reading_order':'left to right then top to bottom','seconds_per_full_sheet':50/fps,'sheets':[str(x.relative_to(out)) for x in sorted((out/'strips').glob(key+'-*.png'))],'scope':'Complete movie sampled in OS host time; not playback or panel cadence'}
(out/'media.json').write_text(json.dumps(report,indent=2)+'\n');(out/'strip-index.json').write_text(json.dumps(strips,indent=2)+'\n');print({k:(v['retained_duration'],v['all_retained_packets_equal_source_prefix']) for k,v in report.items()})
