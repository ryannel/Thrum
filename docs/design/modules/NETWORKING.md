# Network sessions and replication — N1

**V1 direction:** [the A0 transport/codec decision](../decisions/A0-N1-TRANSPORT-CODEC.md)
selects QUIC/WebTransport and acknowledged state deltas; [the A5 timing decision](../decisions/A5-INPUT-TIMING-RECONCILIATION.md)
selects the first conditioned input fixture. [NETWORK-SPEC.md](../NETWORK-SPEC.md) is retained as
the historical I2n authority/result analysis; its WebSocket and full-record recipe is superseded.
The final V1 byte/session specification and runtime qualification remain open.

**Historical design closure:** the [initial protocol](../NETWORK-SPEC.md) resolves the
[SR01–SR03/SR07 review](../SIMPLIFICATION-REVIEW.md) with committed per-action results, atomic full groups,
one gameplay semantic identity and an analytical R4 workload. The
[reviewed provisional cadence study](../research/N1-OUTBOUND-CADENCE-2026-11-03.md)
reopens the 60-to-20 Hz outbound burst and same-tick multi-group replica
consumer before a production queue/codec can qualify. Transport, runtime,
latency and device proof remain unrun. The authority model and early I2n
milestone stand.
The [first R4 payload codec](../../evidence/integration/N1-R4-PAYLOAD-CODEC-2026-11-03.md)
now checks fixed framing and local result shape in a registered crate; it
does not validate the envelope or admit a semantic group to a replica.

Selected architectural direction, 1 November 2026. The user requires future multiplayer as a first-class citizen to
avoid rebuilding the engine around it later. N1 is an optional runtime responsibility with contracts designed now;
the first runnable slice remains local. No network implementation, service, device qualification or latency claim is
made. This module extends the custom engine's existing world/time/content owners; it does not introduce another game
state authority or require an online service for single-player games.

## Model and alternatives

Choose **server-authoritative simulation, explicit replicated state and tick-associated player intents**. Standalone
play uses the same local authority role and game rules with networking disabled. A listen host combines a server world
and local client presentation; a dedicated host runs the authoritative world headless. Each world/session has its own
clock and scope. Replica worlds consume server results; optional prediction is a separately bounded speculative branch.

| Alternative | Decision and reason |
| --- | --- |
| Authoritative snapshots + inputs | Selected: accommodates heterogeneous clients, existing physics/environment providers and headless authority without cross-platform bitwise determinism |
| Deterministic lockstep / universal rollback | Not the baseline: requires a stronger deterministic and restorable simulation envelope than our arbitrary selected libraries/models promise; a game may later supply a qualified specialist protocol |
| Peer authority for arbitrary world fields | Not admitted initially: ambiguous ownership and conflict resolution would undermine the single-authority contract |
| Replicate the renderer or serialize the entire ECS | Rejected: render histories, GPU handles and storage layout are neither gameplay wire semantics nor stable identifiers |
| Add transport after local-only gameplay APIs harden | Rejected: commands, authority, identities, events and content compatibility must already support separate peers |

Multiplayer is a capability a game selects. It does not make every component replicated or every action a network RPC.
Single-player code does not pay for sockets, replicated-field scans, prediction rings or network snapshots when N1 is
off. A game advertised as network-capable declares its commands, replicated fields and side effects through the public
contracts; arbitrary local-only rules do not become safe multiplayer rules automatically.

## Authority and owner boundaries

| Owner | Network obligation |
| --- | --- |
| F1 | Schedules command admission and coherent correction/publication barriers per world; bounds work and clocks |
| F3/W1 | Stable authored IDs and scoped runtime identities; schema-checked state projections; one authority per committed world |
| N1 | Peer/session lifecycle, validated envelopes, ownership routing, replication interests/baselines, bounded transport queues and protocol state |
| P1/P6 | Typed actor actions and game authorization/validation; only the authoritative rule execution commits gameplay consequences |
| P3/P4/E1 | Authoritative physical/animation/environment state; explicit capture/restore support for any opted-in prediction subset |
| F4/W2 | Compatible gameplay content and ready collision/navigation; local visual products and per-peer presentation residency remain independent |
| P7 | Server/standalone authoritative saves and replay; private developer test saves remain separate; no client overwrite of server saves |
| V2/V3/V4/V5/E1/A1/U1 | Local presentation of confirmed/interpolated or explicitly predicted samples; deduplicated effects and history resets on correction |
| T1/T2 | Isolated multi-world play, exact build/protocol/content provenance, network fault fixtures and resource evidence |

