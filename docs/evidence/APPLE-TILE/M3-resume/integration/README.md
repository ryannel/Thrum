# M3 integrated target proof

Merged source is `8bf0515a40309ea61b058a40145a51c690e644ae`. The official merge returned `MERGED`
after `./dev check` passed; its [result](checks/first-merge.log.gz) records the upstream changes.
The [source list](upstream-files.json) includes CULLING's frustum default and forward shader changes.
All refreshed target runs held the repository GPU lock. No engine code changed in this follow-up.

The [attribution audit](reuse-audit.json) ties the actual published SEA M1 oracle run at `43caeacc` to
the integrated covered-source key `cd206e937b398b54ce891846d019ab228499139d5228bd002ad570a6c25fbe14`.
The [exact record](exact-record.json) passes 38 native linear/SDR comparisons, four native surface-format/
letterbox comparisons and 1,924 Chrome linear/SDR records. The rebuilt native executable is byte-identical
(`6866a296645dff4a70f64942565d1810c19b5c8b38f880ad54fdddc7357f22a6`); all rebuilt browser artifact hashes
also match. This is attributed reuse of real same-target measurements, not a new verifier execution.

The fresh [capture report](capture-report.json) pins this merged revision, artifact hashes, commands and
actual Metal adapter/recipe identity. All six native realtime sequences complete: portable/Apple tile route
959/964 images, variants 481/481 and injected stall 450/450. Chrome day/night/resize complete with
481/475/481 source images; route histories advance 1→2, and resize presenter generations advance 1→2→3.
Two input, five timing and eight optional-layout tests pass in the retained [test logs](checks/input-tests.log.gz).
The [actual timing audit](timing-audit.json), produced by [audit.py](audit.py), confirms named maps,
240 star-present/240 absent samples per variants run, matching separate present values and explicit
unavailable upload, output-readback and query counters. Clock domains remain separate.

Builder inspected every chronological strip across all nine runs, the full-resolution portable cut frames
770/771, Apple night frame 479, Chrome night post-cut frame 441 and the 1000×800 composed canvas.
Terrain, mirror, rock, cubes, shadows and local light remain through approach/cut; night stars and silhouettes
remain. The inherited Chrome night light saturation matches its exact oracle. Stalls resume the pan, and
resize retains centered letterboxing and source proportions. A partial final resize strip has padding after
its final sample. Original-time previews and selected lossless sources accompany the strips. No physical
cadence, performance, traffic, current OS-window or physical-iPhone qualification follows from these runs.

The [recipe](capture.py) and [ownership record](OWNERSHIP.md) identify reproduction and retention. Bulk source
spools remain deliberately in `tmp/apple-tile-m3-integrated/captures` through independent review; selected
records and images are committed here. Final milestone and repository checks are recorded by the official loop.
