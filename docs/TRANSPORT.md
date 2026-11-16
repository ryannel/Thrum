# Message links: a game maker's guide

`thrum-transport` moves whole messages between a host and its players over QUIC: native programs on the Mac and
the iPhone simulator, and pages in Chrome through WebTransport. It gives no meaning to what it carries; the match,
its sequences and what a late message means are `SESSION`'s. The design is
[TRANSPORT's](plans/TRANSPORT/DESIGN.md), and [the networking design](design/modules/NETWORKING.md#transport-and-overload)
says where it sits in the engine. A game that does not name `thrum-transport` contains none of it, and a game that
names it without a feature gets the interface and the in-memory link only, with no outside crate.

| Feature | What it adds | Where |
|---|---|---|
| none | `Link`, `Profile`, `Ticket`, counters, the in-memory link (`memory`), the link script (`proof`), the framing decoder (`wire`) | every target |
| `native` | `native::Host` and `native::connect`, on Quinn with one I/O thread | Mac, iPhone |
| `browser` | `browser::connect`, on the page's own `WebTransport` | `wasm32` in Chrome |

## A link

A **link** is one connection between a client and a host. It has two lanes:

| | Reliable lane | Datagram lane |
|---|---|---|
| Size | 1 to 65,536 bytes (`RELIABLE_LIMIT`) | 1 to 1,000 bytes (`DATAGRAM_LIMIT`) |
| Delivery | every message, once, in the order sent, or the link closes | at most once; some lost; any order |
| A send it can't take | refused at the call: `Empty`, `TooLarge`, `NotOpen`, or `Full` when the lane's queue is at its cap | refused at the call when `Empty`, `TooLarge` or `NotOpen`; otherwise queued, and discarded and counted if it waits past the age limit or is the oldest when the queue is full |

Both limits are the same on every adapter, so a packet is sized once. Nothing is promised about order between the
lanes. Every call returns at once: `send`, `receive`, `event`, `counters`, `close` and `is_open` never block or
wait on the network, so call them from the tick, normally at its pre-tick barrier.

Three things a game must not assume:

- **A datagram may arrive long after it was sent.** The age limit (250 ms by default) bounds only its wait in the
  link's own queue. Once handed to QUIC or the browser it has no deadline, and one held by a congested or cut path
  can arrive seconds later. Judge a datagram's age by its content, as `SESSION` does with its target tick and
  envelope sequence.
- **A close can cut off the last reliable messages.** When a link closes, for any reason including either end's
  `close`, what arrived is an unbroken run from the first message sent, and nothing after it is ever delivered, on
  that link or a later one. If the last message must arrive, wait for the peer's answer before closing.
- **A link may open before either game speaks.** Each end reports `Event::Open` once the link's own startup is
  done, and either may send first.

## Start a host and hand out a ticket

```rust
use thrum_transport::{Profile, native};
let host = native::Host::start("0.0.0.0:7400".parse()?, Profile::default())?;
// Poll until the host has made its key and certificate: a few milliseconds.
let ticket = host.ticket(); // Option<Ticket>; its text is thrum1@<address>@<64 hex digits>
while let Some(link) = host.accept() { /* a new player's link, already open */ }
```

The **join ticket** is the host's address and the SHA-256 of a certificate the host makes when it starts, held in
memory only. Its text is about a hundred characters, to put in a link or paste, not to type. **A ticket dies with its
host:** a restarted host has a new certificate and a new ticket, and a host must restart within 13 days, the
certificate's life. `Ticket`'s text parser refuses anything but the exact form the host prints.

`host.counters()` says what happened to connections that never became links the game could take: refused at the
cap, refused for another link version, handshakes that failed, connections that broke the link's rules before
opening, and connections dropped at the startup timeout. The host holds up to `Profile::links` links and
handshakes at once (four by default) and refuses the rest.

## Open a link

```rust
let link = native::connect(&ticket_text.parse()?, Profile::default())?;    // Mac and iPhone
let link = browser::connect(&ticket_text.parse()?, Profile::default())?;   // in a Chrome page
```

Every adapter pins the ticket's hash: a client connects only to the host whose certificate has exactly that hash.
There is no option that skips it. In Chrome the page must be a secure context, which `http://127.0.0.1` is.
`connect` returns at once; the link reports `Open` or a close through its events.

## Events and close reasons

`link.event()` returns `Event::Open` once, then `Event::Closed(reason)` once; closed is final. A reconnect is a new
link with a new `id()`, and nothing sent on the old link arrives on the new one. The reasons:

| Reason | When |
|---|---|
| `Local(code)` | this game called `close(code)`, or its host was dropped (`Local(0)`) |
| `Peer(code)` | the other end closed with that code; a Chrome page that reloads or closes reports `Peer(0)` |
| `TimedOut` | nothing heard for the idle timeout (4 s by default), startup not finished by the startup timeout, or this process was frozen for longer than the idle timeout |
| `Refused` | a wrong ticket, another link version or a full host |
| `Protocol` | the peer broke the link's rules: a first frame that is not the link hello, a length prefix of zero or over the limit, a second stream, or a datagram over the limit. None of the offending bytes are delivered |
| `Transport` | the connection failed for another reason |

A frozen Chrome page is not a dead link: Chrome's network process keeps answering while the page's script is
stopped, so a page frozen for longer than the idle timeout keeps its link. A player who stops sending is
`SESSION`'s to time out.

## Counters

`link.counters(now)` returns, for each lane, messages and bytes sent and received, the bytes queued each way and the
age of the oldest outbound message; sends refused by reason; datagrams handed on, expired, dropped at the cap and
dropped inbound; and, on Quinn ends, the bytes of datagrams held by QUIC, the round-trip estimate, packets sent and
lost and bytes on the wire. What an adapter can't supply is `None`, never zero: Chrome supplies no QUIC figures, so
the host's side of the same link gives the round trip.

## Bounds

Every queue has a cap set by the `Profile`: reliable and datagram bytes each way (1 MiB and 64,000 bytes by
default), the age limit, the bytes of datagrams QUIC may hold (4,096), the startup and idle timeouts and the link
cap. A link's memory is the sum of its caps, and a host's is that times its link cap. A player who stops reading
fills only the lane that feeds them: the host's sends to them return `Full`, and the other players see no
difference. What a full lane means is the game's to decide; C13 has it resynchronise or disconnect that player.

## Rehearse with no socket

The in-memory link has the whole contract with a conditioner applied to each message, in virtual time you supply,
so the same seed and the same calls give the same events. `Conditioner::default()` is A0's profile: 50 ms ± 10 ms
each way and 2% loss. `Memory::cut` cuts the path, `Memory::stall` holds the QUIC stand-in so datagrams arrive
late, and `duplicate` duplicates datagrams as a fault beyond the contract.

```sh
cargo run -p thrum-transport --example rehearse
cargo run -p thrum-transport --features native --example host_and_client
```

The first rehearses a link in memory; the second runs a host and a client over this machine's loopback.

## The proof command

`scripts/transport_link.py` runs the module's proofs through the engine's own adapters, with a separate relay
process that adds A0's delay, jitter and loss to real UDP packets. Each run writes a record under `tmp/transport/`
and prints its path; a failed expectation exits with an error. Simulator runs take the `simulator` lock and Chrome
runs the `chrome` lock (wrap them in the orchestrate script's `lock`).

```sh
python3 scripts/transport_link.py conformance --target mac-metal      # the link script, direct and through the relay
python3 scripts/transport_link.py conformance --target chrome-webgpu
python3 scripts/transport_link.py conformance --target ios-sim
python3 scripts/transport_link.py suspend --target mac-metal          # a frozen client; Chrome: reload, tab close, frozen renderer
python3 scripts/transport_link.py hostile                             # hostile peers beside three healthy links
python3 scripts/transport_link.py overload                            # a player who stops reading
python3 scripts/transport_link.py session                             # four clients on one host for ten minutes
python3 scripts/transport_link.py costs --target mac-metal            # baselines, never thresholds
python3 scripts/transport_link.py closure                             # who links which crates
```

The stdin fuzz targets for the link's own parsers are `fuzz_stream`, `fuzz_hello` and `fuzz_ticket` in
`crates/thrum-transport/examples/`; `cargo test -p thrum-transport --test untrusted` runs their property tests.

## What has not been run

Every proof is on one Mac over loopback: no second machine, Wi-Fi, home router or internet path. Chrome is the only
browser run; Safari and Firefox are designed for, not tried. The simulator proves function, never speed, and a
physical iPhone, where the local-network permission applies, has not been run. There is no public host, no
certificate from a public authority and no WebSocket fallback.
