# Failed observed-policy work-removal checkpoints

Both candidates are **FAILED against the predeclared maximum 4 ms whole synchronous CPU step plus publication gate**. Neither is selected runtime source. Wet-case passes do not override a dry-case failure. This package retains functional progress and adverse cost evidence; it supplies no GPU, display-cadence, art, full-sky, hardware-portability or physical-memory acceptance.

| Frozen candidate | Recorded observed maxima (ms) | Gate |
| --- | --- | --- |
| 902f2608, original b003d308 protocol | single wet 2.025709; single dry **4.462250** at 420 s; two wet 3.220583 | FAIL |
| 12777301, original 65fff59a protocol | single wet 1.832583; single dry **4.203417** at 420 s; two wet 3.029750 | FAIL |
| 12777301, supplementary 27ccaf73 two-dry protocol | two dry **7.425500** at 420 s (legacy 3.378667) | FAIL |

These are retained single-protocol CPU observations, including all samples and GC events, not rerun timings or a GPU claim. The complete evaluators, immutable configurations, raw F64 arrays, timing guards and reports are inside the two archives. [Raw integrity checks](verification/raw-cost-integrity.json) confirm all fourteen arrays have exactly their recorded counts and maxima; no cost evaluator was executed during packaging. Existing guards and their failures remain historical, with no threshold change, tail deletion or later candidate substitution.

## What is retained

- [work-902f2608a776560b.zip](work-902f2608a776560b.zip): all 143 non-native files from the frozen work-removal candidate (1,437,806 decoded bytes). This includes source and exact implementation diff, baseline/reference imports, admission provenance, tests, the per-opportunity shared healthy-response profile and lazy prepared-endpoint derivations, review records, pre-lazy attempt, functional receipts and b003 cost/GC data.
- [front-12777301a56a54c3.zip](front-12777301a56a54c3.zip): all 160 non-native files from the frozen front-midpoint candidate (1,630,019 decoded bytes). It adds the X-only midpoint proposal and differential tests, the owned observation API, original and two-dry cost protocols, and the pre-input-Q-guard attempt and corrections.
- [restore-spec.json](restore-spec.json) pins every archive member, full source marker, cost receipt and seven external native payloads. No R16 field, capture or later weather-zero candidate is duplicated here. Shared inputs are recovered by hash from retained sibling evidence, including the [original failed f57 CPU package](../cloud-observed-policy-cpu-2026-11-03/README.md); no cook or download is required.

902f source marker: `902f2608a776560b1b88a998a0555b1605db2ea2937f443cebdceb04ca00436d`. 12777301 source marker: `12777301a56a54c3bcc0cf29b23b7a33eb3e3cfad186dc908d04b0b22f4715cf`. Archives preserve historical statements such as “cost unrun” at their original pre-timing functional/review freeze; the later archived COST-HANDOFF/COST-RESULTS and the table above are the final adverse cost disposition.

902f removes repeated pure healthy-response/midpoint and endpoint preparation work while retaining the original f57 states, counters, traversal order, failures and publications. 12777301 narrows the held-front half-step query to the exact X terms and adds on-demand owned capture. Its independent review caught a replaced-Q fallback edge, corrected before the final freeze, and a near-EPS capture-publication issue: **any presentation time strictly before commit receives the complete prior-barrier summary**. The pre-fix attempt, final review and regression tests remain in the front archive. Review closure covered code correctness, while both cost gates were still unrun; it did not approve runtime, art or cost.

The capture API is functional source plumbing only: an explicit caller clock epoch, presentation-causal summary, eligible presented detail, separately labelled committed audit and old-epoch committed pending interruption. It does not invent epoch zero, inject epoch into stepping, or publish future full detail as displayed. It remains unadopted along with this source candidate.

## Restore and functional checks

Keep this directory beside its retained sibling evidence dependencies. From the repository root, choose fresh absent outputs outside `docs/evidence`:

```sh
python3 docs/evidence/cloud-observed-policy-work-2026-11-03/restore.py --variant work --output /private/tmp/cloud-work-restored --test
python3 docs/evidence/cloud-observed-policy-work-2026-11-03/restore.py --variant front --output /private/tmp/cloud-front-restored --test
```

Run sequentially. `--test` runs only the explicit original functional command lists under each archived process-tree guard (224 MiB early, 256 MiB hard); it rejects timing evaluator commands. Shared native inputs occupy 17,956,864 unique decoded bytes per restored tree, with equal payloads hardlinked across the natural/legacy paths. All baseline imports and the nonserved `src/source/admission-policy.provenance.json` are retained. Cost JSON and source pins are verified unchanged after tests. Omitting `--test` only restores and verifies bytes.

[Fresh restore verification](restore-verification.json): work eight phases and front ten phases all passed, including source8 in each, native field/legacy checks and the f57 differentials; front also checks the capture boundary and coupled-dry functional path. Work relocation was repeated once because first completion was initially absent from the tool response; both completed successfully and both receipts are retained. Highest sampled process-tree RSS across those fresh functional runs was 192,380,928 B, below the unchanged early guard. This is a sampled functional-test resource observation, not runtime performance or an OS physical-memory bound. No timing evaluator, GPU, browser, source modification or canonical change occurred during retention.
