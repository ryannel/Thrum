# Inspection plan, recorded before viewing captures

Review e2129272a95335ae94aeb680668dc553c60b61d4.

- D3.2: compare both native recipes against retained M0 exact linear/SDR checkpoints; inspect full frames and native-size Sun, stars, shadow, geometry, local-light and mirror regions. Check cut before/after, exposure/sample identity, both surface formats, non-1:1 letterbox. Inspect realtime route/variant and stall transitions separately from fixed-time byte proofs.
- D3.4: run moving smoke; compare Chrome day/night fixed linear/SDR checkpoints with its retained baseline, inspect the same content, cuts and letterbox, and inspect realtime motion separately.
- D3.5: establish OS capture permission and active session; identify game PID/window/tool, bounds/backing scale and host clock. Compare both recipes' paused day/night/window states with T2 references. Exercise ordinary moving controls, resize and surface/device replacement; inspect recovery and geometry. Do not infer compositor or cadence from engine images.
- Profile edges: low sun, storm/variants, distance/route, fast camera/cuts, full-refresh operation where reachable. Panel cadence remains outside this milestone.
