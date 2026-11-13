# SESSION planning probe, 13 November 2026

A throwaway timing model the planner ran before writing [the design](../../plans/SESSION/DESIGN.md). It answers
three questions the design had to settle and that reading alone could not. It is **not** engine code and proves
nothing about the engine: milestones M2 and M5 repeat every row through `thrum-network` and `thrum-transport`. The
model's source stayed in the planner's scratch folder and is not kept, so nothing here is a baseline.

## What ran

One Python file, no dependencies, in virtual time. It has no world and no bytes: it models only when things arrive.

- **Host.** Commits one tick every 1/60 s. At each tick's barrier it takes what has arrived, notes whether the
  client's frame for that tick is there (if not, the tick is **held**), and sends the client one datagram that
  holds every admitted set the client has not yet acknowledged.
- **Client.** Runs its own 60 Hz ticks, out of phase with the host. Each tick it authors one frame for tick
  `now + lead`, and sends a datagram with its most recent frames (eight unless a row says otherwise) and the
  highest tick up to which it holds every set.
- **Lead.** The host reports, for the client's newest frame, how many barriers before its own tick the frame
  arrived (its **margin**; negative when late). Every half second the client takes the 1st percentile of the last
  five seconds of margins: below one, it raises the lead by the shortfall; two or more, it lowers the lead by half a
  tick. The lead starts at two ticks, which is too short on purpose.
- **Link.** Each datagram, each way, is lost with a fixed probability or delayed by a uniform, independent time,
  so datagrams reorder. Three profiles: **A0** (50 ms ± 10 ms each way, 2% loss: A0's 100 ms round trip, ±20 ms
  jitter and 2% loss), **harsh** (125 ms ± 25 ms, 5% loss) and **local** (1 ms ± 0.5 ms, no loss).
- **A reliable lane, for comparison.** The same sets sent as one ordered message a tick. A lost message arrives
  one and a half round trips later and everything behind it waits. This is a model of a stream, not QUIC.
- 36,000 ticks (ten minutes) a row, two seeds. The second seed's figures were within a few percent of the first.

## 1. Can the client size its input lead from what the host reports?

Yes, in this model, and a fixed lead cannot cover the range.

| Row | Lead settles at | Ticks held, of 36,000 | Of those, in the first minute |
|---|---|---|---|
| A0, adaptive | 4.6 ticks (77 ms) | 45 (0.13%) | 39 |
| Harsh, adaptive | 9.6 ticks (161 ms) | 196 (0.54%) | 95 |
| Local, adaptive | 1.6 ticks (27 ms) | 1 | 1 |
| A0, lead fixed at the old fixture's 5 ticks | 4.6 | 12 (0.03%) | 6 |
| Harsh, lead fixed at 5 ticks | 4.6 | 36,000 (100%) | 3,599 |
| A0, then harsh from tick 12,000, then A0 from tick 24,000 | 4.6, then 9.6, then 4.6 | 131 (0.36%) | 39 |

In the last row 46 ticks were held in the ten seconds after the link worsened, and 42 in the three minutes after
that; when it recovered the lead came back down within about ten seconds and no tick was held while it did.

After the first minute a held tick needs two losses in a row. At A0 that is about one tick a minute.

The lead of 4.6 at A0 agrees with the five ticks
[the A5 timing decision](../../design/decisions/A5-INPUT-TIMING-RECONCILIATION.md) chose by hand for that profile.
The half tick comes from the phase between the two clocks.

**How many frames should an input datagram repeat?** At A0, with the lead as above:

| Frames in each datagram | Ticks held |
|---|---|
| 1 | 2.09% |
| 2 | 0.14% |
| 4 | 0.13% |
| 8 | 0.13% |

Two frames already recover almost every single loss. Eight, the codec's limit, buys nothing more at this loss
rate and costs 124 more bytes a datagram.

## 2. Should the admitted sets travel as datagrams or on the reliable lane?

As datagrams that repeat what is not yet acknowledged.

| Profile | Lane | Delay from the host committing a tick to the client being able to step it: mean | 99th percentile |
|---|---|---|---|
| A0 | Datagrams, each with every unacknowledged set | 76 ms | 91 ms |
| A0 | One reliable message a tick | 227 ms | 362 ms |
| Harsh | Datagrams | 164 ms | 184 ms |
| Harsh | Reliable | 720 ms | 1,265 ms |
| Local | Either | 3 ms | 3 ms |

The delay includes a playout buffer sized to the 99.9th percentile of recent lateness, which is what a client
needs in order to step at an even pace. On the reliable lane one loss holds back every later tick, so the buffer
has to cover a retransmission. With datagrams the next one, a tick later, repairs the loss.

**How many sets does a datagram carry?**

| Profile | Median | 99th percentile | Most |
|---|---|---|---|
| A0 | 7 | 8 | 10 |
| Harsh | 16 | 19 | 21 |
| Local | 1 | 1 | 1 |

That is the round trip in ticks, plus one. At eight or nine bytes for each player whose axes changed, four players
and 21 ticks come to about 750 bytes, which fits the link's 1,000-byte datagram with the 64-byte envelope. A
longer run does not fit and needs the reliable lane.

## 3. How far ahead of the confirmed world is a player's own input?

The lead plus the confirmed delay: about 150 ms (9 to 10 ticks) at A0 and 325 ms (20 ticks) at harsh. Three things
follow for the design:

- with no prediction a player's own character answers a key press 150 ms late at A0;
- a predicted controller replays 9 to 10 of its own steps at a correction at A0 and about 20 at harsh, so the old
  fixture's cap of nine replay steps is too small and its ring of 64 records is enough;
- rolling the whole crowd back that far costs that many ticks. [SIM-2D's probe](../SIM-2D/planning-probe.md)
  measured 5.0 ms a tick at 10,000 bodies on the Mac and 5.6 ms in Chrome, so about 50 ms a correction at A0,
  against a frame of 8.3 ms at 120 Hz.

## What the model leaves out

- Real congestion control, pacing and MTU. A datagram here is lost or delayed on its own.
- Bursts of loss. Losses are independent, so runs of three or more are rarer than on a real path.
- The cost of stepping: a client that cannot step as fast as the host is not modelled.
- The bytes themselves. Sizes above are arithmetic from the design's encoding, not measured.