The client may request an action, not send an arbitrary component mutation or invoke editor operations. N1 validates
schema, size, epoch, sequence, sender ownership and admission rate; P6 validates game legality against current server
state. A locally controlled actor is not client authority over its inventory, contacts or damage. The listen host's
local player passes through the same command-validation boundary; it does not acquire a second privileged rule path.

## Identity, connection and content

A shared `MatchId` identifies the network match; each peer also has a distinct local `SessionId` and `WorldEpoch`.
A match identifies the exact wire protocol manifest and gameplay semantic manifest, including rule/action meanings,
authority/tick policy, required gameplay query/physical meanings and role-specific product mappings. The server pins
its authoritative gameplay BuildRevision/content for the match. Source-sync or recook cannot mutate
it: restart or an explicit coordinated migration/re-handshake is required. Client platform/render BuildRevisions may
differ while satisfying the same semantic gameplay/protocol manifest. A connection has an authenticated or explicitly game-authorized player identity and fresh connection epoch.
A wire entity uses `(MatchId, monotonically allocated entity ID)` with no reuse within that session; exhaustion
starts a new session rather than wrapping. W1 maps it to local generation-checked handles. Durable save IDs remain
separate. Reconnect rebuilds the client-local mapping from a baseline; existing server wire IDs persist within MatchId.
Loading a saved authoritative world starts a new MatchId unless an explicit coordinated migration says otherwise.

Use `disconnected → negotiating → synchronizing → active → resynchronizing/draining → disconnected`. During negotiation,
reject incompatible protocol or gameplay semantics before accepting player commands. Different local
render profiles, shaders, textures and GPU caches are allowed; differences in required collision/rules/physical data
need an explicitly compatible gameplay manifest. Local visual/tool schemas are outside its hash; a new rule or required
collider is inside it. A seed alone does not establish equal solver state or content. E1 compatibility includes physical model/schema, domain/edge topology, forcing and query semantics. The match records
exact server physical product hashes as provenance. Peer admission names the required consumer capabilities and their
compatible product/projection versions against that semantic manifest; it does not demand server solver bytes from
a nonpredicting client receiving query projections. Locally evaluated/predicted queries require the declared compatible
client product closure and exact hashes, with an explicit mapping to the common physical source/semantics. Visual
material identity alone is insufficient; compatibility is a declared validated relation, not a guessed similarity.

For join-in-progress, send a bounded full baseline at server tick n with schema/content and interest revision. Buffer
only the bounded subsequent updates needed to reach an admitted state; overflow restarts synchronization from a newer
baseline. Activate the replica at a barrier only when its required state/content closure is ready. Optional presentation
may use a declared proxy. A missing gameplay representation cannot silently become a no-collision world. Unavailable
boot/gameplay content leaves the peer loading or rejects entry; the server does not stop its world for one slow client.

Replica synchronization and local presentation readiness are separate gates. V1 prepares that client's required
pipelines, using only declared ready fallbacks while loading. It cannot block server ticks or other peers.

Reconnect creates a new connection epoch and full resynchronization. Prior inputs/acks cannot be replayed into it.
Player reassociation is server game policy after authorization. Host migration, shared-authority worlds, matchmaking,
accounts, relay/NAT services and commercial hosting are separate later integrations, not hidden prerequisites for
local play or a direct-connect session. Initial multiplayer does not promise seamless host migration.

On mobile suspend the client stops input/acknowledgements and treats the socket as lost, while server authority
continues. Resume reauthorizes, takes a new epoch and full baseline, and discards uncertain old-epoch intents;
background catch-up is not part of the first profile.

## Commands, clocks and state publication

