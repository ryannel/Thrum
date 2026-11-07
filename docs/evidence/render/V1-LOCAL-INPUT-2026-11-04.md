# V1 local input, 4 November 2026

The 10,000-entity fixture registers its last entity to one local peer and actor. Chrome and macOS keyboard events keep a current WASD/arrow-key state; the frame host samples its latest axes at each due fixed tick. Changed axes enter the existing `Simulation::admit_frame` validation and authority path before `Simulation::step`. The next tick holds those axes until another valid frame replaces them. One-shot actions remain empty. The local actor is outside the original 1,500 automatically moving entities.

| Target / check | Observation |
| --- | --- |
| Apple M3 Max / Metal, focused native host test | One rightward tick, one held tick after an invalid out-of-range input was refused, then one released tick. Position advanced 0.2 world units and stopped; no duplicate tick. The existing committed-frame/overload host regression also passed. |
| Chrome 156 WebGPU on Apple M3 Max, 1280 × 720 | Holding D during the live run changed the displayed committed player x position from 0.891 to 11.691. 601 ticks, 601 presented frames, 600 GPU-complete samples, zero unavailable frames, overloads, refused ticks, timestamp clamps, shed views or report drops. CPU tick p50/p95 200/400 µs; encode 200/300 µs. GPU values are timer-quantized, so they are not an input-cost comparison. |

The host creates no action payload or heap buffer when an axis changes. The new held-state comparison is three scalar bit comparisons per due tick. Key repeat does not resubmit unchanged browser or macOS controls. This slice does not measure input-to-display latency or establish allocation-free whole-frame execution. The Chrome scene is a synthetic visual diagnostic; macOS visible-window controls could not be tried while the Mac display was locked. iPhone touch controls and the four-peer conditioned transport remain open.
