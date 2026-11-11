# M3 MF1 Mac window rerun

This settles the missing D3.5 observation in [MF1](../../../APPLE-TILE/M3-review.md) on the M3 Max.
The executable is the exact reviewer build of `e2129272a95335ae94aeb680668dc553c60b61d4`, SHA-256
`e2eef29ba2a10f572088e56ad9ea3fbf8a4e6b76f7e0925cd22de9dbb68d32f0`:
[provenance](provenance.json), [source audit](source-audit.json). No engine/game code or grants changed.
The initial read-only [preflight](preflight.json) and continuous [session observations](session.jsonl)
record an unlocked console and the existing Screen Recording grant. All target work held the shared GPU lock.
[Observer tool/version](observer-tool.json) pins macOS and screencapture; [reproduction](reproduction/window.py)
retains the protocol. [Inspection plan](inspect-pending.md) predates viewing these captures.

For both `portable` and `apple-tile`, each recipe folder retains `observer.json`, Sun/night/noon OS PNGs and
`control-response.mov`. The observer records PID/window ID, global point bounds, backing scale 2,
1920×1080 content within a 1920×1144 image (64-pixel title bar), monotonic/Unix clocks, recipe/build and
paused controls. `references/<recipe>-<state>/` retains fresh same-build T2 still PNGs, manifests and identities.
These are matching settled states and separate submissions; compositor bytes are not an exact-renderer oracle.
Full frames retain the Sun disk/horizon and terrain, stars/night silhouette, mirror, cubes, rock, local lights and
shadows. The inherited saturated night light matches its reference.

The `interactive/` OS captures and metadata retain ordinary `E` pan, `Q` reversal, CUA bottom-right resize
`[1916,1138]→[1516,938]` and restoration under each recipe. The same PID and binary survive each resize.
Content changes from 1920×1080 to 1520×880, fitting a 1520×855 scene with 12/13 black rows, then returns intact.
[Geometry observations](geometry.json) and [input record](cua-actions.json) provide content-pixel attribution.
Original-time excerpts are `interactive/portable.mov` and `interactive/apple-tile.mov`, covering every
keyboard/resize/restoration action. [Media provenance](media.json) verifies every retained video packet and
its PTS against the original OS prefix. Full 75-second originals remain in scratch through review; the
complete movies are also sampled in [inspection strips](strip-index.json).

Each `<recipe>-recovery/` retains the original four-second OS movie, clock anchor, lifecycle/control logs,
observer attribution, timeline and post-pan frame. Generation 2→3 occurs about 2.1 seconds into the movie;
accepted camera pan follows about 3.1 seconds into it. The recovered scene stays intact and responds to input.
This proves functional compositor output and recovery. It makes no panel-cadence, display-swap or photon claim,
and neither depends on nor qualifies Mac M2's separate windowed-tap correspondence.

[Playback/visual review](playback-review.json) records actual observation coverage. The
[required milestone check](check.json) passes the named moving command and full repository gate.
[Hashes](sha256.json) cover retained artifacts. Two T2 disk-admission refusals
preceded the completed references; no capture setting or guard changed. The first interrupted interactive
recorder discarded its movie, so only the naturally completed rerun movies are retained. A verified
[cache reduction](cache-cleanup.json) removed regenerated Chrome PNG caches while preserving compressed lossless
sources and selected evidence. Scratch masters and the exact executable remain in `tmp/g3t-m3-mf1` through review;
no remote backup is claimed.
