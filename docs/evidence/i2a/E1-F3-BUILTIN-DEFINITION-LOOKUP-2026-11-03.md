# Read-only F3 built-in meaning lookup

3 November 2026. `registry_stage::builtin_schema(type, version)` now exposes the exact immutable `BuiltinSchema` object used by F3's scene@2, authored-environment@1 and light-identity@1 staged validation. It returns `None` for unimplemented type/version pairs and does not validate source bytes or grant accepted-source authority. F4 can use its definition hash after separately authenticating a selected pin, without duplicating hash constants or descriptor text.

The existing real staged constant-light fixture now checks pointer identity between its scene/environment/light definitions and the public lookup, and checks absent versions. The focused test and `RUSTC_WRAPPER= ./dev check` passed. This is a small API seam for future semantic cook identity, not a persistent key, product store, build binding or runtime maturity promotion.
