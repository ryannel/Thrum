# Canonical manifest to SQLite blob bridge

2 November 2026. One small test-only `TreeManifest` writes its canonical bytes using the shared emitter. Their SHA-256 equals `SourceRevision`. The bytes pass through the 4 MiB SQLite blob verifier, become a complete blob, and are named by the root/outcome transaction. After closing and reopening the database, the test reads the stored bytes and checks exact equality with the emitted manifest. Independent review found no issue with this narrow handoff; `RUSTC_WRAPPER= ./dev check` includes the test.

Source fixture SHA-256: `32d84bc20a055aba8b96a632121a887e949e28424e62bdd52194e9f856789591` (`crates/thrum-source/src/sqlite_store.rs`); executed debug test binary SHA-256: `75c721c5cebd920bb26292c33556b4d4321abffe720c7470880908a9743b50da`.

This small fixture buffers the manifest once for simplicity. It does **not** store or verify the source-file bytes referenced by that manifest, authenticate a prior admitted root, reserve a disk/WAL peak, test a process kill, or expose a production accepted-source API. A complete source revision must bind all of those before cook or run may read it.
