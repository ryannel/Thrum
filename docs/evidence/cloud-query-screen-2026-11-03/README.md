# GPU query work-removal screen: strict equivalence rejected

3 November2026. Baseline remains the [c3e0 integrated lab](../cloud-lab-hardening-2026-11-03/README.md). **Neither GPU candidate is adopted; no GPU speedup is claimed.** This screen stays off the visual-content critical path.

Both candidates skip exact zero native density. An explicitly published same-tuple allZero flag can avoid16 local exposure loads; only exact zero loss skips material noise, while fresh distance/taper remains. Full nonzero optical support, alpha, held slots and all consumers are retained. No epsilon or resolution change is introduced. The baseline and candidate are separate GPU pipelines within one untimed diagnostic; the original experimental CPU .002q envelope is an additional, distinct check.

- **e6456b622215cdd7:** [source100s](system-probe-t100000-e6456b622215cdd7-64543355-1.json) passes all held variants bitwise and the CPU oracle. [Real drying550.125s](system-probe-t550125-e6456b622215cdd7-64543355-3.json) fails the predeclared strict bit gate: actual/alpha0/.5/1 have12/18/12/24 component differences, max finite absolute delta1.49e−8. Raw rho is unchanged. Synthetic prior/current1m exposure cases pass.
- **c2f6348636a059e0:** the nonzero-exposure branch restores baseline material→exposure→loss→distance expressions verbatim, keeping the narrower zero-tuple optimization. [The same drying case](system-probe-t550125-c2f6348636a059e0-72825924-1.json) still fails:20/27/20/34/114/114 component differences across actual/alpha0/.5/1/prior-only/current-only, max7.45e−9. All cases remain within the existing CPU envelope and have no invalid samples. This does not turn the failed strict gate into a pass.

The differences are consistent with device compiler contraction/reassociation under changed control flow; no compiler intermediate or exact instruction diagnosis was obtained. A mathematically equivalent scalar expression need not produce identical compiled GPU bits. Tiny observed deltas are not a continuous-field or portable error bound. Keep the original renderer rather than relax a threshold after seeing results. A later deliberate precision contract may revisit this separately, including strict-zero/support and reconstructed path effects.

Read-only review found no actionable code defect in e645 and independently ran13 Node tests plus14 mocked probe success/failure restoration cases; [dated review](../cloud-source-cost-2026-11-03/GPU-QUERY-REVIEW.md) keeps that identity and scope. The author ran13 Node tests and9 Naga modules for each candidate. Actual root browser GPU results above exposed what those checks did not prove. The isolated benchmark uses one coefficient call per point,10 alternating ABBA/BAAB blocks plus A/A, raw timestamp validity and readbacks; **root did not run it after the correctness gate failed**. No whole-graph, device, full-sky or performance conclusion follows.

## Restore a rejected instrument

```sh
python3 docs/evidence/cloud-query-screen-2026-11-03/restore.py --build c2f6348636a059e0 --output tmp/cloud-query-review
python3 tmp/cloud-query-review/serve.py --port 8159
```

Open `http://localhost:8159/`, seek550.125, Replay, then Compare baseline/fast q + CPU. `--build e6456b622215cdd7` restores the first candidate. Each archive contains every served file except four native R16 dependencies shared losslessly with the component checkpoint by decoded SHA. Material noise is included. Exact identities, failure receipts and original freeze records survive; mutable code is not inferred from a newer working copy. Restore refuses existing output and verifies every served hash. This records a failed optimization gate, not a new source or visual system.
