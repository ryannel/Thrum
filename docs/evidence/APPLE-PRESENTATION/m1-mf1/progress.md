# MF1 progress

The game selects an unjittered raster matrix while retaining its submitted 32-phase metadata. Explicit temporal views select the jittered matrix. Stars use the same sampling choice. The original pause and recovery comparisons have been restored, with every consecutive paused frame checked across a complete phase cycle.

The layout tests, native lint, WebAssembly check and documentation check pass. Metal tests and engine captures remain queued behind the shared GPU lock. At observation, lock owner PID 70052 was waiting on child PID 74630 for over 34 minutes. That child polls for its owner’s release file without a timeout. The owner has been notified. GPU proof and the required milestone checker remain live in the queue.

The milestone remains in progress until the Metal tests, reviewer held-camera comparison, full check and integration succeed.
