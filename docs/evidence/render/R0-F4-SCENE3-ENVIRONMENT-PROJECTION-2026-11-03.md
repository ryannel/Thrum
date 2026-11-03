# Scene@3 environment projection for the selected constant-light cooker

3 November 2026. This is a prerequisite for later scene@3 source admission, not an admitted render or F4 product checkpoint.

`parse_environment_scene` keeps the existing scene@2 parser and adds a scene@3 branch through the same opaque checked-record token used by the inactive local decoder. That branch checks the required `render.source` UUID and all local scene@3 grammar, then returns only the scene identity, actual schema and optional environment source needed by the constant-light consumer. The binary source UUIDs become canonical strings only for this selected projection. Scene@2 keeps its existing whole-record check and typed parser. The constant-light provenance now stores the schema reported by the checked scene instead of assuming 2 for every scene.

The new focused test checks scene@3 source/environment IDs, schema 3, escaped-key and escaped-UUID equivalence, and refusal of absent or invalid required render source and unsupported schema. Existing scene@2 and selected-pin cooker tests still pass. `RUSTC_WRAPPER= cargo test --locked --offline -q -p thrum-source -p thrum-cook` passed with 301 source and 41 cook tests; `RUSTC_WRAPPER= ./dev check` passed on rerun. The first full run encountered a transient `Budget(Busy)` in a provisional-store fixture; that isolated test passed immediately and the next full run passed. Independent read-only defect review found no issue.

No scene@3 source can be admitted yet: its descriptor is unregistered and the F3 global closure is incomplete. An authenticated scene@3 accepted-pin provenance test belongs at that activation gate. No target render product, visual result, CPU/allocation comparison or combined resource measurement is claimed.
