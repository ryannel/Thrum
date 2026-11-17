# M2 Chrome queue repair

The clean-main M2.6 check timed out while repeatedly waiting for Chrome. The
conformance command previously acquired Chrome separately for the profile vectors
and each of eight trace routes. Each release let another queued campaign run
before conformance could continue.

At `69395002b`, compilation remains outside the resource lock. The nine requests
run sequentially in one real Chrome session under one `chrome` lock. Crowd and
Mixed still run at all three retained sizes, with each smallest-size wire route.
Python still compares the full profile vector table, timeline folds, checkpoints
and required execution witnesses against the unchanged retained references.

Conformance does not read the per-tick timeline array: the runner already computes
its fold and checkpoints. The browser omits that unused array before returning the
batch, bounding the transport reply. The discriminating control still receives its
timeline. Debugger errors and connection closure now reject outstanding requests
instead of silently retaining the lock while waiting for a response.

An initial batch attempt at `7ee85d580` consumed about nineteen minutes of renderer
CPU, then its debugger connection disappeared while the harness waited. No receipt
from that attempt is accepted. Reply reduction and explicit disconnect handling
were added before the final run; payload size as the cause of the disconnect is
an inference, not an isolated measurement.

Eleven focused Python tests pass, including all retained batch routes and refusal
of missing browser results. `node --check scripts/sim2d_browser.mjs` passes.
The native and Wasm artifacts remain byte-identical to the prior M2 cost artifacts:
`dc0466a77b51273c95cbfcc25bc233ab3eb728de320a4376ee6424b785216dee` and
`0a84f5f2ed10fe65fe2c4043c9c49a427f2c6a768657587e8ea728300d2678bd`.
Fresh cost measurements are required because the source key also binds the harness.

The final command passed on real Chrome:
`python3 scripts/sim2d_conformance.py run --target chrome-webgpu` (with
`--output tmp/sim2d/M2-chrome-batch.json` to retain the receipt).
The [retained receipt](chrome-webgpu.json) records all eight routes and the full
vector comparison at `69395002b`. The uninterrupted browser batch completed in
about twenty minutes, below the thirty-minute resource hold limit. No trace was
shortened and no digest, checkpoint or witness comparison was removed.
