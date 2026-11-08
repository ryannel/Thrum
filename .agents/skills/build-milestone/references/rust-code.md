# Rust code shape for Thrum

Read the section for the decision at hand.

## Variants without twins

Mode differences belong in types, so each behaviour has one body and misuse fails to compile.

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

An optional capability, such as strict accounting, is a type parameter whose default costs
nothing. It is not a parallel API:

```rust
pub trait Budget { fn charge(&mut self, bytes: usize) -> Result<(), Capacity>; }
pub struct Unbounded;
impl Budget for Unbounded {
    #[inline]
    fn charge(&mut self, _: usize) -> Result<(), Capacity> { Ok(()) }
}
```

Use a builder once construction has more than a few optional settings. Several constructors that
differ by one argument become one builder.

## Errors

- Use one enum per public boundary. Its variants name the caller's decision: reject input, retry
  later, or treat as a fault. Each variant carries the context needed to act.
- Private helpers return the boundary error, or a private error converted at the edge.
- Report the first error found. Ordering among simultaneous problems is not part of the contract.

## Decoding

- **Authored JSON:** serde derive with `#[serde(deny_unknown_fields)]` into a raw struct, then one
  validation pass that produces the checked type. Check sizes and counts before allocating.
- **Wire data:** decode once into the checked form, or a borrowed view over checked bytes. Never
  re-decode a payload you produced or already validated.
- **Numbers:** use tolerance checks in `f32`/`f64` with named epsilons tied to units. Exact
  arithmetic is for identity and hashing, not geometry or light.

## Visibility and modules

- The crate root re-exports the public model. Modules are private by default, and items are
  `pub(crate)` unless re-exported.
- A file holds one responsibility. Lay out modules as `foo.rs` plus a `foo/` directory.
- Limits and capacities for a subsystem live in one profile struct, not in scattered constants
  that drift apart.

## Tests and fixtures

- Put unit tests in `#[cfg(test)] mod tests;` with the body in a sibling file once they exceed a
  screen or two.
- Never add test-only functions, fields or branches to production items. To inject faults (I/O
  failure, crash points), pass a trait whose production implementation is zero-cost.
- Shared fixtures, such as a rules implementation or a world builder, live in one support module
  or dev-dependency crate. Do not copy them per file.
- Use properties where an invariant is clearer than examples, and fuzz targets for untrusted
  parsers.

## Suppressions

Use `#[expect(lint, reason = "...")]` on the narrowest item. An unfulfilled expectation fails the
check, which is how stale suppressions get found. Do not keep inactive code compiling across
slices behind a `dead_code` expectation. Give it a consumer or delete it; Git keeps the history.

## Failure in runtime code

A panic is a bug, not a control path. Do not build unwind-recovery or poisoning state machines in
runtime code. Return `Result` for recoverable failure. Where a failure is impossible by
construction, use `expect("why it cannot fail")`; `unwrap` is linted in production code.

## Dependencies

- Each external crate needs an exact version and a reuse record in `.config/architecture.toml`.
- Never mirror a dependency's private layout or assert on its internal allocation sizes. Measure
  through its public API, or replace it.
