# Rust code

How Thrum shapes public types, variants, decoders and suppressions. The error shape and test layout are in
`patterns.md`, and the lint policy is in the workspace `Cargo.toml` and `clippy.toml`. `./dev check` denies every
warning.

## Variants without twins

When behaviour differs by mode, put the difference in a type, so each behaviour has one body and misuse fails to
compile:

```rust
pub struct Host<M: Mode> { world: World, clock: Clock, mode: M }
pub struct Local;
pub struct Match { peers: PeerTable }

impl<M: Mode> Host<M> {
    pub fn step(&mut self, now: Instant) -> Result<TickReport, StepError> { /* one body */ }
}
impl Host<Match> {
    pub fn admit_peer(&mut self, hello: Hello) -> Result<PeerId, AdmitError> { /* ... */ }
}
```

An optional capability, such as strict accounting, is a type parameter whose default costs nothing. It is not a
parallel API:

```rust
pub trait Budget { fn charge(&mut self, bytes: usize) -> Result<(), Capacity>; }
pub struct Unbounded;
impl Budget for Unbounded {
    #[inline]
    fn charge(&mut self, _: usize) -> Result<(), Capacity> { Ok(()) }
}
```

Several constructors that differ by one argument (`new`, `new_with_capabilities`, `new_with_setup`) become one
`*Config` struct or a builder.

## Public surface

- Items are private or `pub(crate)` unless something outside the crate uses them. The `unreachable_pub` lint
  catches a `pub` item that nothing exports.
- Crates expose their API in one of two ways today: `thrum-render` and `thrum-network` expose `pub mod` trees, and
  the asset, cook and model crates re-export a flat model at the crate root. Follow the crate you are in.
- Test-only helpers are not exported from a production library, `#[doc(hidden)]` included. Put them behind a
  `test-support` feature, or move the test into the crate.
- Limits and capacities for a subsystem live in one profile or config struct, not in scattered constants that drift
  apart.

## Decoding

- **Authored JSON:** derive serde with `#[serde(deny_unknown_fields)]` into a raw struct, then run one validation
  pass that produces the checked type. Check sizes and counts before allocating.
- **Wire data and cooked products:** decode once into the checked form, or into a borrowed view over checked bytes.
  Don't re-parse a container on every accessor, and don't re-decode bytes you produced or already validated.
- **Numbers:** compare `f32`/`f64` with named epsilons tied to units. Exact arithmetic is for identity and hashing,
  not geometry or light. The CPU model crates work in `f64` and narrow to `f32` at the GPU boundary under a reasoned
  `#[expect]`.

## Failure in runtime code

A panic is a bug, not a control path, so don't build unwind-recovery or poisoning state machines in runtime code.
Return `Result` for recoverable failure. Where a failure is impossible by construction, use `expect("why it cannot
fail")`; `unwrap` is linted in production code.

Don't discard a `Result` with `.is_ok()` or `let _ =`. The surface encode in `runtime.rs` turns every encode error
into "dropped frame" this way, so a validation fault looks like a missed frame (M41).

## Suppressions

Use `#[expect(lint, reason = "...")]` on the narrowest item. An unfulfilled expectation fails the check, which is how
stale suppressions are found. `#[allow]` is rejected; three benches bypass that with `#![allow(unsafe_code)]`, which
is not a pattern to copy.

Don't keep inactive code compiling behind a `dead_code` expectation. Give it a consumer or delete it; Git keeps the
history.

## Dependencies

Never mirror a dependency's private layout or assert on its internal allocation sizes. Measure through its public
API, or replace it.
