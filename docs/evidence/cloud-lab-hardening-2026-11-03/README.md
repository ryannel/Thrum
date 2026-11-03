# Visible cloud lab and causal changed-weather proof

3 November 2026. [CLOUDS](../../design/modules/CLOUDS.md) governs; [migration](../../design/modules/CLOUD-MIGRATION.md) is the only active queue. This checkpoint hardens the running finite Cu instrument and exercises two live weather changes. **It does not accept the smooth crown anatomy, complete CF01, Cb, whole atmosphere or full-sky/device budgets.**

## Exact runnable build

```sh
python3 docs/evidence/cloud-lab-hardening-2026-11-03/restore.py --output tmp/cloud-lab-review --test
python3 tmp/cloud-lab-review/serve.py --port 8157
```

Open `http://localhost:8157/`. The actual observed build is **c3e0ebee9680337c**, [39 served-file pins and archive](build.json). It opens paused at source100s with a visible growing crown. Clear birth remains at0s. Review labels name actual source stages; the unsupported nonquarter SourceClock now rejects construction rather than seeking to an offset time. Direct-model refinement is a different test. Source model, shaders, ingredients and optical calibration remain identical to73b in the [prior pilot](../cloud-system-pilot-2026-11-03/README.md).

The desktop scene is fully visible beside a scrollable controls pane; root inspected the actual1280×720 UI and native image. Backing1280×720 remains distinct from896.4×504.2 CSS scene pixels/DPR2. Native orbit and wheel zoom were responsive while paused; source time/tick and pose/control upload counts stayed unchanged. Shift-pan/native pinch and the narrow responsive layout are not visually qualified here. Native captures are scene-only, not UI screenshots.

A cold earlier78cc build spent over60s in WebGPU initialization/compilation, then completed. Its three changed UI files and full identity are retained in the archive's `checkpoints/` directory. It was not a permanent lockup. c3e0 reports elapsed initialization stage and real replay progress; exact cause/substage of the earlier cold delay is unqualified. No timeout or asynchronous device leak was added. Adapter receipts now explicitly copy non-enumerable fields: vendor `apple`, architecture `metal-3`, fallback false, timestamp-query enabled; device/description remain unavailable. This cannot retrospectively identify adapters in older `{}` receipts or certify a target GPU.

Four native R16 dependencies share the [component checkpoint](../cloud-component-proof-2026-11-03/README.md) by exact decoded SHA. The archive includes complete served code/noise/metadata, scoped tests, server and historical startup checkpoint. The admission-preparation archive retains its existing producer. `restore.py` verifies manifest, archive, dependencies and every served file into a fresh output. Root restored it afresh:8 renderer tests and all8 source-suite groups passed. The baseline itself remains unchanged by the separately evaluated CPU/GPU optimisations.

## Actual forcing and motion

[Programme](analysis/programme.json) pins full controls entered as two future commands at browser source100s. At150s mean wind turns fromX toZ, shear turns fromX toZ and uplift changes; dry gain is zero. At500s an explicit front is moved toX522m with35m/s speed,800m width and gain1. Existing matter receives those causes; no clip or replacement source is loaded. The recorder's `recorded-fixed-controls` label means a fixed queued programme, not constant weather.

- [Actual normal1x130→220 recording](observations/system-renewal-1x-90s-c3e0ebee9680337c-eea93588-7.webm),1,349 decoded frames. [Ten-second native crops](observations/system-renewal-1x-90s-c3e0ebee9680337c-eea93588-7.contact.png) show new heads emerging as old heads recede; no sampled wind-change snap is apparent. World focus is fixed once at the start; it does not follow the source bounds. Heads remain broad and changes retain some size-pulse character.
- [Actual16x0→640 recording](observations/system-accelerated-16x-40s-c3e0ebee9680337c-eea93588-10.webm),600 decoded frames. [Four-second samples](observations/system-accelerated-16x-40s-c3e0ebee9680337c-eea93588-10.contact.png) span repeated renewal and late front loss. This view explicitly follows weatherR. [Native end-window samples](observations/changed-front-tail.contact.png) show clear sky at approximately576/608/624s; the retained end receipt proves all14 parts retired and none active at640s. Visible clear and exact source zero are distinct observations.

Root assessed decoded temporal samples and native PNGs, not uninterrupted watched playback. Encoder startup block artifacts remain in the original clips; no encoded bytes were dropped. Full recorded-window `.analysis.json` files use all frame PTS and all retained submissions; neither ring overwrites records. Observed submission median/p95 is35.76/47.57ms normal and31.39/44.34ms accelerated. Queue fences and source backlog are separate; these are observer-on runs, not quiet GPU execution, scanout FPS or target admission.

The [independent analytic CPU replay](analysis/cpu-replay.json) checks R_X=521.76m after150s and R_Z=6.3912(t−150). The actual220.002215s browser BODY pose matches that path within3.6e−12m in host arithmetic. Surviving old BODY retains K_X=.948; a new170s crown starts K_X=0. No future-front exposure acts before500s. [Portable runner](analysis/run-reproduction.py) replays the original producer in a fresh sibling layout:

```sh
python3 docs/evidence/cloud-lab-hardening-2026-11-03/analysis/run-reproduction.py --runtime tmp/cloud-lab-review --output tmp/cloud-forcing-review-replay
```

Root ran this fresh and obtained14 births/0 final active. Numerical agreement does not establish meteorological fidelity.

## Consumers, near views and remaining rejection

At changed wind/source220s and8km, [main+mirror+receiver](observations/system-t220000-mode0-c3e0ebee9680337c-eea93588-11.png) and [main hidden](observations/system-t220000-mode0-c3e0ebee9680337c-eea93588-12.png) retain the independently traced mirror and receiver shadow. The common tuple/source time, camera and lighting are unchanged. Actual native GPU/CPU probes at startup100s and changed-wind220.002215s pass the existing experimental .002q envelope, with no invalid samples. This proves this bounded retained-consumer case, not production rough water, environment capture or all atmosphere paths.

Root also inspected native source220s at2km/384rays from [front](observations/system-t220000-mode0-c3e0ebee9680337c-eea93588-4.png), [side](observations/system-t220000-mode0-c3e0ebee9680337c-eea93588-5.png) and [oblique](observations/system-t220000-mode0-c3e0ebee9680337c-eea93588-6.png). The head is still too oval/smooth, with an upright side wall. More ray samples do not supply the missing geometric structure. Keep these v2 ingredients as a coupling/cost reference; final crown content and dynamic art remain on the critical path. Changing weather is functional, but the full requested cloud appearance has not passed.
