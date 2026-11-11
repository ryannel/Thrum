exec(open('tmp/g3t-m3-mf1/run.py').read().split("for n in ['window-references.py'")[0])
for recipe in ['portable','apple-tile']:run(['python3',str(r/'interactive.py'),recipe])
(r/'interactive-video-complete').touch()
(r/'playback-ready').touch()
start=time.monotonic()
while not (r/'playback-done').exists():
 preflight()
 if time.monotonic()-start>600:raise RuntimeError('Playback review timed out')
 time.sleep(.5)
