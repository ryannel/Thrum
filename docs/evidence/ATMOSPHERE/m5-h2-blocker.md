# M5 H2 numerical admission blocker

11 November 2026. Base `a471fe5d83beb926bdd1cf49e123696090d4e262`. Target Apple M3 Max,
Metal, production RGBA16Float sky chart and linear sampler. M5 remains open. Partial implementation
and numerical referees are in this worktree. Integration is pending.

## Failure

The [H2 filtered-decode rule](../../plans/ATMOSPHERE/DESIGN.md#numerical-acceptance) fails on the
live chart at Sun 45°, view 1° above the anti-Sun horizon. The independent f64 reference reads back
the stored texels and evaluates their bilinear blend. Green returns `0.086669921875` against
`0.0866538115913`, with allowance `0.0000123132324219`. Blue also exceeds its allowance.
The [live and aerial log](m5-h2/handoff.log) records the failure and the passing H4 filtered arms.

A signed-π wrap change was tried and removed after the result persisted. A separate controlled
measurement isolates the interpolation precision. Across 257 positions in one texel interval,
encoded endpoints 0 and 1 return 257 distinct values and meet the H2 bound at all positions.
Endpoints 64 and 64.0625 return two distinct values and exceed the bound at 235 positions.
The largest GPU/f64 addressing difference is 0.000006814 texels.

At the midpoint of the low-contrast field, decoded radiance is `0.06256103515625`.
The f64 result is `0.06253051756008`, with allowance `0.000002532958984375`.
Using the observed GPU address in the f64 blend gives `0.06253051754902`.
This demonstrates output precision consistent with fp16 filtering arithmetic. The high-contrast
control resolves 1/256 weight increments. Addressing and the assumed 1/64 weight error cannot
account for the measured low-contrast error. [Precision log](m5-h2/handoff.log)

## Reproduce

Run under the shared GPU lock, using the build skill's `agent_loop.py lock gpu --worktree . --`.
The precision measurement is

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.7.0/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- \
  sh -c 'RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-render --lib metal_h2_sampler_precision_report -- --nocapture'
```

The live failing predicate uses the filter `metal_h2_filtered_decode` in the same command.
[Source hashes](m5-h2/sources.json) identify the worktree code. [Base record](m5-h2/base.json)
records the shader hashes and sampled-texture counts read from source taken before the light-group edit.

## Progress and required decision

`./dev check` passes native and Wasm Clippy, then fails with 221 renderer unit tests passing and
H2 failing. [Full check log](m5-h2/check.log.gz). `./dev check-docs` passes.

CPU identity and cloud-field tests pass. Seven Metal cloud-factor cases cover 56 point queries,
including height-dependent projection, advection and exact clear support.
[Cloud-factor log](m5-h2/cloud-factor.log). Receiver-kind and cooked-volume qualification remains pending.
The Metal contribution compiles all five operations and
observes profile and actual device replacement. H1's 20 low-altitude cases pass an independent f64
quadrature. H4's live and known-field filtered-decode arms, exposure control and three wrong-lookup
controls pass on Metal. These are partial handoff proofs. Remaining work includes H1's full receiver
matrix, the other H2–H5 obligations, Chrome, receiver/volume qualification, parity, motion and cost.

The design directs the builder to return a correct lookup's bound miss to planning. M5.2 needs a
planner decision covering sampler result precision and the approved lookup mechanism/cost, followed
by implementation and target qualification. The done-when item retains status `open` and links here.