A command envelope carries connection epoch, controlled actor, action schema, contiguous input sequence, tick hint
and bounded payload. The client estimates server time; the server checks a bounded age/window rather than trusting
client wall time. Timed controls use intended ticks; board/inventory actions use the observed server tick for age and
execute on the next authority tick in sequence order, subject to P6 state/precondition checks. F1/P1 admit validated
commands at the authoritative pre-tick boundary. Per-actor ordering, missing-input behavior and late action rejection
are declared by the game action type. Held controls
have a bounded expiry and become neutral/cancelled on disconnect; one-shot purchases/fire/interactions are not invented
by reusing the previous held input. Already applied sequences are deduplicated within a bounded connection window.
Too-old commands fail explicitly; no forever-valid exactly-once claim or unbounded retry history is made. An
application-message acknowledgement is transport disposition only. P6 emits one committed/rejected result per
valid sequenced action, and N1 binds its contiguous resolved-input watermark and any individually resolved higher
sequences to the controller's exact committed tick/state in a StateGroup. Prediction discards each named resolved input
after that atomic admission and replays only still-pending future inputs. Reconnect discards old-epoch
pending intents and takes a fresh baseline; uncertain one-shots are not automatically retried. P7/game operation IDs
and outcome lookup provide durable retry semantics when a game needs them, after the outcome is committed with the
authoritative recovery root.

Authoritative simulation runs at its selected fixed tick. Replication cadence may be lower and does not change that
tick or physical quality. After commit, N1 briefly leases a purpose-specific immutable world snapshot to copy a
bounded, N1-owned replication projection, then releases the W1 lease before remote send or acknowledgement. It does
not read a mutable solver or serialize a render packet. Only N1-owned projection, result and event bytes may wait for
a remote peer. Coalesce outbound state before submission and resynchronize/disconnect when declared age/queue limits
are exceeded; remote ACK and queue age never govern W1 snapshot release.

Each replicated schema declares fields, visibility/ownership audience, units, quantization/error bounds and dependency
groups. Dependent fields in a group are from one committed revision; partial updates must not form an impossible pose,
attachment or gameplay transaction. Independent entities may update at different rates only when their consumers accept
that sample age. Authoritative game rules never run on an arbitrary partially received replica state.

The V1 recipe uses acknowledged-baseline deltas; the historical I2n recipe used bounded full
StateGroups without delta history. Each admitted group atomically carries
spawn/re-entry with required first pose, ownership and dependent state, or detach/despawn with relationship changes.
An oversized dependency closure fails profile admission. Re-entering interest establishes a full state and a new
visibility generation; stale old-generation groups reject. Leaving interest is not authoritative deletion. Server
interest controls bandwidth and information disclosure;
it is distinct from that client's W2 streaming cells and camera-frustum culling. Required dependency groups travel or
activate together. Unknown required schemas fail; unsupported optional fields follow the declared protocol policy.

Optional water queries replicate a coherent group: body pose and matching sample tick, domain generation, required
physical projection, edge/source revisions, boundary data and validity. A retained exact pose may be referenced, but
missing correspondence rejects the group. A partial group is unavailable, never dry/empty. Server solver support persists
independently of peer interest. View-only surface residuals stay within contact/shoreline error limits against the
physical query, including underwater occupancy; client pixels do not supply server forces or saved state.

Gameplay-significant visual cues constrain client quality: F2 admits only tiers meeting the game’s cue/error envelope.
Server information disclosure never relies on fog/cloud pixels hiding an already replicated secret. Purely cosmetic
media approximations may differ; authoritative visibility and permitted replicated data remain server policy.

## Prediction is opt-in and bounded

Baseline remote presentation interpolates timestamped authoritative samples with a bounded delay. Extrapolation has
an explicit time/error envelope; beyond it show a stale/reconnecting state or hold a declared pose, never manufacture
unbounded authoritative movement. Local camera/UI response can remain immediate without moving authoritative objects.

A latency-sensitive game can opt a local controller/interaction subset into prediction only after declaring complete
capture/restore and replay inputs: state, random stream, tick, relevant collision/physical content revision, dependent
objects and side-effect policy. The speculative branch has its own world/epoch and authoritative sample origin. Its
results may drive labeled local presentation; they do not overwrite the confirmed replica or server world.

