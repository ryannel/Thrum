# TRANSPORT: two processes exchange messages over real internet protocols, one of them a browser

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C11, C12 and C13 in [CONTRACTS](../../design/CONTRACTS.md),
[the networking design](../../design/modules/NETWORKING.md#transport-and-overload),
[the A0 decision](../../design/decisions/A0-N1-TRANSPORT-CODEC.md) and
[the input wire](../../design/decisions/A5-INPUT-WIRE.md) govern.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module TRANSPORT. It
needs: SHOWCASE", which is row 17 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../BRIEF.md) carries
as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

That page also sets defaults, which are an agent's and not the owner's: cooperative online play for 2 to 4 players,
on the Mac, Chrome and the iOS simulator. The module list turned the page into six modules and settled that
[a native process hosts and browsers join](../ARCHITECTURE.md#mechanisms-settled-here). This module is the part that
moves bytes between those processes. The module list's brief for it, which
[its entry](../ARCHITECTURE.md#transport) now replaces with a pointer here, left three choices to this planner: the
WebTransport crate, where the network's thread sits beside the world lane, and whether A0's WebSocket fallback is
built.

This design reconciles the code on main; it is not a blind study. Read for it: `thrum-network` (the envelope and the
input, state and acknowledgement codecs, none of which opens a socket), the tick-quantised `LinkConditioner` in
`crates/thrum-core/tests/support/v1_workload.rs`, the contracts and decisions above, the sources of the candidate
crates, and [a planning probe](../../evidence/TRANSPORT/planning-probe.md) run for it on all three targets.

## Purpose and scope

A game opens a **link** to a host and exchanges whole messages on it: some that must all arrive in order, some that
may be lost but must not be late. The link behaves the same whether the other end is a Mac program, a program in
the iPhone simulator or a page in Chrome, and a game can rehearse it with no socket at all.

**Consumers.** `SESSION` builds the match on the link and tests it first on the in-memory link. `SWARM` starts a
host and hands players a join ticket. `GAME-UI` shows the link's state and takes the ticket.

**What exists.** Byte codecs with no I/O. A conditioner that counts ticks inside one test crate. No socket, no TLS,
no thread for the network, and no shipping crate that depends on an async runtime.

**Commissioned.** The nine mechanisms below, proved by one host holding Mac, Chrome and simulator clients on this
Mac through A0's conditioner.

**Not built.** What a message means: the envelope, epochs, sequence numbers, acknowledgement, clock sync and
admission stay `SESSION`'s. A WebSocket fallback. A certificate from a public authority, a public host, a relay,
NAT traversal, matchmaking and accounts. A browser that hosts. More than one reliable stream. A link between two
machines: every proof here is on this Mac.

## What a consumer must be able to do

| Need | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **N1 Send messages that all arrive, once, in order** | 1 to 65,536 bytes, the envelope's cap; a burst of 1 MiB queued at once; both directions; through the conditioner | Passing each read of the stream on as a message. On loopback reads match writes; a message that spans packets under loss does not. Rejected by content checks on multi-packet messages through the conditioner |
| **N2 Send messages that may be lost but never wait behind a lost one** | 1 to 1,000 bytes; 60 a second each way on each of 2 to 4 links; while a large reliable transfer runs | Carrying them on the stream. Delivery is then perfect and ordered, and one lost packet delays everything after it. Rejected because under the conditioner this lane must lose some and reorder some |
| **N3 Get the same behaviour from every adapter** | In-memory, Mac, iPhone simulator, Chrome | A Chrome row driven by hand-written JavaScript, or a simulator row run by the Mac binary. Rejected by [one script](#8-one-link-script-four-adapters) run through the engine's own adapter on each, with the peer's platform in the record |
| **N4 Know that a link opened, that it closed, and why** | A clean close from either end; a peer that is killed; a path cut for longer than the idle timeout; a native client frozen and resumed; a wrong ticket; a full host | Learning of a dead peer only when the next send fails. Rejected by a cut on a link that is sending nothing |
| **N5 Be sure who the host is** | Every adapter; a host on a local machine with no public certificate | Switching certificate checks off to make Chrome connect. Rejected by a wrong ticket that must be refused on every target, and a Chrome launch with no flag that touches certificates |
| **N6 Keep running when one peer is slow or hostile** | A peer that stops reading, floods either lane, breaks the framing, or never finishes a handshake; three healthy links beside it | A host that is fine with well-behaved peers. Rejected by M2's hostile and stalled peers beside healthy links |
| **N7 Rehearse a match with no socket** | A0's delay, jitter and loss; reordering; a cut; a slow peer; the same seed twice | A lossless in-process queue. Rejected by running the link script, loss and all, on the in-memory link |
| **N8 See what the link is doing** | Bytes and messages each way on each lane, what was refused or discarded, how old the oldest waiting message is, and round trip and loss where the adapter knows them | Counters only on the Mac. The record marks what an adapter cannot supply as absent, never as zero |
| **N9 Pay nothing when the game is not networked** | The V1 game, the starter and the creator scenes; and any package that uses the link's interface without sockets | A feature flag inside an existing crate. Rejected by a check of each package's dependency closure |

## Mechanisms

### 1. The link

A **link** is one connection between a client and a host, as the game sees it. It has two lanes and gives no
meaning to what they carry.

| | Reliable lane | Datagram lane |
|---|---|---|
| Unit | A whole message, 1 to 65,536 bytes (`thrum-network`'s `MAX_MESSAGE_BYTES`) | A whole message, 1 to 1,000 bytes (A5's datagram budget) |
| Delivery | Every message, once, in the order sent, or the link closes | At most once; some are lost; any order |
| Content | The bytes sent, whole | The bytes sent, whole |
| A send it cannot take | Refused at the call: too large, empty, or `Full` when the lane's outbound queue is at its cap. Nothing is dropped silently | Refused at the call when too large or empty. A datagram that has waited longer than the profile's age limit is discarded, oldest first, and counted |

Nothing is promised about order between the two lanes.

**Both limits are constants of the link**, the same on every adapter, so `SESSION` sizes a packet once. The
datagram limit is A5's budget and not the adapter's ceiling: the probe measured ceilings of 1,162 bytes for raw
QUIC at its 1,200-byte starting MTU and 1,211 in Chrome, and QUIC guarantees that starting MTU on any path it works
on ([RFC 9000, section 14](https://www.rfc-editor.org/rfc/rfc9000#section-14)). An adapter whose ceiling is below
1,000 at open refuses the link.

**Events.** A link is connecting, then open, then closed, and closed is final. A close carries one reason: closed
here; closed by the peer, with the peer's code; timed out; refused (wrong ticket, wrong version or a full host);
the peer broke the link's own rules; or a transport error. A link has an identifier that is never reused in the
process. A reconnect is a new link with a new identifier, and nothing sent on the old one is delivered on the new
one, which is what lets `SESSION` give it a fresh connection epoch (C13).

**The interface is calls that return at once.** Send on a lane, take the messages that have arrived, take events,
read counters, close with a code. A host adds: start with an identity and a bind address, take newly opened links,
hand out its ticket. No call blocks, awaits or names a type from the libraries beneath. The **link profile**, set
when a host or a client starts, holds the timeouts, queue caps and age limit; they are settings a game may change,
as C13 says adapter settings are, and their starting values are the builder's to choose and record.

### 2. The wire

**QUIC under every adapter**, as A0 selected. The reliable lane is one bidirectional stream that the client opens,
carrying length-prefixed messages. The datagram lane is QUIC's unreliable datagram extension
([RFC 9221](https://www.rfc-editor.org/rfc/rfc9221)). The host is one QUIC endpoint on one UDP port that accepts two
kinds of client:

- **native clients** (the Mac and the simulator) speak raw QUIC under Thrum's own ALPN, with no HTTP;
- **browsers** speak WebTransport over HTTP/3 under `h3`, the only way a page reaches a UDP socket.

The ALPN, and for a browser the request path, name the link's version. A client with another version is refused
before any message.

**Libraries.** Audited on 13 November 2026 for licence, datagrams, a browser client and upkeep:

| Crate | Licence | Finding | Decision |
|---|---|---|---|
| `quinn` 0.11.12 | MIT OR Apache-2.0 | Datagrams, pluggable congestion control, released September 2026 | Used, as A0 selected |
| `web-transport-quinn` 0.13.6 | MIT OR Apache-2.0 | The WebTransport handshake on a Quinn connection the caller owns; also wraps a raw QUIC connection in the same session type; released 3 November 2026; the transport of Media over QUIC's Rust stack | Used on the host for browsers |
| `web-transport-wasm` 0.8.8 | MIT OR Apache-2.0 | Needs `--cfg=web_sys_unstable_apis` on the final build, which would change the flags of every Wasm build in the workspace | Not used |
| `wtransport` 0.7.2 | MIT OR Apache-2.0 | Native only; hides the Quinn connection, so raw QUIC needs a second stack; its README calls it "not considered completely production-ready" | Not used |
| `webtrans` 0.6.0 | MIT | About a hundred downloads in ninety days | Not used |
| `h3-webtransport` 0.1.2 | MIT | No release since May 2025 | Not used |

**The browser adapter binds the page's own `WebTransport` directly**, with hand-written `wasm_bindgen` imports, as
`thrum-asset` already binds `fetch`. It needs one stream, datagrams, the close and the certificate hashes, so no
crate is added for it. It asks for a dedicated connection that must support datagrams (`requireUnreliable`), so the
browser never falls back to HTTP/2, which has none. If the hand-written binding proves costly, `xwt-web` (MIT, its
own bindings, no build flag) is the replacement.

**Why not WebTransport for native clients too.** It would give one path on the host. It also puts an HTTP/3
handshake, about three round trips where raw QUIC takes one in the probe, and a prefix on every datagram, in front of
clients that need neither. `web-transport-quinn` gives both kinds one session type, so the second path is a branch
at accept and not a second adapter.

**Evidence.** In the probe one endpoint served raw QUIC and WebTransport on one port; headless Chrome 157 connected
with no flag that touches certificates or QUIC and exchanged datagrams in both directions; the same client program,
built for the simulator, ran under `simctl spawn`.

### 3. The host's identity and the join ticket

**Decision.** When a host starts it makes a key and a self-signed certificate, held in memory only: ECDSA on
P-256, valid for 13 days. Its **join ticket** is the host's address and port and the SHA-256 of that certificate.
Every adapter pins the hash: a client connects only to a host whose certificate has exactly that hash, and checks
no name and no authority.

**Why.** A browser accepts a host with no public certificate only this way. `serverCertificateHashes` requires "an
X.509v3 certificate with a validity period of less than 2 weeks", a key that is ECDSA on P-256 and "must not
include RSA keys", and the hash "`sha-256`"
([MDN](https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport)). Pinning the same hash on the
native clients gives one trust rule for the three targets and needs no certificate store.

**What follows.** A ticket is good until its host stops, and a host must restart within 13 days. A ticket is about
a hundred characters as text: something to put in a link or paste, not to type. How a player receives it is
`SWARM`'s and `GAME-UI`'s. The page that opens a link must itself be a secure context; `http://127.0.0.1` is one.
A public host with an authority's certificate would add a second kind of ticket; it is not built.

**No early data.** 0-RTT is off, so nothing a client sent on an old connection can be replayed into a new one.

### 4. The world lane never waits on the network

**Decision.** On native targets the adapter owns one I/O thread that runs a single-threaded Tokio runtime, the
runtime Quinn supports. A host's thread serves all its links. The game's side and the I/O thread meet only at
bounded queues: a send copies the message into the lane's outbound queue and wakes the thread if it is asleep;
the thread moves arrived messages into the link's inbound queue, which the game drains when it chooses, normally
at the pre-tick barrier. In the browser the page's event loop takes the thread's place and the same queues sit between the
promise callbacks and the game.

**Why.** `SESSION` admits input at a barrier and must not find the world lane parked in a socket call
([networking design](../../design/modules/NETWORKING.md#commands-clocks-and-state-publication)). Driving Quinn's
sans-I/O core from the world lane would avoid the thread and would mean rewriting Quinn's socket layer; at four
links that buys nothing.

**What it costs.** One thread, and one hop between threads each way. Wakes are coalesced, so a tick's sends cost
about one.

### 5. Bounds, backpressure and liveness

Every queue has a cap, so a link's memory is the sum of its caps and a host's is that times its link cap.

- **Reliable, outbound.** At the cap a send returns `Full`. The game decides what a full lane means; C13 has it
  resynchronise or disconnect that peer.
- **Reliable, inbound.** When the game stops draining, the adapter stops reading the stream, QUIC's flow control
  stops the sender, and the sender's own lane fills. A stalled reader fills only the lane that feeds it.
- **Datagrams, outbound.** Bounded by age. Quinn's default buffer is 1 MiB, a minute and a half of input at the
  game's rate, all of it stale; the adapter holds datagrams itself and hands Quinn only what it can send now. The
  browser has the same control as `outgoingMaxAge`.
- **Datagrams, inbound.** A bounded queue that drops the oldest and counts it. Chrome's own queue holds one
  datagram by default, so the adapter raises it to its cap.
- **Liveness.** Each end sends keep-alives and closes a link as timed out after the profile's idle timeout with
  nothing heard. This is how a dead peer, a cut path or a frozen process is noticed, and it needs no traffic from
  the game. C11's "a suspended app has lost its socket" follows for native clients: a client frozen for longer
  than the timeout finds its link closed when it resumes, the host closed its end at the timeout, and the client
  opens a new link. Calling close when an app goes to the background is the shell's and `SESSION`'s.
- **A frozen page is not a dead link.** Chrome's network process answers keep-alives while a page's script is
  frozen: in the probe a page frozen for 9 s under a 4 s timeout kept its link and carried on. So the link reports
  a path or a process that has gone, never a peer that has gone quiet. A player who stops sending input is
  `SESSION`'s to time out, as its held-control expiry already requires.
- **The host.** A cap on open links and on handshakes in progress; a client beyond it is refused. A handshake that
  does not finish is dropped at a timeout.

### 6. Congestion control on the Quinn ends

**Decision.** The host and the native clients use Quinn's BBR controller. The profile can select Cubic, Quinn's
default, and M1 records both.

**Why.** A0's conditioner loses 2% of packets at random. A loss-based controller reads each loss as congestion, and
its rate is bounded by about `1.22 × packet size / (round trip × √loss)`
([Mathis et al., 1997](https://dl.acm.org/doi/10.1145/263932.264023)): about 100 KB/s here. In the probe Cubic
held 50 KB/s, because the conditioner's jitter also reorders packets and Quinn counts those as lost. The game's
steady traffic, about 11 KB/s each way for a link, fits under that. A join does not: a 1 MiB message from the host
took 18 s, later reliable messages waited seconds behind it, and the datagram round trip's 95th percentile rose
from 148 to 227 ms while it ran. BBR sets its rate from measured delivery, not from loss
([Cardwell et al., 2016](https://queue.acm.org/detail.cfm?id=3022184)); in the probe the same message took 0.7 s and
datagrams did not notice.

**What it costs.** Quinn marks its BBR "Experimental! Use at your own risk". Under the conditioner it sent 1.7 to
3.2 times the bytes Cubic did for the same transfer, because it declared many reordered packets lost.

**Chrome's sending** is Chrome's controller. The adapter passes the `low-latency` hint; Chrome 157 does not report
whether it honoured it. A 256 KiB message from Chrome took about 5.5 s in the probe. The game sends only input
upward, so `SESSION` should keep anything large flowing from the host down.

**What would change the decision.** If M1 finds BBR stalling, or its overhead growing with the length of a run,
the profile's default returns to Cubic and `SESSION` sizes its baseline for about 50 KB/s under loss. Either way
M1's record gives `SESSION`'s planner both controllers' rows.

### 7. Conditioning: one profile, two carriers

The **conditioner profile** is a one-way delay, a jitter range, a loss rate and a seed. **A0's profile**, used in
every proof here, is 50 ms ± 10 ms each way, uniform and independent for each unit, and 2% loss each way: A0's
"100 ms RTT, ±20 ms jitter and 2% loss with a fixed seed".

- **The in-memory link** is a pair of link ends in one process, with the whole contract of mechanisms 1 and 5:
  both lanes, the limits, `Full`, the age limit, events, the idle timeout and a host's link cap. It applies the
  profile to each message. A datagram may be lost, delayed and so reordered; a reliable message is only delayed,
  and a "lost" one pays a retransmission's delay, holding back those behind it. The caller supplies the time, so
  nothing sleeps, and the same seed and the same calls give the same events. It can also cut the path, and
  duplicate datagrams as a named fault beyond the contract, which `SESSION`'s proof order asks for and which
  serves here as the control that the link script's duplicate check can fail. It models no
  congestion control and no MTU.
- **The relay** is a small program that sits between a client and the host and applies the profile to each UDP
  packet. The QUIC packets it delays and drops are real ones, so loss recovery and congestion control run for
  real. It is proof tooling, never part of a game.

**Why a relay.** macOS's own conditioners (`dnctl`, Network Link Conditioner) need root or a person at a settings
pane, and act on the whole machine. Delaying messages inside the adapter would leave QUIC untouched, and mechanism
6 shows QUIC's response to loss is where the behaviour is decided.

**The old fixture** in `thrum-core`'s tests stays as the evidence of closed gates. New work uses this link.

### 8. One link script, four adapters

The **link script** is a fixed exchange written once against the link's interface. Run on an adapter, it proves
the contract there. Its parts:

1. **Lanes.** Patterned messages across each lane's range, both directions, sizes at 1 and at the limit. Reliable
   messages arrive once, in order, whole; datagrams arrive whole and at most once. Each end's counters match the
   script's own tally of what it sent and received.
2. **Limits.** A send over a limit or of nothing is refused at the call. A reliable lane driven to its cap returns
   `Full`, then drains with nothing lost. Datagrams sent into a path cut for a moment are discarded at the age
   limit and counted, and after the path heals none arrives that waited longer than the limit.
3. **Identity.** A ticket with one bit of the hash changed is refused, and so is another version.
4. **Lifecycle.** A close from each end with a code; a peer that vanishes; a path cut on a link that is sending
   nothing; a reconnect that gets a new identifier and receives nothing from the old link.
5. **Load.** The game's rates for a stated time: a 189-byte datagram at 60 Hz each way and a reliable message at
   10 Hz, with host-to-client transfers of 256 KiB and 1 MiB part-way through.

The in-memory link runs it in virtual time inside `./dev check`. The three socket adapters run it through
**the proof command**, one script with a target argument, directly and through the relay under A0's profile. The
**session run** puts four clients on one host through the relay for ten minutes: two on the Mac, one in Chrome, one
in the simulator, with each kind closed and reconnected, one killed, and one path cut and healed, while the
untouched links carry on.

**Under the conditioner the datagram lane must look unreliable**: some lost, some reordered. A run in which fewer
than 90% arrive, in either direction, fails as a collapse; at 2% loss about 98% should. That floor is the
only figure in this module that decides a result.

### 9. What a stranger can send

A host's port is open to anyone who can reach it, so every byte from a peer is untrusted.

- **Parsed here:** the length prefix of a reliable message, and a ticket's text. Both get a property test in
  `./dev check` over valid input cut at every byte, lengths of zero, the limit, one over and the largest a prefix
  can say, and random bytes; and a stdin target in the style of `thrum-network`'s. A peer that sends a prefix of
  zero or over the limit, opens a second stream, or sends a datagram over the limit has broken the link's rules:
  the link closes with that reason and the bytes are never delivered.
- **Parsed beneath:** QUIC, TLS and HTTP/3 are Quinn's, rustls's and `web-transport-quinn`'s. No cryptography is
  written here.
- **Proved on sockets in M2:** random UDP at the port, a wrong ALPN, a wrong path, each rule broken in turn, a
  stream stalled mid-message, a flood on each lane, more clients than the link cap and handshakes that never
  finish, each beside healthy links that must not lose a reliable message.

## The system as it runs

| State | Owner | Kind |
|---|---|---|
| A link's state, close reason, queues and counters; the link profile | `thrum-transport`, a new crate with no workspace dependency (this module) | Runtime |
| The QUIC connection, TLS session and congestion state | Quinn, or the browser, inside an adapter | Library |
| The host's key and certificate | The host, in memory, never written out | Secret |
| The join ticket | Made by the host; carried by the game | Value |
| The link script, the relay, the programs and the page that run them, the proof command | `examples/link-proof` and `scripts/` (this module) | Proof tooling |
| The match, epochs, sequences, what a message means | `SESSION`, in `thrum-network` | Not here |

The link's interface and the in-memory link build with no outside crate, so `SESSION`'s first tests need no
socket. The socket adapters are an opt-in part of the same crate, and a game that does not name `thrum-transport`
links none of it.

**Counters,** for each link: messages and bytes sent and received on each lane; sends refused, by reason;
datagrams discarded for age and dropped inbound; each outbound queue's depth and the age of its oldest message;
and, where the adapter has them, the round-trip estimate, packets sent and lost, and bytes on the wire. Quinn has
those. Chrome 157 has no `getStats`, so a browser link's own figures are marked absent and the host's side of the
same link supplies the estimate.

**A tick, walked.** (1) At the pre-tick barrier `SESSION` takes each link's arrived messages and events; nothing
blocks. (2) It stages and admits input and the world steps. (3) `SESSION` sends: each message is copied into a
lane's queue, and the I/O thread is woken. (4) The thread hands datagrams and stream bytes to Quinn, which
packs, encrypts and paces them. (5) Arriving packets are decrypted by Quinn; the thread reassembles reliable
messages and moves whole messages to the inbound queues.

**A join.** The host starts and its ticket exists. A client opens a link with the ticket; the handshake pins the
hash; a browser then makes its WebTransport request; the client opens the stream; both ends report the link open.

**A drop.** Nothing is heard for the idle timeout; both ends report the link closed as timed out; the game opens a
new link with the same ticket and `SESSION` starts a new epoch on it.

## Interfaces

Provided to other modules, as [the module list](../ARCHITECTURE.md#interfaces-track-3) names it:

| Interface | To | What it is |
|---|---|---|
| **Message link** | `SESSION` | Mechanisms 1 and 5: the two lanes with their limits, events and reasons, the profile, the counters, and a host's side |
| **In-memory conditioned link** | `SESSION` | Mechanism 7: the same interface with no socket, deterministic under a conditioner profile |
| **Join ticket** | `SESSION`, `SWARM`, `GAME-UI` | Mechanism 3: what a client needs to reach and trust one host |
| **Proof command and relay** | `SESSION`, `SWARM` | Mechanisms 7 and 8: how a match is run across the three targets under a conditioner profile |

The link is promised the same across adapters for one version of the link. It is not promised across versions.

**No contract changes.** C13's "Networking-off removes sockets" is met by the crate boundary. A0 still selects a
WebSocket fallback; this module does not build it. `thrum-network` is not edited: `SESSION` adds the dependency
when it needs it.

## Cost

No budget is set (brief, decision 3). The account, per link at the game's rates:

| Work | Count | From the probe |
|---|---|---|
| Datagrams sent and received | 60 a second each way, 189 bytes for four input records | About 11 KB/s each way |
| Reliable messages | About 10 a second, and a baseline at a join | 104 ms median round trip under A0's profile |
| Copies of a message | One into a queue; Quinn's own; for a browser link the host's session prefix adds one more for each datagram | Not measured |
| Thread wakes | About one for a tick's sends; one for each arriving packet | Not measured |
| A join's handshake | Once | 0.5 ms direct; about one round trip for raw QUIC and three for WebTransport under the conditioner |

A host with four links moves about 500 packets a second, so the cost that matters is latency added to a message,
not throughput. The design keeps that to one queue and one wake. The adapters are written to reuse their buffers,
and M1 counts what a warmed send and drain allocate.

**Baselines and tripwires.** Every time, byte count and allocation count this module records is a baseline. The
cost command succeeds when its record is complete and made at the claimed commit, whatever the figures are. Three
things are flagged in the record and investigated while building goes on: a row about a quarter worse than the
last comparable one; a datagram round trip that rises while a reliable transfer runs; and bytes on the wire for
each application byte that grow with the length of a run.

## Risks

- **BBR is experimental upstream.** Early sign: M1's overhead row grows, or a run stalls. Mechanism 6 says what
  changes.
- **Chrome differs from the probe**: the Wasm adapter, a headed window, a later version. Early sign: M1's Chrome
  row loses datagrams the Mac row keeps. The record holds the browser's version, and the builder stops as
  [the handoff](#proof-and-builder-handoff) says.
- **A convincing fake of the conditioned run:** a client that connects straight to the host. The record holds the
  relay's forwarded and dropped counts and the measured round trip, which must show the profile.
- **A convincing fake of a reconnect:** a paused exchange on a link that never closed. The record holds both link
  identifiers and the close reason between them.
- **Loopback hides the network.** MTU below 1,500, NAT rebinding, IPv6, Wi-Fi's bursts and a second machine are
  not run. Nor are a physical iPhone, where the local-network permission applies, Safari, Firefox, or a page from
  the public web reaching a host on a private network, which meets the browser's local-network rules.
- **A hidden or frozen tab** stops sending and keeps its link, as mechanism 5 says. Early sign: a browser player
  who stands still while the host shows the link open. `SESSION` times out silent input; this module records what
  the host sees.
- **The lockfile.** New dependencies must not move the version of a crate already locked, or an exact image
  baseline may move. Each milestone carries the verifier.

## Proof and builder handoff

- **Targets.** The host always runs natively on the Mac. Clients run natively on the Mac, as a plain executable in
  the iPhone 18 Pro simulator through `simctl spawn`, and as Wasm in headless Chrome with no flag that touches
  certificates, QUIC or origins. None needs the GPU. The simulator proves function, never speed.
- **The sandbox.** Sockets, Chrome and the simulator do not work inside Codex's `workspace-write` sandbox, and
  nearly every proof here needs one of them. So every socket proof is behind the one proof command, which a
  builder or a reviewer asks to run outside the sandbox, and nothing that opens a socket runs under `cargo test`'s
  defaults or in `./dev check`, where it would fail every other module's builder. A reviewer whose request is
  refused cannot judge those items, and the review goes to Claude. Fetching the new crates needs the
  network once. [Whether to keep asking for each run is the owner's](../ARCHITECTURE.md#owner-decisions).
- **Costs never stop the build.** A row that trips a tripwire is recorded as measured, with what was found.
- **Stopping.** If the engine's Chrome adapter cannot hold datagrams to the local host, or the simulator cannot
  reach it, the builder keeps the measurements and ends `BLOCKED`. A planner then applies the fallback the module
  list named: input on the reliable lane for that adapter, with its cost recorded.
- **Exact baselines.** This module adds a crate, scripts and lockfile entries, which are covered paths, and moves
  no retained image.

## Open questions

- Whether Quinn's loss detection should tolerate more reordering under A0's profile. M1 records the default; a
  builder who tunes it records both.
- Whether `SESSION`'s baseline fits a join under each controller. Settled by M1's transfer rows and `SIM-2D`'s
  state size.
- Whether the I/O thread needs a priority beside the render and world threads. Settled when `SWARM` measures the
  link inside the running game; here it runs at the default.
