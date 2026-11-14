# Disocclusion depth candidate

The range-based candidate at `959ada11` passes all 1,966 exact comparisons
([record](range-candidate-exact.json.gz)), the still reference and cut controls,
and the 640×360 timing pair. It has a visible ragged/speckled fringe in both
targets' reveal captures, as [the fresh opinion](../motion/chrome-review-independent.md)
records. Those passes remain evidence for that candidate only.

The next candidate replaces acceptance of any depth inside the four-tap minimum
and maximum with interpolation of reciprocal view depth. On a projected plane,
reciprocal view depth is affine in screen coordinates. Evaluating the plane
`1/z = 0.031*x + 0.019*y + 0.12` at `(0.31,0.73)` gives `z = 6.96961249`.
Linear interpolation of its corner z values gives `7.06172211` (1.32% error),
whereas reciprocal interpolation gives the direct ray result. This avoids the
nearest-depth false rejection without accepting the entire interval between
unrelated foreground and background surfaces.

This is a hypothesis for the observed fringe, not a demonstrated visual repair.
The test-only Metal entry point invokes the production helper on that plane,
a 4/12 depth discontinuity, sky and a sky/geometry boundary. Its old range control
must accept the stale discontinuity while the candidate rejects it. Shader
validation passes; target execution, six still comparisons, both motion campaigns
and timing are pending. The acceptance bound and reference remain unchanged.

AMD's [FSR2 depth-clip stage](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-temporal/#depth-clip)
is a published example of treating disocclusion explicitly before accumulation.
The candidate uses Thrum's existing depth history and an independently derived
plane interpolation; it copies no FSR2 code. The source-fringe diagnosis is an
inference to test, not a finding established by that source.

Cost account: the same four depth texture loads, resources and dispatch remain.
The candidate adds four reciprocal operations and a weighted sum per pixel.
It allocates nothing and changes no public layout or frame contract. Actual GPU
cost must be measured again; the old timing does not qualify this kernel.

Run `run_candidate.py --output tmp/apple-presentation-m3/depth-candidate-1`
under `agent_loop.py lock gpu -- python3`. It stops at the first failing stage,
retains exact commands/logs, and requires one clean source throughout. After a
successful run, inspect the new motion source frames and obtain a fresh opinion
before marking either motion item pass. Final milestone checks and merge are
still required; this runner does not perform them or set any result.
