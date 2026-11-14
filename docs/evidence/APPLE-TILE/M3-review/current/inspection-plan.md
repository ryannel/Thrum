# Independent M3 inspection plan

Review source: 9d6318cff422df7f2b05519f7a30b72821f73b63.

Before inspecting images, check:
- D3.2 native portable and apple-tile: complete cooked valley geometry, Sun/sky, shadows, local lights, mirrored objects and night stars at full resolution; inspect source-pixel crops if needed.
- Exact identity: retained M0 linear and SDR controls, both surface formats, nearest source coordinates and non-1:1 letterbox, matching exposure and sample/history identity. Distinguish still/fixed checks from elapsed-time motion.
- Motion: route start/hold, moving camera and objects, cut and final hold; night/day variant transition; stalled rendering must resume without hidden loss of content. Review frame strips with host timing; do not claim panel cadence or direct video perception.
- D3.4 Chrome: same content at day/night, cut and moving route, resize/letterbox; compare with Chrome's own pre-change deterministic baseline, not Metal bytes.
- Listed target edges: low sun, storm, distance, fast camera and requested full-refresh opportunities. Record any unexercised condition and whether the existing fixtures cover it.
- D3.3: optional pass presence/absence, names and nulls in actual runtime samples; CPU callback/encode and GPU counters stay distinct.

Artifacts and logs below are owned by this review. Retain compact review evidence, remove only review-owned reconstructible bulk output at closeout. No OS-window proof is required by the current M3 items.
