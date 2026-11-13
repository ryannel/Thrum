# TRANSPORT: the planner's answer to the design review

13 November 2026. Answers [the design review](design-review.md) of commit `f5749921`, which asked for changes on
three Must Fix findings and made two suggestions. [The design](../../plans/TRANSPORT/DESIGN.md) and
[the done-when file](../../plans/TRANSPORT/DONE.json) hold the result; this page holds the reasoning, what was
considered and not chosen, and a second probe run to check the new mechanisms before anyone builds on them.
[The log](../../plans/TRANSPORT/LOG.md) has one short entry for each.

| Finding | Answer | Where |
|---|---|---|
| Must Fix: datagram expiry after handoff | The promise is narrowed to the link's own queue and said outright. A timer enforces it there; a stack allowance caps what sits beneath with no deadline; `SESSION` rejects old datagrams by content | Design, mechanisms 1, 5, 7 and 8; M1.1 to M1.3; the module list's message-link row |
| Must Fix: native link open needs wire progress | The link hello: each end writes a short frame of the link's own at startup and reports open on reading the other's, under a startup timeout | Design, mechanisms 1, 2, 8 and 9; M1.1, M1.6, M2.1, M2.2 |
| Must Fix: session proof promises delivery after death | M1.5 and M2.9 are rewritten from delivery or closure, link by link | Design, mechanisms 1 and 8; M1.5, M2.9, M1's bar |
| Suggestion: datagram cap policy | Taken | Design, mechanisms 1 and 5; the counters |
| Suggestion: lifecycle contract citation | Taken | Design, mechanism 5 |

## Datagram expiry after handoff

The review is right that the first design promised more than Quinn can do. Quinn 0.11.12 queues a datagram with
no expiry, sends it when congestion control and pacing allow, and has no call that takes one back (`quinn-proto`
0.11.19, `src/connection/datagrams.rs`). No sender can bound what the network delays either. So the choice was
between a promise that covers the adapter's own queue and one that cannot be kept.

**What the design now says.**

- A datagram's age starts when the send call takes it and stops when it leaves the link's own queue for the QUIC
  stack. A timer discards it at the age limit whether or not anything is sent after it.
- The queue also has a cap. At the cap the newest is taken and the oldest discarded, counted apart from expiry.
- Beneath the queue, the **stack allowance** is how many bytes of datagrams the stack may hold. They have no
  deadline, and they are the only datagrams this end itself can hold back. The native adapter sets Quinn's
  datagram send buffer to the allowance and hands on with `send_datagram_wait`, which waits for room and drops
  nothing. The browser adapter writes only when the page's datagram writer is ready, keeps the browser's buffer
  small and sets `outgoingMaxAge`.
- The expired count is the link's queue only. What a browser discards itself is absent from the record, never
  zero.
- `SESSION` rejects old datagrams by content: input by target tick against its bounded window, state by envelope
  sequence. C13 and the networking design already ask that of it, because the lane reorders in any case.
- The module list's message-link row and `TRANSPORT`'s entry there say so, the design lists it under "What
  `SESSION` must not assume", and M2.7 has the game maker's guide say it.

**The proof.** The link script gains an expiry part. The path stalls; the script sends until the sender's queue
has backed up, for one age limit more, and stops. With the path still stalled and another age limit gone, the
queue must be empty and the sender's counters must account for every datagram: handed on, expired, or dropped at
the cap. Only then does the stall end. Expiry is read from the sender's counters while the path is still cut, so
it cannot be confused with a packet the relay dropped, and the last datagram queued has no later send to push it
out, which is the review's counterexample.

**Considered and not chosen.**

- *Pushing Quinn's last datagram out with an empty filler datagram.* `send_datagram` discards the oldest queued
  to make room, so a filler could evict a stale one. It depends on behaviour Quinn does not promise, sends a
  zero-length datagram the peer must then ignore, and does nothing for the browser.
