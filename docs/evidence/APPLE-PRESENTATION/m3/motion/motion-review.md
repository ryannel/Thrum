# M3 Mac motion inspection limits

14 November 2026, source `8fa5592d`, Apple M3 Max / Metal, portable recipe,
1920×1080 real-time submitted engine capture. The ten arms in `mac/campaign.json`
all complete, with the TAAU GPU pass observed only in the enabled arms. The binary
digest is recorded there. Separate elapsed-time arms have different submission
times; equal filenames do not mean equal world state.

Inspection list: trailing geometry and translating highlights during the pan;
the ground and small blocks as the moving cube reveals them; the first accepted
tick-60 spawn; individual bright and dim stars throughout the held camera;
sky/horizon brightness as solar hours change from 6 to 18. Compare enabled and
disabled frames, preserve native pixels in crops, and distinguish source omissions
from playback decoder drops.

The builder inspected ordered whole-frame samples spanning all five pairs,
native-pixel pan/spawn crops near 2 seconds, twelve consecutive reveal source
frames near 2.4 seconds, four all-frame star atlases, and both all-frame scrub sky
atlases. The spawn appears opaque on the first submitted tick-60 frame; the reveal
samples do not retain a displaced blue silhouette. Pan silhouettes are smoother
with TAAU and the inspected light highlights have no second bright patch. No
visible star core disappears in the inspected atlases. The scrub returns to the
evening state without retaining the blue midday sky in the sampled frames.
These observations do not establish the absence of every short trail or crawl.

All 481 original frames per held-star arm were independently decoded with their
writer RGBA hashes checked. Four selected 7×7 star-core integrated flux traces
have relative standard deviations 0.000416, 0.000607, 0.001559 and 0.000449 in the
TAAU arm; the off arm is constant in those regions. These are diagnostics with
no invented acceptance threshold. They do not measure every star or perception.

The scrub audit independently decodes every original source frame and samples
the sky. Fitting the separate on/off RGB traces over the changing interval gives
zero milliseconds as the minimum-error lag, with RGB RMSE 0.138688 code values.
Control scheduling, submission timing and jitter limit this comparison. It is
supporting evidence, not a substitute for a visual brightness-lag judgment.

All ten movies completed a separately recorded 1× CPU software decode with every
movie frame delivered; their frame counts and movie hashes are in the playback
record. The earlier browser playback also completed but dropped frames. Neither
decoder execution nor these static image inspections is direct continuous 1×
visual observation. The builder therefore leaves D3.3 open: its instruction to
watch each pair at 1× and say that motion was watched has not been fulfilled.
No such observation is claimed for Chrome either.

The initial archive identities are listed in `mac/retention.json`. The failed
candidate's full-frame archives were subsequently reverified and released as
`source-release.json.gz` records; they have no remaining recovery claim.
Selected movies, compressed manifests/runtime records, source crops, all-frame
diagnostic atlases and audit scripts remain retained. Do not infer display
swaps, panel cadence or photons from these engine captures or decoder records.
