# First production F3 constant-light numeric projection

3 November 2026. The existing F3 parsers for `core.scene@2`,
`thrum.environment.authored@1` and `thrum.light.identity@1` now expose
narrow read-only typed projections from exact JSON bytes in
[`environment_json.rs`](../../../crates/thrum-source/src/environment_json.rs).
The same strict parser and unchanged built-in descriptors still validate the
staged registry; this slice does not change source meaning, hashes or
admission. It makes the validated raw numeric values available to the first
F4 constant-light cooker instead of discarding them outside tests.

`parse_authored_environment` returns the stable environment ID, checked
Sun light reference, authored receiver-to-source direction, linear
unit-luminance colour, raw illuminance, angular radius, ambient irradiance
and background radiance. `parse_environment_scene` returns the schema-2
scene ID and optional active environment edge. `parse_environment_light_identity`
returns the checked light ID with its required incoming-environment-sun
driver. Each public parse runs the existing bounded no-tree header pass and
the same strict typed record parser. It checks labels, documentation and
opaque extensions but omits them from the public product projection.
Numbers retain exact decoded f64 bits, including signed zero; F4 owns its
separate positive-zero normalization.

Three new focused tests exercise exact raw values, escaped/plain semantic
equivalence, omitted cosmetic/opaque meaning, wrong type/version/driver,
invalid numbers, decoded duplicate keys, malformed extensions and scene/light
edges. The 14 environment-parser tests passed. The complete `thrum-source`
test run passed 213 tests with 52 ignored, including existing F3 registry and
definition-hash tests. A defect-first review found no actionable code issue.
`RUSTC_WRAPPER= ./dev check` passed repository checks, 118 Python tests
(five skipped), rustfmt, workspace Clippy and Rust/doc tests.

The public functions accept byte slices; a caller must first establish that
the entire bytes came from an authenticated accepted `PinnedSource`, check
selected scene/environment/light identity and typed closure against its
manifest, and bind the fixed built-in meaning. Parsing a detached byte slice
alone is not an admitted product. The selected pin-to-codec adapter, semantic
`CookKey`, product manifest and exact-source build binding remain open. This
small projection performs no per-tick work and adds only a few f64 fields to
one parsed record; no assembled edit-to-ready latency or peak memory was
measured here.