- *A receiver-side age check.* It needs a clock the two ends share, and clock sync is `SESSION`'s.
- *An allowance of zero.* Quinn refuses a datagram larger than its buffer, so the buffer must fit one.
- *Keeping the stronger promise for Chrome only*, where `outgoingMaxAge` may give it. The link is promised the
  same on every adapter, so the contract is the weaker one and Chrome's tighter bound is recorded, not promised.

**Not verified.** Whether Chrome's datagram writer signals that it is full on a stalled path. The Chrome row
records whether the page's queue backed up and does not depend on it; the host-to-Chrome direction is a Quinn end
and must show expiry.

## Native link open needs wire progress

Quinn tells the accepting end about a stream only when the opener writes to it, and `web-transport-quinn` writes
no header on a raw connection's stream (`src/session.rs`, `header_bi` is empty). The planning probe's client
always sent first and so hid this.

The link now has a startup of its own. The client opens the stream and writes the **link hello**; the host reads
it, writes its own and reports open; the client reports open on reading the host's. A hello is never delivered
and is in no message counter. A first frame that is not the hello breaks the link's rules. Startup has one
deadline on both ends, the profile's startup timeout.

A hello in both directions was chosen over a client-only one. With the client's alone, the client would report
open before anything had come back on the stream. With both, open means the same thing on every adapter: the
stream has carried bytes each way. It costs the client one round trip, measured below.

The proof: the script opens a link on which neither game sends and takes the host's reliable message as the
first; it opens a link into a cut path, which must close as timed out at the startup timeout; M2.1 treats the
hello as untrusted input; M2.2 adds a first frame that is not the hello and clients that never send it.

## Session proof promised delivery after death

M1.5 required every reliable message on every link to arrive in a run that kills one client and times another
out, and asked for a close reason from both ends of each. The design's own rule is delivery **or closure**, and
the design now states its consequence: a close, even a clean one, can cut off messages already accepted.

The run is now judged link by link:

- A link still open when the run ends has lived through the others' events. The script stops sending, waits and
  closes, and every message that link accepted has arrived once and in order. This keeps the review's "healthy
  links lose no reliable messages".
- A link that an event ended delivered an unbroken run from the first message sent, and nothing after it on any
  link. Messages are patterned by link and position, so the receiver checks this without the sender's tally,
  which matters when the sender is dead.
- A killed process reports nothing. What it had received is not claimed. The kill and the cut are the harness's
  notes, kept apart from what links reported.
- Every link carries load until its own end reports the close. The record must show messages accepted and never
  delivered for the cut, and the host accepting messages for the killed client after the kill. A run that drains
  every link before each event shows neither and fails.

The check that a timeout comes from the link itself, with no traffic on it, stays in the script's lifecycle part.

## The suggestions

