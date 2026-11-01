# N1 network reconciliation handoff

Owned edits: `notes/engine-rebuild/NETWORK-SPEC.md` and `modules/NETWORKING.md`. The first I2n protocol now has one
committed/rejected result per parseable sequenced action, with exact result IDs and a contiguous resolved watermark
atomically paired to committed controller state. Future timed input may leave a gap while a later nextTick action
resolves; prediction removes every individually resolved input and replays only unresolved future inputs. Reconnect
discards old-epoch pending actions; a game offering durable retry commits its operation ID/outcome with P7's root.

Live publication uses full bounded StateGroups. Each group includes lifecycle and required first/dependent state at
one barrier, with no separate LifecycleBatch or delta history. Optional water groups include matching body pose and
sample tick. Protocol identity covers envelope/wire schemas; the single gameplay semantic manifest covers rules,
action/query/physical meanings and accepted role-product mappings while visual/tool builds may differ. Direct-connect
invite, content-ready, baseline, ack, mobile suspend/resume and static-preloaded first server policy are explicit.

R4 proof anchor: `NETWORK-SPEC.md#r4-encoded-admission-fixture`. Four 32-entity groups at 20 Hz plus max action
results, declared events and clock cost 156,248 B/s per peer (<262,144 B/s). A one-chunk join baseline is 9,932 B;
a maximum same-epoch baseline with 256 results and 128 events is 23,244 B. Two maximum baselines with throttled
state and declared bursts cost 60,208 B in a quarter second (<65,536 B). The largest group fixture is 7,008 B
(<8 KiB). Charging every prompt action its own InputBatch, client input using R4 payload caps costs at most
15,448 B/s and 5,976 B per quarter-second burst. These
are encoded arithmetic, not measured latency, transport memory or device behavior.

Cross-owner reconciliation requested of coordinator: C13 must distinguish receipt from result, preserve atomic
groups and exact result-state correspondence including above-watermark IDs; P6 supplies authoritative disposition,
stable event origin and current-state preconditions; P7 owns crash-durable operation outcomes; F3/F4 own the shared
semantic compatibility mapping and role-specific product hashes; F1/W1 supply a coherent controller projection and
publication barrier; E1/P3 pair optional water samples/pose; A1/V5 deduplicate event presentation. BASELINE and
CONSTRUCTION should link the R4 fixture and retain early real I2n, with prediction/E1 projection optional.

Limits: R4 uses one controlled actor and a declared 56-byte actor/40-byte event schema; a game with larger
dependencies, event bursts or payloads needs its own admission proof. 50/150 ms RTT with configured timed leads are
candidate envelopes; 300 ms RTT/100 ms jitter is degraded and may reject stale actions. Real WSS, mobile suspend,
resource/timing and game-feel qualification remain unrun. First server preloads finite authoritative content; future
streaming remains first-class through a separate staged commit while authority continues.
