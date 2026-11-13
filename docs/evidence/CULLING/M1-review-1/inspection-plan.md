# Supplemental visual inspection plan

Recorded before opening the reviewer's captures. M1 has no `inspect` proofs; its visual acceptance is byte equality. These fixed-time endpoint images corroborate that the real mixed-mesh scene ran, and do not qualify elapsed-time motion or panel cadence.

- Inspect native 640×360 frames on both sides of the first snap turn, later turns and the first cut: walls, props, mesh silhouettes and frame edges should remain fully drawn, with no obvious holes.
- Compare adjacent frames at the first cut and a turn endpoint. Distinguish the intentional camera discontinuity from object disappearance.
- Confirm mixed props and cover are actually visible. Exact equality and hidden-work counts are judged by the every-tick GPU check, not inferred from pictures.
- For the native retained-image matrix, inspect an independently generated contact sheet for nonempty output and the intended scene. Its complete byte comparison remains the proof of unchanged baselines.

Target: M3 Max Metal; code 9cba741733015cc6608322bc505a70a0b138ed0f, builder 4c2690bb7e0f. Low sun/storm are exercised by the native matrix where configured; distance and fast camera by the field route. Full-refresh panel cadence is outside M1 and is not claimed by these paced readbacks.
