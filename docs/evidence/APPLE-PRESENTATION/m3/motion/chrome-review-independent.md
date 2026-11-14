# Chrome motion: fresh builder self-check

GPT-6.1 Sol, medium effort. This is a subjective-work self-check, not the independent milestone review. No renderer, browser, GPU or capture job was run and no code or milestone record was changed.

## Observation

The inspected Chrome sheets show no obvious long duplicate silhouette behind the moving prop, no persistent ground-shaped smear, and no gradual transparent fade on spawn. The first populated spawn crop is already a solid cube. TAAU smooths the sphere and shadow boundaries and noticeably softens the ground versus off.

There is a visible imperfection worth retaining in the record: in reveal-taau frames 144–155 the exposed middle stack of small blocks and the moving cube’s lower/trailing border have a fine ragged/speckled fringe. Off has crisper, simpler stair steps. The Mac reveal TAAU sheet shows the same kind of ragged boundary, so this is not evidence of a Chrome-only regression. I would investigate/reduce this disocclusion fringe if pursuing the stated shipped-game edge-quality bar; I would not describe these pixels as perfectly clean.

All four Chrome star atlases look spatially stationary across their rows in both modes. I do not see stars hopping, disappearing or forming obvious trails. This is a static all-frame comparison, not an observation of real-time twinkle perception. Mac star atlases 0 and 2 look similarly stationary. The broad sky colour progression in the Chrome scrub whole sheets agrees between modes, without an obvious stale sky state at the sampled points. Mac sky sheets show the same general progression. Half-second whole-sheet sampling cannot rule out a short brightness lag.

No clear Chrome-only fault is established by these images. Chrome is 1280×720; the Mac crop source box and apparent scene scale differ, and separate runs are not pixel-aligned comparisons. The visible reveal fringe appears on both targets.

## Exact visual inspection scope

Viewed with view_image: every Chrome *-whole.png and *-consecutive.png for pan, reveal, spawn, stars and scrub, both taau and off, plus stars-{taau,off}-star-{0,1,2,3}.png in chrome-review. Whole sheets are downscaled context images; consecutive and star sheets contain native source-pixel crops, although tool presentation may resize the sheet. Ordinals below refer to manifest records, in row-major order.

- pan-taau: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, source box [0, 360, 850, 280] (x, y, width, height).

- pan-off: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, source box [0, 360, 850, 280] (x, y, width, height).

- reveal-taau: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

- reveal-off: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

- spawn-taau: whole ordinals 0, 30, 60, 90, 120, 146, 176, 206, 236, 265; consecutive 119, 120, 121, 122, 123, 124, 125, 126, source box [350, 400, 550, 240] (x, y, width, height).

- spawn-off: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 118, 119, 120, 121, 122, 123, 124, 125, source box [350, 400, 550, 240] (x, y, width, height).

- stars-taau: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

- stars-off: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

- scrub-taau: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

- scrub-off: whole ordinals 0, 30, 60, 90, 120, 150, 180, 210, 240, 269; consecutive 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, source box [350, 400, 550, 240] (x, y, width, height).

Chrome star atlases: all ordinals 0–269 in each arm, 24 columns, boxes [1136,112,48,48], [416,0,48,48], [672,160,48,48], [144,56,48,48]. Source correspondence is recorded by chrome-review/run.json and the original chrome-motion-final manifests. I read the generator and metadata; I did not independently rehash sources.

Mac comparison images actually viewed: native-crops/pan-taau-crops.png (labels 000455–000460), spawn-taau-crops.png (000460–000465), reveal-taau-consecutive-source.png (all 12 cells; source metadata submissions 506 onward); star-review/{taau,off}-{0,2}.png (all cells); scrub-review/{taau,off}-sky.png (all cells). No other Mac images were inspected by this self-check.

## Limits and disposition

My visual input here was static image sheets. I did not view the MP4s playing or the running portable game, and decoder execution would not establish that perception. Consequently this note supplies supporting observations and a concrete edge-quality concern, not a full D3.3c motion pass. The categorical “no ghosting, crawl, shimmer or brightness lag” claim still needs motion observation at native scale. Diagnostic flux/lag numbers were not used as visual acceptance thresholds.
