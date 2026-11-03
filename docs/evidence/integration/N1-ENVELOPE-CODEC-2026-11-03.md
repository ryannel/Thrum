# Exact N1 application envelope codec — 2026-11-03

The registered `thrum-network::envelope` module encodes into caller storage
and decodes a borrowed 64-byte first-version application header. It checks
the `ENG1` magic, major version 1, all thirteen declared message kinds,
zero flags/reserved bytes, exact declared payload length, the 64 KiB complete
message cap and initial Hello's zero identity/epoch/sequence/tick/ACK fields.
The payload is left borrowed; decoded arrays are not allocated.

An independently written 67-byte golden vector checks every header field,
byte order, a three-byte payload and borrow identity. Two further tests
refuse short/trailing/over-limit messages, wrong magic/version/kind,
nonzero reserved fields and malformed Hello identity; exact maximum size
round-trips. Command: `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p
thrum-network --test envelope`. The full `RUSTC_WRAPPER= ./dev check`
gate is required before the checkpoint is committed. Independent read-only
review found no functional defect; it corrected the ACK field's wording to
"dispositioned" because baseline staging and supersession do not imply
StateGroup admission.

This is **structural framing** only. A session owner must validate sender
role, match/epoch, message-sequence and ACK progression, state transitions,
rate limits and exact per-kind payload schema before acting. The fixed R4
StateGroup payload decoder separately enforces its 8 KiB complete-message
cap. Neither module grants admission by itself. No WSS, handshake, baseline,
outcome-progress or whole-callback qualification is claimed.