On a matching server snapshot, reset that branch to the confirmed tick/state, discard acknowledged inputs and replay
unacknowledged intents within a bounded tick/byte/work budget. If a required input, dependency state or content version
is missing, or replay exceeds the budget, discard prediction and resynchronize from confirmed state. Do not approximate
a dependent body/fluid interaction by rewinding only one entity and claim it is the same simulation. Arbitrary full
physics/world rollback is not promised. Quantization and platform differences cause corrections; fixed dt does not
prove cross-platform determinism. Rendering may smooth a correction under a declared policy while gameplay state is
already corrected; teleport/topology/content changes invalidate affected V2/V3/V4/V5/E1 histories under C07.
Water-contacting bodies default to confirmed interpolation; enabling prediction requires the complete P3/E1 domain,
boundary forcing, dependent-body state and once-only disturbance closure. A visual sea mesh or seed is insufficient.

Confirmed effects use authoritative event identity. A permitted speculative cosmetic effect uses a stable
`(connection epoch, input sequence, effect kind, ordinal)` key, echoed on authoritative confirmation where applicable.
Replaying prediction cannot repeat it; confirmation reconciles/suppresses the duplicate. Irreversible gameplay or
external effects execute only on authority after validation. An already played speculative sound cannot be undone:
the game chooses which transient cues tolerate that risk. Save, achievement and payment operations are not speculative.

## Transport and overload

N1 depends on message capabilities, not platform sockets in gameplay code: reliable ordered control, bounded command
submission with semantic acknowledgement, and replaceable timestamped state. Adapters expose actual capabilities,
message limits and backpressure. V1 selects native QUIC and browser WebTransport where qualified,
with an ordered WebSocket fallback. Browser availability and interoperability are device gates;
the world API assumes neither raw browser UDP nor a universally available WebTransport service.

Use maintained transport/cryptographic libraries and platform integration; do not write custom cryptography. Treat
incoming data as untrusted regardless of transport. Authentication/service integration stays behind explicit adapters;
the game selects its access policy. Parsing, allocations, command rates, replication work, input/prediction history,
queued bytes and synchronization duration have per-peer and whole-session caps. Coalesce unsent replaceable state;
never drop a required control transaction and continue as if it arrived. If backlog cannot meet the profile's limits,
resynchronize or disconnect that peer. Transport congestion/backpressure behavior must be qualified per adapter.

## Initial design proofs and construction boundaries

The first local slice already uses actor-directed typed commands, world/session scopes and authoritative side effects;
networking-off has no active replication work. I1/I2 identity, save and publication APIs must preserve these distinctions.
An early N1 in-memory two-peer fixture follows the selected source/world contracts before claiming network-ready APIs:
local versus remote command routing, duplicate/out-of-order intent rejection, coherent baseline admission, interest
leave/re-enter, stale connection rejection, missing content and two independent editor-play sessions. This is later
implementation work, not a networking build commissioned by this document.

Before shipping multiplayer, prove transport versions of that fixture with loss, delay, jitter, disconnect/reconnect,
join-in-progress, slow content, bounded memory and server overload. Then qualify any prediction provider under correction,
missing dependency and replay-budget exhaustion. Test presentation-event deduplication and ensure client/save/apply-back
operations cannot change server authority. Specify players/entities/update rate and bandwidth/latency budgets for a
named game workload before performance claims; no universal player-count promise follows from N1's existence.

## Evidence and limits

Primary sources inspected 1 November 2026:

- [Unreal networking overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/networking-overview-for-unreal-engine):
  server authority and early multiplayer-aware gameplay design are production precedents. We do not adopt its actor/RPC framework.
- [Godot high-level multiplayer](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html):
  separates peer/transport capabilities and documents browser networking differences. This supports explicit transport
  profiles, not a claim that Godot's layer or any current browser transport qualifies our implementation.
- [Unity Netcode prediction details, 1.3 documentation](https://docs.unity.cn/Packages/com.unity.netcode%401.3/manual/prediction-details.html):
  selective rollback and interacting partial snapshots illustrate why prediction needs complete dependencies and bounded
  history. This is a cited versioned precedent, not the selection of Unity's algorithm or a current version claim.

The selected N1 architecture and bounded failure behavior are our synthesis. NETWORK-SPEC now fixes the wire encoding, initial access/transport adapters, bounded prediction state and workload
limits. Implementation and actual platform/latency/resource proof remain unrun. Future adapters cannot weaken the
authority/identity/readiness rules above.
