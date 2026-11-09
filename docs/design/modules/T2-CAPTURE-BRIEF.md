# T2 unattended frame evidence brief

Owner request, 9 November 2026: visual and live-motion evidence for Mac qualification M2 and every later visual
milestone should run without the owner unlocking the console or granting Screen Recording permission wherever
possible. Mark genuinely attended proof with its reason. This is evidence tooling, not a change to the V1 exit.
The request concerns **milestone M2 on the M3 Max**, not an Apple M2 device.

T2 Diagnostics and delivery owns scenarios, evidence artifacts and provenance under
[TOOLS-DELIVERY](TOOLS-DELIVERY.md) and the [architecture map](../ARCHITECTURE-BRIEF.md#creation-and-delivery).
This child plan adds no crate or second rendering authority. V1/V4 (`thrum-render`) own image production/readback;
F1 (`thrum-app` and core) owns scheduling and platform lifetime; games own scenes and control bindings.
C02/C06/C07/C11/C12 and [storage rules](../../ASSET-LIFECYCLE.md) apply.

Deliver reproducible exact-build stills, fixed-time visual sequences and separately labelled real-time runtime
motion sequences on the named GPU/recipe. Preserve input, committed state, interpolation/history, frame identity,
clock domains and missing samples. Runtime capture must not turn a slow run into a smooth fixed-step movie.
Use the real render graph and temporal path, bounded asynchronous readback and bounded disk/encoding work.
Capture is opt-in; disabled capture adds no image copy, readback, allocation or synchronization to frames.

First prove a complete cooked noon pan on Mac while the console is locked and capture permission is unavailable.
Keep the world paused for the Mac M2 camera proof; also exercise a running world with interpolated presentation.
Review full frames, native-size crops and normal-speed motion; retain negative controls for timing and capture gaps.
Then qualify Chrome and iOS simulator consumers separately, without turning either into native panel evidence.
Every later visual milestone must name its required capture mode, surfaces, target and inspection list.

Physical panel cadence, compositor/native-widget appearance, OS input/focus and physical EDR/photon response need
separate proofs. Automate what can be automated; label attended setup only where an unlocked console, OS grant or
physical observation is genuinely required. Never bypass security, automatically unlock the Mac or silently
substitute an offscreen sequence for a display result. Preserve historical Mac evidence and leave its cadence open.

No numeric performance gate or paid tool is commissioned. Measure capture overhead, memory/bandwidth/disk scaling
and investigate visible hitches or comparable regressions under the existing performance policy. No implementation
or qualification is claimed by this planning task. Independent design/JSON review is owed; the owner forbids agents.
