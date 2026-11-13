# M3 local output ownership

The CULLING M3 builder owns new `tmp/g7-showcase/runs/culling-*` runs and
`tmp/culling-m3/`. These hold exact route comparisons, paired timing rows and T2
fixed/realtime captures from this worktree. Expected peak is 16 GiB per serial
capture plus reconstructible build outputs. Inputs are the checked-in valley,
moving-v1 route and culling-fast-yaw route. Selected reports, source identities,
stills, consecutive crops and motion previews are promoted to `docs/evidence/CULLING/M3/`.
Keep complete local capture sources through M3 review; close out only these owned
outputs after retained sources and promoted evidence are verified. No backup is implied.
