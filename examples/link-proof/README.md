# Link proof

This package is proof tooling for [TRANSPORT](../../docs/plans/TRANSPORT/DESIGN.md).
It drives `thrum-transport` through its public API. JavaScript only schedules the Wasm runner and records it;
it implements no message exchange. The native binary never runs under default tests.

The shared script is in `thrum_transport::proof`. Its target runners emit independent content-checked counts
beside adapter queue counts. `scripts/transport_relay.py` conditions real UDP packets in a separate process.
