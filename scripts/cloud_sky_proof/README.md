# Complete-sky proof instruments

Standalone experiments for the [cloud design](../../docs/design/modules/CLOUDS.md), not a Thrum runtime.
[Selected R0 evidence and first rejected Cu image](../../docs/evidence/cloud-sky-r0-2026-11-01/README.md)
own the actual result. Five-family/weather/full-sky qualification remains incomplete.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/cloud_sky_proof -p 'test_*.py'
node scripts/cloud_sky_proof/test_convective.mjs
node scripts/cloud_sky_proof/test_clock.mjs
python3 scripts/cloud_sky_proof/reference_packing.py --output scripts/cloud_sky_proof/fixtures/packing.json
python3 scripts/cloud_sky_proof/optical_reference.py --json > scripts/cloud_sky_proof/fixtures/optics.json
python3 scripts/cloud_sky_proof/serve.py
```

The Python references use NumPy; the observed interpreter is `/opt/homebrew/bin/python3.14` with the existing
installation. No package install is required. Pure JS source/clock tests use Node. The host freezes its selected
source bytes/manifest at startup; restart it after edits and reload the page. A stale HTML build cannot run under
a different host manifest. This is a bounded reproducibility tool, not Thrum's future asset-admission service.

- `http://localhost:8143/scripts/cloud_sky_proof/packing.html`: Run performs actual WebGPU packing/guards/half/mip,
  translated anisotropic world sampling and finite Beer comparisons against independently generated references.
- `http://localhost:8143/scripts/cloud_sky_proof/cu.html`: **unaccepted** Cu lifecycle candidate. Four recipes, exact
  age/1x/16x/64x, coalesced orbit/pan/pinch, native1280x720, receiver shadow and independent mirror, source probes.
  Its vacuum/constant-external-sky,64cube point-tau/8cube five-axis ambient/constant receiver diffuse fixture is
  narrower than SKY-G. It neither qualifies the atmosphere nor the complete clouded surface environment.

`convective.js` owns a finite optical source graph, CPU evaluator, packed frame and WGSL evaluator. It is an
**appearance failure in its current coarse form**, retained for diagnosis. Numerical birth omits support below1µm
with an explicit optical bound; radii/centres have the same float32 numerical definition on CPU and GPU.
`clock.js` preserves elapsed old-rate time when changing speed, independently of the last rendered frame.

Captures go only to owned `tmp/cloud-sky-2026-11-01/captures/`, with bounded names/body sizes; selected evidence is
promoted separately. Buttons pause/capture a latched source state. Paused scenes do no background rendering. Actual
canonical coefficient probes do not qualify sampled fields, optical paths, appearance or performance. Frame GPU
compute brackets/queue observations are diagnostic; no device or full-frame admission follows from a single frame.
The earlier best verified coupled proof remains separately paused on port8142.
