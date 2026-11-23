# SESSION: two to four players share one run and keep seeing the same crowd

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C02, C03 and C13 in [CONTRACTS](../../design/CONTRACTS.md),
[the networking design](../../design/modules/NETWORKING.md), [the A0 decision](../../design/decisions/A0-N1-TRANSPORT-CODEC.md),
[the input wire](../../design/decisions/A5-INPUT-WIRE.md) and
[the A5 timing decision](../../design/decisions/A5-INPUT-TIMING-RECONCILIATION.md) govern.
[SIM-2D](../SIM-2D/DESIGN.md) and [TRANSPORT](../TRANSPORT/DESIGN.md) provide what this module stands on.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module SESSION. It
needs: SIM-2D, TRANSPORT", which is row 19 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../BRIEF.md) carries
as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

That page also sets defaults, which are an agent's and not the owner's: cooperative online play for 2 to 4 players,
and thousands of enemies and projectiles on screen at once. The module list settled that
[the crowd is shared by causes, not by state](../ARCHITECTURE.md#mechanisms-settled-here): every peer steps the same
rules from the same inputs, and no enemy's position travels. `SIM-2D` makes the rules give the same bytes
everywhere. `TRANSPORT` moves bytes between processes. This module is the match between them: who is in it, whose
input counts for which tick, how a newcomer gets the world, and what happens when a copy goes wrong. The module
list's brief for it, which [its entry](../ARCHITECTURE.md#session) now replaces with a pointer here, left three
choices to this planner: whole-world rollback or a confirmed crowd with a predicted player, how often digests
travel, and how a modal choice reaches every peer in order.

This design reconciles the code on main; it is not a blind study. Read for it: `thrum-network` (the envelope, the
input batch, the future-input window and the state-delta codec, none of which runs a session), `thrum-core`'s
`sim` module (the runtime, its capture and restore, history, input), the contracts and decisions above, the two
designs this module depends on with their probes, and [a planning probe](../../../.orchestrate/evidence/SESSION/planning-probe.md)
run for it. `SIM-2D` and `TRANSPORT` are designed and not yet built, so what this design says of their code is
what their designs promise.

**14 November 2026.** The owner said: "Let's move the 2d engine side of things to before v1 and decouple them.
But we don't want to start working on the swarm game till v1 lands." This module is engine work and waits for
nothing of V1's. One piece of engine work the game's plan had carried, a world that steps when a match says,
moved here as [mechanism 12](#12-a-driven-world) and M7, so that it is built and proved without the game.

## Purpose and scope

Players on separate machines hold out against one crowd. Each sees the same enemies in the same places, each
character moves when its own player says so, and a player who arrives late or drops out comes into the same run.

**Words used here.** A **match** is one run shared by a host and its players. The **host** is the native process
that holds the authority world. A **peer** is any process in the match. A **player slot** is one of the places a
player can hold; a match has up to four. The **roster** says which peer holds which slot. An **admitted set** is
everything the host admitted for one tick. The **world image** is a world's whole authority state as bytes. A
client's **progress report** is two ticks: **held**, up to which it holds every set, and **stepped**, which its
world has committed. Their difference is its **backlog**. A client's **playout position** is where its picture
stands on the tick line; a **stall** holds it still and a **skip** moves it on at once
([mechanism 6](#6-the-playout-clock)).

**Consumers.** `SWARM` declares its rule set and actions to a match, starts a host, and reads the confirmed world
and its own predicted character. `GAME-UI` shows what the session reports about each connection.

**What exists.** Byte codecs for the envelope, the input batch and acknowledged state deltas; a window that stages
future input frames; a runtime that can capture and restore a world inside one process. Nothing negotiates,
relays, joins or repairs, and no test puts two worlds in step.

**Commissioned.** The twelve mechanisms below. M1 to M6 prove the first eleven with no window: by four players
and a host on the in-memory link with [the reference crowd](../SIM-2D/DESIGN.md#7-the-reference-crowd), then on
real sockets across `mac-metal`, `chrome-webgpu` and `ios-sim`. M7 proves the twelfth, and what a player sees of
the sixth, in a window on the Mac and on a Chrome page.

**Not built.** Host migration: when the host stops, the match ends. Matchmaking, accounts, passwords and a public
host. Interest management: every peer holds the whole world. Whole-world rollback. Spectators. A replay file. Any
protection against a player who reads the world from a modified client. More than four players. Travelling state
for a game without deterministic rules: A5's acknowledged deltas stay in `thrum-network`, unused and unqualified
by this module. Drawing a game: M1 to M6's proofs have no window, and M7 draws only the reference crowd, through
`RENDER-2D`'s crowd view. Extrapolating a picture past the newest confirmed tick: in a stall the picture holds.

## What a consumer must be able to do

| Need | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **N1 Keep every peer's world the same** | A host and 1 to 4 players; 0 to 10,000 bodies; 36,000 ticks; A0's profile and a harsh one (250 ms round trip, ±50 ms, 5% loss), with reordering and duplicated datagrams; any mix of the three targets | Peers that share one world in one process, or digests compared once at the end. Rejected by separate worlds, on sockets separate processes, compared at every digest tick |
| **N2 Get input to the host in time as the link changes** | Round trips from under 2 ms to 250 ms; a link that worsens and recovers mid-match | A fixed lead. The old fixture's five ticks pass A0 and miss every tick on the harsh profile ([probe](../../../.orchestrate/evidence/SESSION/planning-probe.md)). Rejected by the harsh profile and a profile change, with a pinned lead as the control that must fail |
| **N3 Make a one-time choice happen once, in order, on every peer** | Under loss, a stalled link, and a choice made while a join is under way | Sending it on a timed input frame, where a late frame is dropped. Rejected by commands issued into loss and a stall |
| **N4 Join a match that is already running** | A world of 10,000 bodies at any tick; while the others play; a join that takes longer than a second; two joins under way at once | Everyone starting together at tick 0. Rejected by a join after tick 3,600 with slots reused |
| **N5 Drop and come back** | A link lost for longer than the idle timeout; a client killed and started again; an app sent to the background | Pausing on a link that never closed. Rejected by a new link and epoch, the same slot, and old input refused |
| **N6 Have a wrong copy found and put right** | Any one value the digest covers, changed on one peer and still different at a digest tick; a peer that goes wrong again and again. A difference the rules overwrite before a digest tick is outside this need ([mechanism 8](#8-digests-and-repair)) | A comparison that never fires. Rejected by a forced difference, in state the rules carry forward, that must be found at the next digest tick |
| **N7 Play on when one peer is slow, silent or hostile** | A peer that stops reading, reads on time and steps at half speed, freezes, floods or sends malformed bytes | A host that waits for every peer's input, or one that watches only what a peer received. Rejected by the host committing every tick on schedule beside each kind, and by a slow stepper that never stops acknowledging |
| **N8 See your own character answer at once** | Every tick the host admitted your frame; a tick it did not; a link stalled for longer than the history | Drawing the confirmed position, about 150 ms late at A0. Or a prediction never checked against the host, which drifts. Rejected by exact equality with the confirmed state, tick for tick |
| **N9 Be refused when your copy would disagree** | Another protocol, another rule set, another build of the same rule set, a full match | A version number someone has to remember to raise. Rejected by a changed rule constant that must change the identity with no other edit |
| **N10 Know what the match is doing** | Each state a connection passes through, with the reason for a refusal or a close; round trip, lead, bytes, corrections | Counters on the host only. The record names each peer's own |
| **N11 Pay nothing when the game is not networked** | The V1 game, the starter, the creator scenes; and a match rehearsed with no socket | A feature flag in a crate every game links. Rejected by a check of each package's dependency closure |
| **N12 See the crowd move evenly while the sets arrive unevenly** | Sets that arrive together inside the playout buffer; a buffer that runs out; a client behind by up to the behind threshold, and by more; a display faster or slower than the tick rate; a browser tab hidden and shown | Committing every set held and drawing between the last two ticks. Digests and interpolation values all pass while the picture jumps (the design review of 14 November, in [the log](LOG.md)). Rejected by a bound on how far the position moves between two frames, with that policy as the control that must fail |

## Mechanisms

### 1. One authority, shared by admitted sets

**Decision.** The host commits one tick at a time from the frames it admitted for that tick, and once the tick
has committed it sends each client exactly what it admitted. A client commits tick `n` from the host's admitted
set for `n` and from nothing else. Its own input reaches its own world only by coming back in a set.

**The admitted set** for tick `n` is, for each player slot in order, either the frame the host admitted or
nothing, and after them any roster events of [mechanism 2](#2-the-world-knows-slots-the-session-knows-peers). A
slot with no frame holds its last axes and fires no action, on every peer alike (C13). The host builds the set at
the pre-tick barrier, steps, and sends the set only when that step has committed.

**A set is sent only for a committed tick.** A set's arrival is therefore the confirmation that the authority
committed that tick with that input, as C13 and
[the A5 timing decision](../../design/decisions/A5-INPUT-TIMING-RECONCILIATION.md) require: origins are "reported
committed only **after** `Simulation::step` commits". No set is a proposal and a client needs no second signal.
The price is that a client's wait for a set includes the host's own tick: about 5 ms at 10,000 bodies by
`SIM-2D`'s probe, beside a wait of 76 ms at A0. M2 records it.

**When the host's step fails.** The runtime can refuse a tick and leave the world where it was (`TickError` in
`thrum-core`'s `sim`). A game in a match makes its rules step for every set its admission accepts, so no peer's
input can cause a refusal: a frame is refused at admission, or a rule that finds it senseless changes nothing
([mechanism 4](#4-timed-frames-and-ordered-commands)). A step that still fails is a fault of the authority. The
host sends no set for that tick and does not retry it, since the same set on the same world fails the same way.
It ends the match: on the reliable lane each client gets the sets it has not acknowledged and then a
**match-ended message** with the reason and the host's last committed tick, and the host's game is handed the
error. A client steps what it holds up to that tick and stops there. Any frame or command of its own that those
sets did not answer is resolved as not committed, because the host has said where commitment stopped. A host that
stops on purpose ends the match with the same message. If the host vanishes without it, as when its process dies,
a client's unanswered inputs are uncertain and are not resent (C13).

**Why this gives one world.** `SIM-2D`'s profile makes X[n+1] a function of X[n], the admitted frames and the rule
set's constants. Every peer has the same three, so every peer has the same X[n+1]. C13's authority is kept whole:
only the host decides what a tick's input was, and a client that alters its rules changes only its own copy.

**No committed tick is rewound,** on the host or on a client. The host admits a frame for a tick or does not, and
that is final. G2S's corrected replay stays in the engine, unused by a match.

**Evidence.** Factorio shares its world this way: clients "simulate the game state and they only receive and send
the player input", and the server's job is "making sure all clients execute the same actions in the same tick"
([Friday Facts 302](https://factorio.com/blog/post/fff-302)). Like Factorio's, this host does not wait for a
lagging client.

**What would change the decision.** If `SIM-2D`'s first milestone cannot hold the three targets equal,
[the module list's fallback](../ARCHITECTURE.md#mechanisms-settled-here) applies and this design is replanned. If
M2's record shows the host's tick to be a large part of a client's wait, a later plan can send a set early as a
proposal and confirm it with a committed tick in the next datagram; that is not built here.

### 2. The world knows slots, the session knows peers

**Decision.** A game's world holds player slots and nothing about connections. The roster, which is session
state, maps each slot to a peer and a connection epoch, and says whether its player is present or away. A frame
is handed to the game stamped with its slot: the origin's peer is the slot, its epoch is fixed for the match and
its sequence is the tick. The real origin (peer, epoch and the client's own sequence) stays in the session, which
uses it to tell each client what became of its frames.

**Why.** A join, a drop, a return and a reconnect then change the roster and never the world's bytes, so they
cannot make two worlds differ and the digest needs no knowledge of connections. A returning player finds the same
character, because the character belongs to the slot.

**Roster events.** There are four: joined, away, back and left. The host puts each in an admitted set, so every
peer sees it at the same tick. The game is told and decides what it means: a hero appears, stands idle, leaves.
The session itself does one thing for a slot that goes away: the set for that tick carries a frame of neutral axes for it,
which is C13's "held controls … become neutral" made a cause every peer steps.

**A listen host's own player** holds a slot like any other. Its frames skip the link and nothing else: they are
staged for a tick, admitted at the barrier and sent in the set (C13's one rules path).

### 3. The set stream

**Decision.** Sets travel to each client on the datagram lane. Each tick the host sends that client one datagram
holding every set the client has not yet acknowledged, oldest first. A client acknowledges by tick, in the
**progress report** that rides in each datagram it sends: `held`, the highest tick up to which it holds every
set, and `stepped`, the tick its world has committed. The host frees sets up to `held`. A client sends the report
from the moment it has a baseline, whether or not it has frames to send. `stepped` is never above `held`, and a
report that says otherwise is a protocol error. Datagrams reorder, so the host keeps the highest of each tick it
has been told and ignores an older report.

**Why not the reliable lane.** A client cannot step tick `n` without set `n`, so on an ordered stream one lost
packet holds back every later tick until it is resent. In [the probe](../../../.orchestrate/evidence/SESSION/planning-probe.md)
the wait from the host committing a tick to a client being able to step it, with the buffer a steady pace needs,
was 76 ms on datagrams and 227 ms on a reliable lane at A0, and 164 ms against 720 ms on the harsh profile. With
datagrams the next one, a tick later, repairs a loss. This is the practice for input-only netcode: "redundantly
include all inputs in each UDP packet until we know for sure the other side has received them"
([Fiedler, Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/)).

**Encoding.** The stream is new wire, so it is compact from the start: a set is written once and sent to every
client, a slot whose frame repeats the last axes with no action costs bits, and a changed slot costs its three
16-bit axes, its client's sequence and its actions. The probe's datagrams held a median of 7 sets at A0 and at
most 21 on the harsh profile, which fits the link's 1,000 bytes for four players who all move every tick.

**When a run is too long for a datagram,** after a stall or during a join, the host sends the sets on the reliable
lane, many to a message, and returns to datagrams when the client is within a datagram of the present. A client
takes sets from both lanes by tick and ignores one it has.

**The ring.** The host keeps, for each client, the sets it has not acknowledged, in a ring with a cap. A client
that falls off the end is not sent what it missed; it is resynchronised by [repair](#8-digests-and-repair) or
closed ([mechanism 10](#10-slow-silent-and-hostile-peers)).

**The backlog.** `held − stepped` is the sets a client holds and has not stepped. Both ticks come from the client
in one datagram, so the link's delay is not in the figure. Receiving and stepping are thereby seen apart: a client
that cannot receive fills the host's ring, and a client that receives and cannot step shows a growing backlog
while its ring stays short. The client keeps its sets in a buffer capped at the **backlog cap**, a match setting.
A set that would not fit is not taken, so it is not acknowledged and stays in the host's ring.
[Mechanism 10](#10-slow-silent-and-hostile-peers) says what the host does with each measure.

**Results.** C13 gives every admitted input "a bounded committed/rejected result". The set is that result, and a
true one, because a set is sent only for a tick the host committed
([mechanism 1](#1-one-authority-shared-by-admitted-sets)). For each slot a set names the sequence of the timed
frame it admitted and the last command sequence it admitted, so the client that authored them knows which were
committed at which tick, and that an earlier one that never appeared was refused. The ring is what retains
results until they are acknowledged. At a match's end the match-ended message resolves what is left.

**Old datagrams.** A datagram can arrive long after it was sent
([TRANSPORT](../TRANSPORT/DESIGN.md#1-the-link)). A set for a tick a client holds is ignored, a frame for a tick
already committed is refused as late, and a datagram from another epoch is refused, each by its content.

### 4. Timed frames and ordered commands

**Decision.** A player's intent travels in one of two ways, and the game says which for each action.

| | Timed frame | Command |
|---|---|---|
| Carries | The held axes, and actions that belong to one moment | An action that must happen once |
| Lane | Datagram, in A5's input batch, each datagram repeating recent frames | Reliable, numbered by the client |
| Admitted | For the tick it names, if it is staged before that tick's barrier. Later, it is refused; its action is lost and never invented (C13) | At the first barrier after it arrives, in the client's order, into that slot's frame. It is never refused for being late |
| Lost link | Old frames are discarded | A command not yet answered is uncertain and is not resent (C13). Its effect is in the world or it is not, and the game shows which |

**Why.** A level-up choice sent on a timed frame can be lost. Vampire Survivors' online co-op met this from the
other side: its choice screens arrived out of order after dropped packets, so that "players could see different
UIs at different times", and were moved to ordered commands
([coherence](https://coherence.io/blog/tradecraft/vampire-survivors-online-coop-case-study)). Here both halves of a choice are ordered by construction. The offer is world state, made by a rule at a tick from the
random function, so every peer shows the same offer at the same tick. The answer is a command, admitted at one
tick and sent in that tick's set, so every peer applies it at the same tick. Whether the run pauses while a
player chooses is a rule of the game's: the session's ticks never pause.

**Legality** is the rule's to decide, on every peer alike: a command that makes no sense in the world it meets
changes nothing there. The host checks only what C13 gives N1: the sender, the size, the epoch, the order.

**A frame's limit.** A frame holds at most eight actions (A5). Commands that would overfill a slot's frame wait
for the next tick, in order.

### 5. The input lead

**Decision.** A client stamps each new frame with a tick ahead of the host's, and sizes that lead from what the
host reports. For each client the host notes how many barriers before its own tick the newest frame was staged:
its **margin**, negative when late. The margin rides in every set datagram to that client. The client holds the
low end of its recent margins at one barrier: it raises the lead at once when frames run late and lowers it
slowly when they arrive with room to spare. The first lead comes from the round trip of the handshake.

**Why.** The lead has to cover the path up, its jitter and a lost datagram, and all three change during play. No
clock is compared: the margin is counted in the host's own barriers. This is the shape of current engines' time
synchronisation; Unity's Netcode for Entities has the server return each command's age, "used as feedback … such
that the client always runs ahead of the server"
([`ServerCommandAge`](https://docs.unity3d.com/Packages/com.unity.netcode@1.1/api/Unity.NetCode.NetworkSnapshotAck.ServerCommandAge.html),
[time synchronisation](https://docs.unity3d.com/Packages/com.unity.netcode@1.5/manual/time-synchronization.html)).

**Whose present.** A client never learns the host's clock, and its own starts wherever its process did. It keeps
an **authoring tick** that advances one tick for each tick period of its own monotonic clock. When it starts to
play, the authoring tick is set to the newest tick it holds a set for, plus the handshake's round trip in ticks,
plus the starting lead; the tick named in the welcome is too old to use after a long baseline. From then on only
the margin moves it: raising the lead jumps it forward, and lowering the lead holds it for a tick. A first
estimate that is wrong, or two clocks that run at slightly different rates, reach the client as a margin that is
too small or too large, and are corrected as a change in the link is. The lead a client reports is an estimate:
the authoring tick less the host tick it infers from its newest set and half the round trip.

**Checked.** In the probe the lead settled at 4.6 ticks at A0, 9.6 on the harsh profile and 1.6 on a local link.
After the first minute about one tick a minute was held at A0. When the link worsened mid-run 46 ticks were held
in the next ten seconds and the lead followed; when it recovered the lead came back down. A lead fixed at the old
fixture's five ticks missed every tick on the harsh profile. The probe's host and client shared one virtual time,
so it did not try a wrong first estimate or a clock that runs fast; M2 does, with an origin for each client and
one clock at another rate.

**Limits.** A client authors at most one frame for a tick, and its target ticks only rise: when the lead falls it
skips a tick of authoring, and when the lead rises the ticks it jumps over are held. The lead is capped by the
staging window, a match setting. A client whose link needs more is told so ([mechanism 9](#9-negotiation-and-leaving)),
and its frames arrive late until the link improves.

**Repeats.** Each input datagram repeats recent frames so one loss costs nothing. The probe found two frames
recover nearly all of A0's loss and eight no more, at 31 bytes each, so the depth is a match setting and M2
records the held ticks at each depth.

### 6. The playout clock

**Decision.** A client's playout is one **position** on the tick line: a tick and a fraction, moved forward by
the client's own monotonic clock and never back. The world commits tick `n + 1` when the position reaches tick
`n`. The two newest committed ticks therefore always bracket the position, and a frame draws between them at the
fraction, the **playout interpolation** (C02). Commitment follows the position and not arrival: a set that
arrives early waits in the client's buffer of sets, whose length is [the backlog](#3-the-set-stream).

**The target.** The position aims at a target that stands a buffer behind the sets' arrival, so that the world
steps at an even pace however unevenly datagrams land. The buffer is sized from how late sets have recently
been. The target is placed by when sets arrive and never by a time the host sent.

**The rate.** On its target the position moves one tick in each tick period. Behind its target it moves faster,
up to the **catch-up rate**. Ahead of it, as when the buffer has just grown, it moves slower, down to the
**slow-down rate**. Both are match settings. They start at 1.1 and 0.85, the defaults Unity's Netcode for
Entities ships for the same job
([`ClientTickRate`](https://docs.unity3d.com/Packages/com.unity.netcode@1.5/api/Unity.NetCode.ClientTickRate.html)),
which likewise "slowly adjusts both prediction tick and interpolation delay in small increments" and never
lets either go "back in time"
([time synchronisation](https://docs.unity3d.com/Packages/com.unity.netcode@1.5/manual/time-synchronization.html)).

**A stall.** The position never passes the newest committed tick. When the next set is not held, or an advance
would need more commits than the cap on catch-up, the position holds there and the picture holds still. That is
a **stall**, counted with its length and its cause. When the set comes the position goes on from where it held.

**A skip.** At a tenth over pace a client regains one tick in ten, which is too slow for one far behind. A
playing client whose backlog is above the behind threshold
([mechanism 10](#10-slow-silent-and-hostile-peers)) therefore **skips**: at each advance it commits as many
ticks as the cap on catch-up allows and its position is placed at the newest of them, until it is back on its
target. A repair or a return ends with the position placed on its target, at a tick never behind the one it
left, and is a skip by that cause. A skip is the one jump this design accepts in a client's picture. It is
recorded with its first and last tick and its cause, and the client is shown as behind while it lasts, so a game
can say so on screen. This is C02's "explicit pause/catch-up policy with a work cap". Factorio's client does the same after a
join: it must "fast-forward to catch up" ([Friday Facts 147](https://factorio.com/blog/post/fff-147)).

**What is promised.** Outside a skip, from one frame to the next the position never moves back, and never moves
on by more than the catch-up rate times the time between the two frames. A client's picture therefore moves at
most a tenth faster than the same world would alone, and holds still only in a stall the record names. Sets
that arrive together inside the buffer change nothing a player can see: the position is still behind them and
takes them one tick at a time.

**Checked against the design review's case.** A client has drawn up to tick 100 and stalled; sets 101 to 103
then arrive together, and its target is three ticks on. Committing all three and drawing between 102 and 103
would move a body at least two ticks' travel in one frame, where a steady frame at 120 Hz moves half a tick's.
Here the client commits 101 alone and its next frame is at 100.55. It commits 102 when the position reaches 101
and 103 when it reaches 102, and regains the three ticks in thirty, half a second, at 0.55 ticks a frame. Had
its backlog been above the behind threshold it would have skipped, and said so.

**What the game gets.** Each time it polls the session with the time, it is told how many ticks to commit now,
the position with its rate, and whether a stall or a skip is under way.

**Synchronising and repairing have no playout.** The client steps as fast as it can and nothing is promised of
its picture. Its position is placed on its target when it starts to play.

**The host** steps by the wall clock, with a bounded catch-up after a hitch and no dropped tick (C02), and its
position is the wall clock's, as a world's alone is. The session holds the pacing for a headless host, so
`SWARM`'s host and the proof's are the same loop.

### 7. The world image and the baseline

**Decision.** A client's world always starts from the host's. The host writes its world image at a committed tick
`b`, and sends it down the reliable lane in pieces with the roster at `b`: together, the **baseline**. The client
builds a fresh world from it, then steps the sets from `b + 1` as fast as it can until it reaches the present.
There is one way into a match, and the first player takes it as the fourth does, whatever the tick.

**The world image** is the bytes [the world digest](../SIM-2D/DESIGN.md#4-the-world-digest) is taken over, in the
same encoding, with a way back: the tick; each slot's live flag, generation, stable identity and structural tick;
the free-slot list in its order; each component's presence, value and change tick; the identity index and the row
directory; and the game's authority resources. It leaves out what the digest leaves out. A world restored from an
image has the image's digest, and steps on to the same digests as the world it was written from. It keeps no
undo history, which a match never uses.

**Where it lives.** `thrum-core` can restore a capture only into the runtime that made it. The image is the same
operation through bytes, into a fresh runtime in another process, so it is built in `thrum-core` beside the
digest: each component and resource type gains the decoding half of `SIM-2D`'s state encoding, and the compiler
refuses a rule set whose type has one half without the other. It is built by this module's M1 builder under
[a seam ruling](../ARCHITECTURE.md#seam-rulings-for-session). `SIM-2D`'s retained digests do not move.

**One boundary asks for both halves.** Registering a component under the profile, and naming a rule set's
authority resources, each require the encoding and the decoding together, so any rule set that can be digested
can be sent. The types registered before this module, the crowd's and the fixtures of `thrum-core`'s tests, gain
the decoding half in M1, and that is the one change the seam ruling allows to a test `SIM-2D` was accepted with:
its assertions, inputs and expected values stay. A second registration for types that can be sent, leaving the
old one as it was, was weighed and left out. It would keep those fixtures untouched at the price of two kinds
of rule set, one of which no match can take, and of a refusal that moves from where a type is registered to
wherever a match is first put together.

**An image is checked whole before it replaces anything.** It names the rule-set identity and the tick it was
taken at, its lengths are bounded by the match's settings, and a client that cannot decode all of it keeps the
world it had and asks again. The bytes come from the host, which is the authority, and are still untrusted as
bytes. A refusal leaves the runtime that was offered the image as it was, able to take a good image next, so a
refusal is only ever proved on a runtime that would have accepted the image intact.

**The host never waits.** It writes the image in one pass at a barrier into a buffer the session owns, then goes
on stepping; the pieces leave as the reliable lane has room, and the client's ring starts at `b + 1`. Factorio
joins the same way: the map goes "in the background, while others are playing the game", the joiner is sent "all
the player input" since, and then "tries to update it as fast as possible to catch up"
([Friday Facts 149](https://www.factorio.com/blog/post/fff-149)).

**Range.** An image of the reference crowd at 10,000 bodies is estimated at about 1.3 MiB (130 bytes a body, most
of it identities and change ticks); M1 measures it. `TRANSPORT`'s probe moved 1 MiB down the reliable lane under
A0's profile in 0.7 s with BBR and 18 s with Cubic. Sets cost a few kilobytes a second, so the ring can hold a
minute of them, and a join has a deadline, a match setting, after which the host gives up on it and the client
may try again. A client steps a 10,000-body tick in about 5 ms, so while it catches up it steps about three
ticks in the time the host steps one.

**A slot is reserved at the welcome and joins at a tick.** A reserved slot counts as taken: a hello that arrives
when every slot is held or reserved is refused as full, and a join that misses its deadline gives its slot back.
Several joins can be under way at once, each from an image written at its own barrier. A joiner is **in step**
when its progress report shows it back on the datagram lane with a backlog at or under the behind threshold
([mechanism 10](#10-slow-silent-and-hostile-peers)). The host then puts the roster event in the next set, and from
that tick the slot takes its frames.

### 8. Digests and repair

**Decision.** Every peer takes the world digest at each tick that is a multiple of the match's **digest
interval**. The host's digest travels in the set stream, with that tick's set. A client compares it with its own
when it has committed that tick, before it steps on. A client that differs stops stepping, tells the host on the reliable lane with its
own digest for that tick, and is given a new baseline: a **repair**. Its link, slot and epoch stay, and its
player keeps moving on the host from the input it keeps sending.

**What is promised.** Equal digests at a digest tick mean equal worlds at that tick, to within the chance of a
64-bit collision. The digest covers everything a later tick reads, so two worlds equal there step on equally.
A difference that is still in the world at a digest tick is therefore found at that tick. The digest is taken
over one committed endpoint and not over the ticks between. A difference the rules overwrite with the same value
on both peers before the next digest tick, such as a resource every tick assigns or a held control the next
admitted frame replaces, is gone by then: the two worlds are the same again, nothing is left to repair, and it is
not reported. Whatever that peer drew in between, it drew from a wrong copy.

**How often.** The interval is a match setting from 1 to 600 ticks, and it starts at 30: twice a second. It sets
how soon a lasting difference is found and how long a passing one can go unseen; at 1 every committed endpoint is
compared. A repair takes a second or more whatever the interval. `SIM-2D`'s probe took the digest in 0.18 ms at
10,000 bodies, which twice a second is about 0.04% of a peer's time and every tick about 1%.

**A client whose own step fails** on a set the host committed holds a wrong copy by definition. It stops and asks
for a repair as it does for a differing digest.

**A copy that keeps going wrong.** A client repaired more often than the match allows is closed with that reason.
That is the fate of a client whose rules were changed, and it costs the others nothing.

**What the host records:** the peer, the tick and both digests. A client also sends the digest of each component
([SIM-2D](../SIM-2D/DESIGN.md#4-the-world-digest)), and the host keeps its own for its last few digest ticks, so
a difference in development can be narrowed to a component.

**The host is never repaired.** Its world is the match's by definition.

### 9. Negotiation and leaving

**Decision.** A client speaks first. Its hello names the protocol, the rule-set identity and, when it is
returning, its player token. The host answers with a welcome or a refusal. The welcome carries the match
identity, a new connection epoch, the slot, the token, the match seed and the match settings. Nothing but a hello
is accepted from a link before it is welcomed.

**The rule-set identity** is one value that changes whenever the rules' results could: it covers the rule
package's sources and constants, the engine sources its results depend on (the profile, the tick, the digest and
the queries), the toolchain, and the game's action wire. It is derived from those when the game is built, never
kept by hand, and is the same on every target for one commit. A host refuses a client whose value differs. This
is [the addition to C13](#interfaces) the module list asked this design to propose.

**Everything a tick reads is in the world or in the identity.** The world holds what the digest covers and the
image carries. Whatever else the rules read is the rule set, and the identity names it. So a rule package that
can be set up to compute differently is a different rule set under each setting, and each states its own
identity: the value derived at build time, combined with every setting its rules read that its world does not
hold. The reference crowd is the case in hand. One flag chooses between the rules of its Crowd and Mixed traces,
and the flag cannot join the world without moving `SIM-2D`'s retained digests, so Crowd and Mixed are two rule
sets with two identities, and a runtime set up for one refuses an image written by the other before it steps. A
setting a game wants to change during a match belongs in the world, as an authority resource. A value the rules
read that is in neither place lets two peers accept one image and then differ, which is the fault this rule
exists to stop.

**The protocol identity** covers the envelope and every message this module defines. It is a number in
`thrum-network`, raised by whoever changes the wire.

**Who may join.** Anyone who can reach the host with its
[join ticket](../TRANSPORT/DESIGN.md#3-the-hosts-identity-and-the-join-ticket) and a matching build takes a free
slot. The game may refuse a hello through a hook. There are no passwords and no accounts.

**The token** is a random value the host gives each player. A client that returns with it within the match's
grace period gets its slot back, on a new link, with a new epoch and a new baseline; nothing sent under the old
epoch is admitted (C13). After the grace period the slot is left and free, and the token opens nothing. Its
holder, like a client with no token, joins as a new player, with a join event and no claim on the old character.
It may be given the same slot number, as any free slot may be.

**States a screen can show.** For this peer: connecting, negotiating, refused (with the reason), synchronising
(with how much of the baseline has arrived and how many ticks remain), playing, behind (with its backlog, while
its picture [skips](#6-the-playout-clock)), repairing, link too slow, and closed (with the reason, a match that ended among them). For every slot: free, present or away. With them, the round trip the lead implies,
the lead, the confirmed delay, and the playout's stalls and skips.

**Answers before closes.** A close can cut off the last reliable messages
([TRANSPORT](../TRANSPORT/DESIGN.md#1-the-link)). So a host that refuses sends the refusal and waits, up to a
timeout, for the client to close; and a client that leaves sends its goodbye and waits for the host to close. A
goodbye that arrives frees the slot at once. A link that closes without one puts the slot away, for the grace
period. A host that ends the match sends the match-ended message and waits in the same way.

### 10. Slow, silent and hostile peers

The host steps on its clock whatever any peer does. Every per-peer store has a cap from the match's settings,
and each cap has one outcome.

| The peer | What the host sees | What happens |
|---|---|---|
| Stops acknowledging sets, as on a stalled link | Its ring fills: `held` stands still while the host's tick goes on | Nothing more is kept for it. A repair when it next answers. One repaired more often than the match allows is closed with that reason |
| Takes and acknowledges sets on time and cannot step them at the pace of play | Its ring stays short and the backlog it reports grows | It is shown as behind while its reports show a backlog above the **behind threshold**. When they have shown one for longer than the **catch-up deadline**, with no report at or under the threshold between, it is closed as unable to keep up. It is not repaired: a new baseline does not make a client step faster |
| Stops taking reliable messages | The reliable lane returns `Full` | Nothing is queued for it beyond the lane's cap; a baseline or a command result that cannot be sent by the deadline closes it |
| Sends no input while its link stays open, as a frozen browser page does ([TRANSPORT](../TRANSPORT/DESIGN.md#5-bounds-backpressure-and-liveness)) | No frame staged for its slot | After the held-control expiry its slot is sent neutral axes. After the absence limit it is closed and its slot is away |
| Loses its link | The link's close event | Its slot is away for the grace period, then left |
| Sends bytes that do not decode, a message out of state, input for another slot, or more than its rate | A refusal in the decoder or the session | Nothing is admitted from the message. A peer that breaks the protocol is closed with that reason |

**Lag, measured.** The host's tick less `held` is how far a client's link is behind, and the ring measures it.
`held` less `stepped` is how far its stepping is behind, and only the progress report shows it. The behind
threshold starts at 30 ticks: the probe's playout buffer was about three ticks on the harsh profile, and a stall
or a join leaves a backlog of hundreds. The same threshold is where a playing client stops gliding back to its
target and [skips](#6-the-playout-clock). The catch-up deadline starts at 600 ticks, counted in host ticks from the
first report above the threshold. Both are match settings, and M3 records the backlogs they are set against. The
deadline judges a playing client. One that is synchronising or repairing is judged by the join deadline alone,
and the catch-up deadline first applies when it is in step. A client closed as unable to keep up leaves its slot
away for the grace period and may return with its token. The two ticks are the client's own word: a client that
misreports them hides only its own lag.

Every decoder that reads a peer's bytes gets a property test over valid input cut at every byte, bad lengths and
random bytes, and a stdin target in the style of `thrum-network`'s existing two (A0: "Fuzz untrusted envelope,
codec and replica admission before opening a real session").

### 11. The predicted controller

**Decision.** The crowd and the other players are drawn at the confirmed tick. A player's own character is drawn
from a prediction that runs ahead of it. Whole-world rollback is not built.

**Why not roll the world back.** A client's newest input is ahead of its confirmed world by the lead plus the
confirmed delay: 9 to 10 ticks at A0 and about 20 on the harsh profile. Predicting the whole crowd means
re-stepping that many ticks whenever another player's input turns out different from the guess, which with
analogue axes is most ticks. `SIM-2D`'s probe measured a tick of 10,000 bodies at 5.0 ms on the Mac and 5.6 ms in
Chrome, so one correction is about 50 ms against a frame of 8.3 ms. Photon Quantum, which does roll its whole
state back, warns that "multiple simulation updates are simulated for each entity which can be very CPU
expensive" and offers prediction culling to skip entities outside an area
([manual](https://doc.photonengine.com/quantum/v3/manual/prediction-culling)). Factorio keeps its lockstep world
confirmed and hides latency for the local player alone, by "simulating the player input, without considering the
actions of other players" ([Friday Facts 302](https://factorio.com/blog/post/fff-302)).

**What a game declares.** A **predicted controller**: the part of its own character's state that answers input,
how to read it from the confirmed world for a slot, and one step function from that state, a frame's axes and
actions, and the rule set's constants to the next state. The step function lives in the rule package, under the
profile, and the game's own rule calls it, so prediction and authority are one piece of code
([A5 controller replay](../../design/decisions/A5-CONTROLLER-REPLAY.md)). Its signature is the closure C13 asks
to be complete: it is given nothing of the crowd, so it cannot depend on it.

**How it runs.** The client keeps a ring of its authored frames and the predicted state after each, with axes
stored through [the engine's input conversion](../SIM-2D/DESIGN.md#the-input-conversion). When it commits
confirmed tick `t` it reads the controller from the world and compares it with the ring's entry for `t`, bit for
bit: the rules are deterministic, so no tolerance is needed. Equal, and the prediction stands. Different, and it
restores the confirmed state and replays the frames after `t`. The ring holds 64 ticks (A5); the old fixture's
cap of nine replay steps is replaced by the ring's length, since a step is a few operations.

**When it differs.** The host held a tick because the frame came late; the lead moved; a command changed the
controller; or a rule lets the crowd act on the character. The last is the game's choice and its cost: a rule
that pushes the hero corrects the prediction every tick it applies.

**When it cannot run.** If the unconfirmed span outgrows the ring, or the world is being repaired or rejoined,
the prediction is dropped and the confirmed character is drawn until the client is back in step.

**What the game gets.** The predicted state for the newest authored tick and the one before it, to interpolate
(C02); and each correction, with the state before and after, to smooth as it chooses (C07). The confirmed world
and its pages are never written by prediction.

**What would change the decision.** M4 puts `SIM-2D`'s recorded eight-tick replay beside the frame period. If a
game's crowd is small enough that the replay is a small part of a frame on every target, a later plan can add
whole-world prediction as a client's choice; nothing here prevents it.

### 12. A driven world

The engine's world lane (`thrum-core`'s `world_lane.rs` and `clocked_world.rs`, run by `thrum-app`) advances on
a fixed clock and raises an overload when it falls behind. A host's and a client's world must instead commit the
ticks the session names, and draw at its [playout](#6-the-playout-clock) position. So a world may be **driven**:

- its **driver**, which the game supplies, says at each advance how many ticks to commit (none, one, or several
  up to the cap on catch-up) and where the presentation stands between the last two committed ticks;
- the driver states that place as the lane's clock sample does, a position with its rate at a time, so the
  frame host evaluates it at each frame's own display time, in place of the lane clock's, and holds it at the
  newest committed tick;
- a driven world raises no overload for ticks nobody asked for;
- a world on the fixed clock behaves as it did, and its tests pass unchanged.

The engine side names no session and no link. A game's driver asks its session how many ticks to commit now and
passes the answer on, which is three lines of game code and is in the guide. A driven world draws what it is
told and adds no smoothing of its own. Whether the picture moves evenly is the driver's doing: a session's
driver keeps [the playout's promise](#6-the-playout-clock), and a driver that names three ticks at once moves
the picture three ticks. The session stays driven by time
from the world's task, which drains the link at the barrier as
[`TRANSPORT`'s design](../TRANSPORT/DESIGN.md#4-the-world-lane-never-waits-on-the-network) expects.

This is new work in core's and the app runner's state, built in M7 under
[seam ruling 3](../ARCHITECTURE.md#seam-rulings-for-session). It is proved with the reference crowd drawn by
`RENDER-2D`'s crowd view, on a Mac host, a Mac client and a Chrome page, so no game is needed to prove it. It
is M7 and not part of M5 because M1 to M6 need no window and no GPU, and this needs both. The playout position
itself is session logic and is proved in M2 in virtual time. M7 proves that a frame is drawn where the position
says, and that the result looks as promised.

*Range.* A driver may name no tick for as long as it likes; the world holds its last pair and the picture holds
still. The cap on catch-up is the session's, already bounded by [mechanism 10](#10-slow-silent-and-hostile-peers).
A hidden browser tab stops its driver; when it returns it is the session that decides between a skip and a
repair, and the world only does what it is told. A frame that falls between two advances draws the same pair
of ticks again, further on; no tick is committed twice.

## The system as it runs

| State | Owner | Kind |
|---|---|---|
| The match identity, the roster, each link's epoch and token, the match settings | `thrum-network` (this module) | Session |
| Staged future frames, pending commands, each client's ring of unacknowledged sets, baselines in flight | `thrum-network`, on the host | Session, capped |
| The lead and the authoring tick, the playout position and its target, the buffer of sets, the predicted controller's ring | `thrum-network`, on a client | Session, local, capped |
| The world image's encoding and restore | `thrum-core`, by [seam ruling](../ARCHITECTURE.md#seam-rulings-for-session) | Engine code; no stored state |
| A driven world: which ticks the lane commits and where the frame stands between them | `thrum-core` and `thrum-app`, by seam ruling 3; the driver is the game's | Engine code; the driver holds no state of its own here |
| The tick, committed state, pages, the digest | `thrum-core`, as G2S and `SIM-2D` leave them | Authority |
| A link, its lanes and counters | `thrum-transport` | Runtime |
| The proof programs for the three targets, the scripted players, the match command | `examples/match-proof` and `scripts/` (this module) | Proof tooling |
| Rules, actions, what a roster event means | The game | Not here |

`thrum-network` gains a dependency on `thrum-transport`'s link interface, which builds with no outside crate. It
names no socket, no game type and nothing of the reference crowd. A session is driven by calls that return at
once, and takes the time from its caller, so the in-memory proofs run in virtual time and repeat exactly.

**A host tick, walked.** (1) At the barrier the session drains each link: hellos, input batches, commands,
acknowledgements, goodbyes. Frames are checked and staged for their ticks. (2) For each slot it takes the frame
due, adds waiting commands, and hands it to the game to admit. (3) It writes the tick's set, with roster events.
(4) The game steps and commits; if the step fails the match ends as
[mechanism 1](#1-one-authority-shared-by-admitted-sets) says and the walk stops. (5) On a digest tick the host
takes the digest. (6) It sends each client its datagram of unacknowledged sets, with the digest if one was taken
and that client's margin. (7) A client waiting for a baseline gets one written now. Nothing in the walk waits on a
peer.

**A client tick, walked.** (1) It drains its link: sets go to its buffer, margins to the lead. (2) It authors
this tick's frame at its authoring tick, steps the predicted controller, and sends its datagram with recent frames
and its progress report. (3) It moves the playout position on by the time that has passed. For each tick the position
now calls for whose set is here: admit the set, step, commit, compare the digest if that tick has one, then check
the prediction against the new confirmed state.

**A join.** Link open; hello; welcome with a slot and a token; the host writes an image at `b`; pieces arrive;
the client restores and steps from `b + 1` until its progress report shows it in step; the host puts the join in
the next set; the hero appears on every peer at that tick.

**A drop.** The link closes as timed out; the host puts "away" in the next set; every peer sees the hero go idle
at that tick. The client opens a new link and says hello with its token; from there it is a join into its own
slot, ending in "back".

**A game's journey.** Write rules under the profile and register the rule package. Declare the action wire, which
actions are commands, and optionally a predicted controller. Rehearse a match on the in-memory link in a test.
Start a host with a ticket; hand the ticket out; give each client's session a link.

## Interfaces

Provided to other modules, as [the module list](../ARCHITECTURE.md#interfaces-track-3) names them:

| Interface | To | What it is |
|---|---|---|
| **Session** | `SWARM` | A host and a client driven by time: join and leave, roster events, the admitted set for each tick, how many ticks to commit now and the playout interpolation, with each stall and skip, the predicted controller's two states and its corrections, commands with their results, and the match settings |
| **Connection state** | `SWARM`, `GAME-UI` | Mechanism 9's states and reasons for this peer and for each slot, with the lead, the confirmed delay and the counters |
| **Match proof** | `SWARM` | The scripted-player harness and the command that runs a match on the in-memory link and across the three targets through `TRANSPORT`'s relay |

Used from other modules: the message link, the in-memory link, the join ticket and the relay from `TRANSPORT`; the
profile, the digest, the state encoding, the input conversion, the conformance command and the reference crowd
from `SIM-2D`.

**The addition to C13,** to be written into [CONTRACTS](../../design/CONTRACTS.md) by this module's last
milestone, in these words:

> A game may declare a deterministic rule set under the rules profile. Its gameplay semantic manifest then names
> the rule set and its build, and a peer whose identity differs is refused at negotiation. Such a match shares
> state by the input the host admitted for each tick it committed, compares peers by world digest at the ticks
> the match sets, and repairs a peer with a full baseline. Bit identity across platforms is that rule package's claim, proved by its conformance run; the
> engine makes no such claim for any other game.

Its producers and consumers are this module, `SIM-2D`, core and the game. `SIM-2D`'s design already leaves the
wording to this one.

**No other contract changes.** C13's authority, epochs, results, bounded peers and optional prediction are kept
as written, and the mechanisms above say where each is met. In particular a result is reported committed only
after the host's step has committed, because no set is sent before it. A5's input batch is used byte for byte, so
`SIM-2D`'s wire-route proofs stay valid. A5's state deltas, their acknowledgements and the three `StateGroup`
modes are not used by a deterministic match and stay in `thrum-network` as they are. New message kinds join the
envelope.

**For `SWARM`'s planner.**
- The engine's world lane steps by its own clock. A client's world has to step when the session says, which
  [a driven world](#12-a-driven-world) allows from M7. The game supplies only the driver. After a stall the
  crowd runs up to a tenth fast until it is back on its clock, and a client that fell behind skips; the
  connection state says which, for a screen to show.
- A hero the crowd can push is corrected every tick it is pushed. Keep the hero's movement a function of its own
  state and input where the game allows.
- The crowd is drawn at the confirmed tick and the hero ahead of it, by the lead plus the confirmed delay. M4 and
  M5 record that offset. How it feels is the owner's to judge in the game.
- One match has one crowd. A peer that cannot step it at the pace of play is dropped; the match does not slow
  down for it.
- A setting the rules read, such as a mode or a difficulty, goes in the world as an authority resource or in
  [the rule set's identity](#9-negotiation-and-leaving). Every component and resource type registered under
  the profile supplies the decoding half with the encoding.

## Cost

No budget is set (brief, decision 3). The account, for a host and four players at 60 ticks a second:

| Work | Count | From the probes |
|---|---|---|
| Host, each tick: drain 4 links, stage up to 32 frames, write one set, send 4 datagrams | 60 a second | Not measured; small beside the tick |
| The tick itself, on every peer | 60 a second | 5.0 ms on the Mac, 5.6 ms in Chrome at 10,000 bodies (`SIM-2D`'s probe) |
| The digest, on every peer | Twice a second | 0.18 ms at 10,000 |
| Client to host | One datagram a tick: the 64-byte envelope, one byte, 31 bytes a frame | 7.6 KB/s with two frames, 11.3 KB/s with four |
| Host to each client | One datagram a tick: the envelope and about 7 sets at A0 | About 6.5 KB/s when axes change now and then, about 19 KB/s when four players' axes change every tick. Arithmetic, not measured |
| A join or a repair | One image write on the host, one transfer, one restore, then a catch-up | About 1.3 MiB at 10,000 bodies (estimate); 0.7 s under BBR, 18 s under Cubic (`TRANSPORT`'s probe) |
| A correction of the predicted controller | When the host held a tick: about one a minute at A0 | 9 to 10 steps of a function of a few operations |
| Memory on the host, for each client | A ring of sets, staged frames, at most one baseline in flight | Bounded by the match settings; M3 records it |
| Memory on a client | Its buffer of sets, up to the backlog cap | Bounded by the match settings; M3 records the largest backlog |

The costs that matter are the wait a set adds before a client can step, which the playout buffer turns into
latency, and the tick the host writes an image in. The design sends a set as soon as its tick has committed,
which puts the host's own tick, about 5 ms at 10,000 bodies, in every client's wait; it writes a set once for all
clients, and writes an image in one pass. The session's buffers are reused, and M2 counts what a warmed
host tick and client tick allocate.

**A0's proposed 64 kbit/s** was written for travelling state at 20 Hz. By the arithmetic above the upward stream
alone is 61 to 90 kbit/s, a third to a half of it the envelope, so this tripwire is expected to be flagged from the
first run. The first savings, none built here, are a short header for datagrams and sending every other tick.

**Baselines and tripwires.** Every time, byte count and allocation count this module records is a baseline. A
cost command succeeds when its record is complete and made at the claimed commit, whatever the figures are. Four
things are flagged in the record and investigated while building goes on: a row about a quarter worse than the
last comparable one; bytes to or from a peer above A0's 64 kbit/s; a host tick that runs long on the tick it
writes an image; and a join whose time grows faster than its image.

**One figure decides a result.** After the first 600 ticks that follow a start or a change of link profile, at
least 95% of ticks must admit each player's own frame. The probe held 0.1% to 0.6% of ticks; a lead that does not
adapt holds all of them. The floor catches a collapse and is no target.

## Risks

- **The image misses something a later tick reads.** Early sign: a restored world has the right digest and
  drifts later. M1 steps a restored world through the rest of a retained trace on each target.
- **A convincing fake of N1:** clients that are sent the host's state, or that step in the host's process. The
  sockets record names each peer's process and platform, and the review reads what crosses the link.
- **A convincing fake of a join:** a client that starts with the others. The join is after tick 3,600, and the
  record holds the baseline's tick and size.
- **A convincing fake of a repair:** restarting the client. The record holds one link identifier and epoch
  across the repair.
- **The lead hunts.** Early sign: it changes more than a few times a minute on a steady profile. The record
  counts its changes.
- **A client cannot catch up.** At 10,000 bodies a tick in Chrome takes a third of its period, so a long join
  catches up slowly, and a denser crowd might not at all. Early sign: M5's Chrome join time. The join deadline
  bounds it and the finding goes to `SWARM`'s planner.
- **The host hitches when it writes an image.** Early sign: M3's row for that tick. If it is long, the image is
  written over several ticks from a leased page, which is a new design.
- **A hidden browser tab** is throttled and falls behind. It is closed as unable to keep up or as absent, or
  repaired when it returns, by whichever rule its reports meet first, and it can come back with its token. M5
  records what the host sees.
- **A convincing fake of smooth play:** equal digests and correct interpolation values over a picture that
  jumps, or a frame record written from the session's own log. M2 fails the policy that commits every set held,
  and M7's record is of the position each frame was drawn at.
- **Running a tenth fast shows.** Early sign: M7's inspection after the underrun. The two rates are settings.
- **The rate hunts** on a steady link. Early sign: M7's frame record, which carries the rate of each frame.
- **The lag rule closes a client that would have caught up.** Early sign: M3's record of how long a backlog
  stays above the threshold after a stall and a join, set beside the catch-up deadline. Both are settings.
- **The host's tick in every client's wait.** Early sign: M2's time from a tick's barrier to its set leaving the
  host, beside the confirmed delay.
- **Cubic.** If `TRANSPORT` falls back to it, a join at 10,000 bodies takes tens of seconds. M5 records both.
- **Loopback hides the network,** as [TRANSPORT's risks](../TRANSPORT/DESIGN.md#risks) say. Nothing here is run
  between two machines.

## Proof and builder handoff

- **Two games.** The reference crowd proves scale: it is stepped by four scripted players whose axes come from a
  seeded script, most of them off the wire's 16-bit grid. Its rules have no actions, so commands, roster events
  and refusals are proved with a small second rule set kept with the tests, in integers. Two games on one session
  is also the evidence that the engine holds no game types.
- **Two tiers.** A short match on the in-memory link runs inside `./dev check`, in virtual time. The full sizes,
  10,000 bodies for 36,000 ticks, run in release through **the match command**, one script with the scenario as
  its argument.
- **Targets.** The host is always native on the Mac. In M1 to M6 clients run natively on the Mac, as a plain
  executable in the iPhone 18 Pro simulator through `simctl spawn`, and as Wasm in headless Chrome with no flag
  that touches certificates, QUIC or origins; none of them opens a window or needs the GPU. M7 alone opens
  windows on the Mac and a visible Chrome page, takes the GPU lock, and shows a reviewer motion through T2
  engine capture in `realtime` mode. The simulator proves function, never speed.
- **The sandbox.** Nothing in `./dev check` opens a socket, starts Chrome or calls the simulator: every such
  proof is behind the match command. [The project settings](../../../ORCHESTRATE.md) give Codex full access
  ([owner decision 11](../ARCHITECTURE.md#owner-decisions)), so a builder or a reviewer runs the match command
  as it is, with no request.
- **Smooth play is proved twice.** In M2 the playout position is driven in virtual time through a burst inside
  the buffer, an underrun and a fall behind, beside the policy that commits every set held, which must fail.
  In M7 the same three are made on a real link by cutting the path at the relay for a time, so that the sets
  lost in the cut arrive together in the next datagram. The frame record is checked against
  [the promise](#6-the-playout-clock), and a reviewer watches the three places.
- **Equal digests** are read from each peer's own record of the digests it took, never from one process on
  another's behalf.
- **Costs never stop the build.** A row that trips a tripwire is recorded as measured, with what was found.
- **Stopping.** If peers cannot be held equal, the builder keeps the first differing tick, the component and
  both digests, and ends `BLOCKED`. If an image cannot carry something the digest covers without changing
  `SIM-2D`'s retained digests, it stops the same way.
- **Exact baselines.** This module edits `crates/` and `scripts/`, which are covered paths, and moves no retained
  image and no retained digest. Each milestone carries the verifier.

### Simulator lifecycle in the proof harnesses (M5)

Use one repository-owned lifecycle for the SESSION socket and image proofs, SIM-2D conformance, the V1
shell route used by HOST, APPLE-PRESENTATION and PLATFORM-RUNTIME-IOS, HOST overload, and the TRANSPORT,
CULLING, COOKED-DRAW and T2-CAPTURE simulator proofs. Preserve each proof's cases, assertions and evidence
scope. Each turn uses the exclusive `simulator` lock from device selection through verified shutdown of one
Thrum-owned iPhone 18 Pro on the configured runtime.
Direct commands take that lock around device work. Reuse HOST's check that the active lock holder is an
ancestor of the calling process to recognise an outer `simulator` admission. A caller holding `gpu` gets an
error before simulator work. Nested operations share their lifecycle owner's selected device and admission.
For Cooked Draw and T2 capture, the waiting supervisor owns admission and cleanup across the LaunchServices
handoff and passes its selected UDID to the attributed launcher and driver. Keep the existing permission
preflights and their launcher-to-driver attribution.

The outer check declarations for [HOST M4.5](../HOST/DONE.json),
[APPLE-PRESENTATION D6.7](../APPLE-PRESENTATION/DONE.json), [CULLING M2.2](../CULLING/DONE.json),
[COOKED-DRAW M1.7–M1.9](../COOKED-DRAW/DONE.json), [PLATFORM-RUNTIME-IOS M7.2](../PLATFORM-RUNTIME-IOS/DONE.json)
and [the T2 capture guide's command](../../CAPTURE.md#ios-simulator-submitted-output-workflow) already take
`simulator`. M5 adapts those entry points to reuse that admission through verified shutdown. Update the Culling,
Cooked Draw, V1 route and T2 script usage text to match, and update the capture guide when reuse lands.
Acquire one resource per turn and build outside it where the entry point controls admission. When an item or
the published command holds outer admission during preparation, count that preparation toward its hold limit.

A live SESSION peer restart replaces the peer process within the same simulator boot. The HOST route and
overload leg also share one boot, as do each V1 route's lifecycle and rotation steps. Invoke the multi-target
HOST command without an outer lock so its target legs can take and release their own admission in sequence.

Each independent case in `python3 scripts/session_match.py image --target ios-sim` and SIM-2D conformance
takes a separate simulator turn. Call these commands without an outer lock so the harness can release
admission between cases. For each of Crowd and Mixed, the image command takes one turn for the simulator
writer and one for each fresh-process restore at ticks 0, 18,000 and 30,000. Build the binaries and prepare
the Mac source images outside those eight turns. Keep the images and digest records on the host filesystem
across boots to check all six images byte for byte and all 1,000 continuation checkpoints against the
accepted references. SIM-2D likewise runs its vector case and each independent trace or wire case through
shutdown, then compares their results with its existing references. Budget each turn from admission,
including boot, with a worker deadline early enough to stop child work and verify shutdown before the
earlier of the outer check timeout and the project's 30-minute hold limit.

Pass the selected UDID explicitly to `simctl` and XCTest and confirm ownership before accepting a caller's
UDID. Keep the dedicated device shut down between turns and reuse it on the next turn. For the V1 route,
Culling, Cooked Draw and T2 capture proofs, a fresh start means a fresh app installation and data container,
a fresh proof output directory and a ready device with any prior app and XCTest runner state cleared.
Re-establish that state at the start of each proof, including after cancellation. Checks that span rotation,
backgrounding or recovery preserve the app state those steps depend on within the turn. Preserve unrelated
simulators and their data.

The outer orchestrate lock holder waits for the process it launched. Run supervision in that process or use
`exec` to hand its PID to the supervisor before touching the device. The HOST M4.5 shell checks for one
`SAMPLE_SLOTS` definition across `crates` and `examples`, then execs the iOS-only HOST command. That command
supervises its route and overload workers in process. APPLE-PRESENTATION D6.7 execs the presentation command,
which also supervises its iOS route in process. Treat the existing 1,500-second presentation route limit as a
budget that reserves time for cleanup. Expiry stops the worker and goes through the same verified shutdown path.
Apply this to the presentation extent, transition and EDR routes and keep their existing assertions and reports.
Any shell front end that the lock holder waits on must hand its PID over before device work. The Cooked Draw
and T2 waiting processes continue to supervise across LaunchServices.

Install the supervisor's signal handlers before changing device state. On the first termination signal from
the outer runner, it stops the worker and any launcher or driver it owns, requests shutdown, and confirms
`Shutdown` from fresh CoreSimulator state before exiting. Bound this cancellation path to fit inside the
runner's ten seconds from SIGTERM to SIGKILL, including child termination and status queries. Reserve time
for normal deadline cleanup within the turn's budget. Report a cleanup failure with the UDID and last observed
state if verification fails. A successful run requires verified shutdown. If the runner forces termination
or shutdown remains unverified, the next holder can obtain the lock, but it must verify shutdown of the stranded
Thrum device before any new device work. Abort the turn if recovery remains unverified.

Keep simulator work out of `./dev check`. The fast guard exercises ownership and failure handling with
simulated state. The real device proof runs supervision in process and queries CoreSimulator independently
under admission. A separate CPU record uses the installed runner's `install_stop` and `run_check`, as a
build does, with a second `simulator` contender in an isolated lock namespace. Run the HOST M4.5 and
APPLE-PRESENTATION D6.7 commands as declared, using the production wrappers and supervisor with simulated
device and route workers. Queue the contender while the first turn owns the lock. With the worker active,
signal the lock holder to cancel each outer check. Also exercise the presentation deadline with an injected
clock. Keep simulated shutdown pending briefly within the runner's grace and check the device state as the
contender's first action under admission. The contender must see verified shutdown and the first check must
report cancellation or timeout. The ordering check must reject a control in which a waiting shell exits
while its supervisor is still cleaning up.

## Open questions

- Whether a 10,000-body image and its catch-up fit a join a player will wait for. Settled by M1's sizes and M5's
  join times.
- Whether the envelope should be shortened for datagrams. Settled when M5's bytes are on record; a planner
  decides then.
- Whether a tenth over pace, and a skip beyond the behind threshold, look right. Recorded by M7's inspection;
  both are match settings, and the owner judges them in `SWARM`.
- Whether the confirmed crowd's delay behind the hero feels right. Recorded here; judged by the owner in
  `SWARM`.

## World-image compatibility

Under [seam ruling 1](../ARCHITECTURE.md#seam-rulings-for-session), a SIM-2D test accepted before SESSION M1
changes only to decode its types and follow the encoding interface, keeping its assertions, inputs and
expected values. The reference crowd’s `control` field injects faults for conformance and stays out of
[the rule-set identity](#9-negotiation-and-leaving).

## Builder rules

- If peers cannot be held equal, keep the first differing tick, the component and both digests, and end `BLOCKED`. Do not compare fewer ticks, send state to a client, or drop a target.
- `SIM-2D`'s retained digests do not move. If an image or the predictor's function cannot be built without changing one, write the proposal in the log and end `BLOCKED needs_dependency`.
- Leave `thrum-transport`, `thrum-sim2d` and A5's input-batch bytes as they are. A change one of them needs is a proposal in the log and `BLOCKED needs_dependency`.
- The V1 game stays outside all of this. Leave its rules, tests and exact comparisons as they are.
- Nothing that opens a socket, starts Chrome or calls the simulator runs under `cargo test`'s defaults or in `./dev check`. Those proofs go through the match command. If a launch fails, leave the item open and say so; do not substitute the in-memory link, Node or another target's result.
- Match time, byte count and allocation figures are performance tripwires (brief, decision 3). Record the
  measured result and the investigation, then keep building. The match still has to meet the 95% floor on
  ticks that admit a player's own frame.
- Record only runs of the commit being claimed, with the load on the machine.
- Follow the [shared exact-baseline proof on main](../ARCHITECTURE.md#proofs-run-once-on-main).

## Reliable-lane recovery fixture (M8)

Use the in-memory transport with a small roster, queue and world and step simulated time to the same
boundaries. Still drive the reliable lane to backpressure, drain and acknowledge on the same link, advance
beyond the old send deadline, and compare terminal authority and replica digests. Preserve the sibling
persistent-full deadline test if it shares the fixture.