Both taken. The cap policy was needed anyway: the expiry accounting works only if every discard has a name. The
citation was wrong: the suspend rule is in
[the networking design](../../design/modules/NETWORKING.md#identity-connection-and-content), not in C11.

## Also changed

- The 90% floor on datagrams delivered now counts the script's lanes and load parts only. The limits and expiry
  parts discard on purpose.
- M1.6 records the time from connect to open, since startup now includes the hello.

## The probe

Run by the planner to check, on sockets, the two Quinn behaviours the answers above rest on. The review
established both gaps by reading source and ran no socket.

**Method.** One Rust program: a Quinn 0.11.12 host and client (`quinn-proto` 0.11.19, rustls with ring, a
single-threaded Tokio runtime), joined through a UDP relay in the same process that can cut the path and, when
asked, add 50 ms ± 10 ms of delay and 2% loss each way. The client's datagrams pass through a queue built as the
design describes: age from the send call, a 250 ms age limit enforced by a timer, a cap of 64, and handoff by
`send_datagram_wait` into a Quinn datagram buffer set to the allowance. Each datagram carries its send time, so
the host records its age on arrival. A run warms the path at 60 Hz, cuts it, sends 1,000-byte datagrams, stops,
waits two age limits with the path still cut, reads the counters, heals the path and waits 3 s. This M3 Max,
Rust 1.99.1, release build, while other agents were building. One run for each row unless it says otherwise.

**Stream opening.** In all fifteen runs the host's `accept_bi` had not completed 2 s after the client's
`open_bi` with nothing written. After a four-byte hello from the client the host accepted the stream, and the
client read the host's hello and a first message from the host 0.25 to 1.4 ms later on the direct path and 98 to
102 ms later under the delay, which is one round trip.

**A stalled path, through the link's queue.** Nothing arrived during a cut in any run, the queue was empty at
the end of every cut, and the three counts summed to what was sent in every run.

| Path | Controller | Sent in the cut | Handed on | Expired | Dropped at the cap | Arrived after the cut, and how old | Congestion window, bytes |
|---|---|---|---|---|---|---|---|
| Direct | Cubic, six runs | 300 at 200 Hz | 24 to 26 | 274 to 276 | 0 | 1 to 3; 1.9 to 3.0 s | 12,000 |
| Direct | BBR | 300 at 200 Hz | 275 | 25 | 0 | 3; 0.67 to 0.78 s | 273,651 |
| Direct | BBR, allowance 1,100 bytes | 300 at 200 Hz | 269 | 31 | 0 | 1; 0.67 s | 273,648 |
| Delay and loss | Cubic | 300 at 200 Hz | 11 | 289 | 0 | 3; 1.5 to 2.1 s | 2,904 |
| Delay and loss | BBR | 300 at 200 Hz | 269 | 31 | 0 | 1; 1.1 s | 272,870 |
| Delay and loss | BBR | 2,499 at 1,000 Hz | 275 | 62 | 2,162 | 3; 2.8 to 3.9 s | 272,880 |
| Delay and loss, after 1 MiB sent up the stream | BBR | 2,499 at 1,000 Hz | 15 | 64 | 2,420 | 2; 1.8 to 2.7 s | 451,137 |
| Delay and loss, after 1 MiB sent up the stream | Cubic | 2,500 at 1,000 Hz | 11 | 64 | 2,425 | 2; 3.8 to 5.0 s | 4,600 |

The allowance is 4,096 bytes except where the row says. The cut lasted 2 s at 200 Hz and 3 s at 1,000 Hz.

**The same cut with the first design's handoff**: every datagram given at once to Quinn's default 1 MiB buffer,
with no queue of the link's.

| Controller | Sent in the cut | Arrived after the cut, and how old |
|---|---|---|
| Cubic | 300 | 277; 1.4 to 3.4 s |
| BBR | 300 | 28; 0.69 to 0.89 s |

**What it shows.**

- The review's counterexample is real: with Quinn's default buffer, hundreds of datagrams arrive seconds old.
- With the link's queue and the allowance, the queue expires what it holds with no later send, the accounting is
  exact, and what arrives late is at most what the allowance held: one to three datagrams, 0.7 to 5 s old. That
  is the late datagram the narrowed promise admits and `SESSION` must reject.
- A stalled stack does not stop at once. It first sends a congestion window's worth into the cut, where it is
  lost: 3 to 12 KB under Cubic, 273 to 451 KB under BBR. So the script's expiry part sends until the queue backs
  up and not for a fixed count: 300 KB was only just enough under BBR here.
- The 1 MiB transfer took 18.4 s under Cubic and 0.54 s under BBR, which agrees with
  [the planning probe](planning-probe.md).

**Limits.** One process on loopback, on a contended machine, with few runs. It is not the engine's adapter. Only
the client-to-host direction was driven. Chrome and the simulator were not run, so nothing here says how
Chrome's own datagram queue behaves on a stalled path. The relay was a simpler one than the proof's. The source
was scratch and was thrown away, as the planning probe's was; M1 repeats all of this through the engine's
adapters.
