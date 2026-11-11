# Mac M2 presented-time qualification, 11 November

Re-plan `ab66bca5` replaces the silent kernel display tables with OS-reported presented time per game drawable.
The new six-recording commission is distinct from the exhausted 9 November commission; both journals are retained.

The [witness command](../../../../scripts/mac_m2_presented.py) resolves XML references and filters the game PID,
uses the `at-time` field, and joins drawable ordinals to game submissions. It never uses handler callback time
or `delta` as presented time. Same-process signpost brackets give trace-origin offset; this Mac reports mach
timebase numer/denom 125/3. Undo the export’s scaling and origin subtraction with
`presented_media_ns = (raw_at_time + marker_offset_ns) * 3 / 125`. Each converted value is validated between its
joined game opportunity and OS handler, against that independently bracketed origin. The marker-offset and
converted-value error bounds are reported separately. [Apple documents the underlying property](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime)
as onscreen host time, with zero for unpresented/dropped frames. The export-unit inversion above is an observed
quirk validated on this Mac. Values are OS witnesses, not kernel swaps or photons.

All unmatched rows remain listed. Only neighboring valid OS presented rows can bound an unmatched ordinal
entirely before or after the marked interval; an unbounded/internal missing row prevents qualification. Both
handler and converted presented rows must span the whole markers. No missing value is filled from engine time.

The known-answer command finds OFF 526→527 (16.665250176 ms), OFF 544→545 (16.666624896 ms), and ON 558→559
(16.665415680 ms). OFF 544→545 joins the 15.470792 ms game present-call gap. The command refuses both historical
traces for full-interval cadence because their retained rows start after the pan began.
[OFF analysis](retained-off-witness.json.gz), [ON analysis](retained-on-witness.json.gz), [targeted tests](tests.log).

```sh
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-off-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111/off-traced --output tmp/mac-m2/retained-off-witness.json
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-on-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111 --output tmp/mac-m2/retained-on-witness.json
python3 -m unittest discover -s scripts -p test_mac_m2_presented.py -v
```

The first two commands intentionally exit 2 with `coverage: false`; their interval findings remain readable.
Tests cover known misses, truncated coverage, zero/absent time, unfamiliar status, missing/duplicate ordinal,
incorrect conversion, and XML reference/PID attribution. A complete synthetic interval is accepted as a control.

The first explicit-window recording retained both markers but hit its process deadline. Its data is diagnostic
evidence, not a bounded-completion pass. The changed configuration uses the installed Blank template plus Metal
Application, Display and Points of Interest. It is pending the shared GPU lock. The collector pins the retained M2.1 release binary,
preset and cooked-store hashes and uses explicit `--window 16s`, a 16-second record cap, 75-second finalization
cap and 30-second export deadlines. Each arm records iostat, process/load snapshots and continuous panel witnesses.
A locked or sleeping panel stops the owned work. No security setting or Screen Recording grant is requested.

Kernel display swaps, scan-out, photons, input-to-photon and physical iPhone timing remain unmeasured.

[Attended collection and inspection commands](RUNBOOK.md).

The first explicit-window attempt recorded 8.704884 seconds, PID 21493, 842 game submissions and 840 OS handler
rows. Both converted presented times and handler rows span the four markers. Status 1 at drawable 1 and missing
terminal drawable 841 are reported, bounded outside the marked presented interval by neighboring valid OS rows.
The whole interval has 491 game submissions and five two-refresh intervals in the complete run (three overlap
the marked interval). The trace is diagnostic only because its process exceeded the 91-second overall cap.
Background one-minute load was 3.245→4.697 over collection/finalization; the retained iostat and lifetime process
snapshot are host observations, not evidence for any specific cause.
[First attempt](attempt-1-off/invocation.json), [bounded failure](attempt-1-off/collection.json),
[converted witness](attempt-1-off/witness.json.gz), [row counts](attempt-1-off/row-counts.json),
[load](attempt-1-off/load.json) and [commission](trace-commission.json).

[First-attempt marked CPU/GPU samples and miss investigations](attempt-1-costs.json.gz) are reproducible with
`python3 docs/evidence/render/mac-m2-presented-20261111/analyse-arms.py --root tmp/mac-m2/presented-20261111 --output tmp/mac-m2/attempt1-costs.json`.
These are diagnostic observations. Host render-method elapsed durations include waits; GPU query intervals
do not locate display presentation. All strict over-one-period values remain in the witness, while
nearest-grid two-refresh intervals are listed separately. No numeric performance threshold is a gate.

Attempt 2 completed collection and exports in 44.663 seconds but kept only 130 handler rows, from 7.582904 to
8.657917 trace seconds, while Whole spans 4.544904–8.645149 seconds. It is refused for incomplete coverage.
All 360 unjoined whole-interval submissions remain listed. The panel stayed unlocked/awake.
[Attempt 2](attempt-2-off/invocation.json), [coverage refusal](attempt-2-off/witness.json.gz),
[display counts](attempt-2-off/row-counts.json) and [load](attempt-2-off/load.json).
The next changed hypothesis removes `--window` entirely while retaining the smaller instrument set.
