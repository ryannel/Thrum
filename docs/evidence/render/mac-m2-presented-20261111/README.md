# Mac M2 presented-time qualification, 11 November

**M2 remains open. M2.1, M2.3, M2.4 and M2.5 have builder proof. M2.2 is collecting its matched ON arms
on the resumed awake-panel sitting; the traced ON arm is queued for GPU.** The earlier stop remains history: At 14:10:01 UTC the continuous queue guard observed display 1 inactive/asleep while
the console stayed unlocked. It cancelled only the owned request before GPU acquisition or recording.
[Stop record](deferred-queue/queue-blocked.json), [panel witnesses](deferred-queue/panel-witness.jsonl.gz),
[guard source](deferred-queue/guard.py) and [queued-request exit](deferred-queue/collector.log).

Two of the six recordings granted by reviewed re-plan `ab66bca5` are used; four remain. The exhausted 9 November
commission is separate history. No Developer mode, Instruments authorization or Screen Recording change is needed.
[Commission](trace-commission.json) · [one-command attended runbook](RUNBOOK.md).

## Unattended witness command: M2.4

The [command](../../../../scripts/mac_m2_presented.py) resolves XML references, filters the game PID and uses
`ca-client-presented-handler.at-time`. Same-process marker brackets and the independently read mach timebase
125/3 invert the observed export units:
`presented_media_ns = (raw_at_time + marker_offset_ns) * 3 / 125`. Results state origin and converted-value
uncertainty separately. Converted times must fall between game opportunity and OS handler.
[Apple’s property](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime) is onscreen host
time; the export inversion is validated here. Callback time and `delta` are never presented time.

| Retained known-answer interval | OS interval | Game post-present record gap |
| --- | ---: | ---: |
| OFF 526→527 | 16.665250176 ms | 8.303958 ms |
| OFF 544→545 | 16.666624896 ms | 15.470792 ms |
| ON 558→559 | 16.665415680 ms | 8.772792 ms |

Both historical traces are refused because retained rows start after the pan began.
[OFF result](retained-off-witness.json.gz), [ON result](retained-on-witness.json.gz), [14 targeted tests](tests.log),
[CPU-only build](test-build.log.gz). Tests cover known misses, coverage, conversion, XML/PID references, invalid
rows, unmatched scope and finalization deadline. Unmatched rows stay listed; only valid OS neighbors can bound
one outside Whole. Internal/unbounded missing rows refuse qualification. No engine time fills missing OS data.

```sh
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-off-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111/off-traced --output tmp/mac-m2/retained-off-witness.json
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-on-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111 --output tmp/mac-m2/retained-on-witness.json
python3 -m unittest discover -s scripts -p 'test_mac_m2*.py' -v
```

The first two commands intentionally exit 2 (`coverage: false`). For a fresh trace, use `--input <arm>/pan.trace`
and `--arm <arm>` with retained game/marker records. Export has a 30-second timeout.

## Fresh attempts: M2.5 and M2.2 stay open

| Attempt | Configuration | Outcome |
| --- | --- | --- |
| 1, OFF | Game Performance, explicit 16 s window | Full marked coverage, 840 handler rows / 842 game records; process hit 91 s deadline. Diagnostic only. |
| 2, OFF | Blank + Metal Application, Display, Points of Interest; explicit 16 s window | Bounded completion/exports in 44.663 s, but only 130 handler rows spanning 7.582904–8.657917 s against Whole 4.544904–8.645149 s. Refused. |
| Queued next, unconsumed | Same smaller set, omit `--window`; installed Blank declares `windowLimit=0` | Cancelled on panel sleep before launch. Configuration remains untested. |

[Attempt 1 invocation](attempt-1-off/invocation.json), [failure](attempt-1-off/collection.json),
[witness](attempt-1-off/witness.json.gz), [display counts](attempt-1-off/row-counts.json), [load](attempt-1-off/load.json).
Attempt 1 reports unfamiliar status at drawable 1 and missing terminal drawable 841 outside the marked OS
interval. Five two-refresh intervals exist, three overlapping Whole. Its deadline failure prevents acceptance.
[Attempt 2 invocation](attempt-2-off/invocation.json), [coverage refusal](attempt-2-off/witness.json.gz),
[display counts](attempt-2-off/row-counts.json), [load](attempt-2-off/load.json). All 360 unjoined Whole submissions
are reported. Both recordings identify only built-in display 1 and retain unlocked/awake panel witnesses.

GPU covered each recording/export batch. Per-arm iostat, load and lifetime process snapshots are retained.
[CPU/GPU samples and miss investigations](attempt-1-costs.json.gz) are reproduced by [the script](analyse-arms.py).
Every strict over-period interval is listed; two-refresh intervals are identified separately.
[OS request comparison](attempt-1-request-comparison.json) has 842 requests versus 842 game records, with median
0.522 ms lag. It is neither presented time nor raw drawable identity. No isolated causal overhead is claimed.

The pinned binary `aa1591522962933b55ec1f347cc538880f5eb14b7ea227bf4adcb7f00b585507`, preset and cooked store
match [M2.1 motion](../mac-m2-20261110/README.md#runtime-motion) and
[M2.3 window proof](../mac-m2-20261110/README.md#real-cametallayer-tap). Exact collector sources/hashes are retained.
Mac M3 owns post-APPLE-TILE requalification. Fresh ON arms and 1× review remain owed. Callback targets, game
records, GPU queries/completion observations, image time, movie PTS and OS presented time are distinct.
Acquisition has no separate absolute timestamp here. Kernel swaps, scan-out, photons, input-to-photon and
physical iPhone timing remain unmeasured.

Local full traces remain under `tmp/mac-m2/{presented,presented-minimal}-20261111/off-traced/pan.trace`, with no
durable backup. Selected exports are in Git. Integration uses the mandated land command, which rebases, runs
`./dev check` under the external GPU lock and pushes normally. Independent lead review remains owed.


## Resumed attended sitting: full-interval OFF witness

Attempt 3 uses Deferred recording (no ring window), PID 14537/window 47455 on the sole built-in display 1,
3456×2234 at 120 Hz, with 1920×1080 engine output. Recording lasts 8.352237 s; collection and finite exports
complete in 81.590 s within the recording/finalization/export bounds. All panel observations remain unlocked,
active and awake. [TOC](attempt-3-off/toc.xml.gz), [display](attempt-3-off/displays.xml.gz),
[collection](attempt-3-off/collection.json), [panel](attempt-3-off/panel-witness.jsonl.gz).

M2.5 has builder proof: all 491 Whole submissions join, 846 total drawable rows join, no unmatched sample affects
Whole, and retained handler rows span 1.255477–8.330502 s around Whole 4.219185–8.319542 s.
Same-process marker offset width is 625 ns; converted presented-value error is 8.5 ns.
[Witness](attempt-3-off/witness.json.gz) and [row counts](attempt-3-off/row-counts.json) retain the complete result.
Raw/derived display tables remain empty and are not the witness. OFF has zero images and no capture spool.
Two Whole two-refresh intervals, 547→548 and 548→549, remain for M2.2 investigation beside the matched ON arms.
Three of six recordings are consumed. ON arms and final M2.2 judgment follow this prerequisite pass.
