# Finite Cu material pilot: correct shared field, failed billowing

3 November 2026. **Source appearance rejected; useful bounded operators retained.** [CLOUDS](../../design/modules/CLOUDS.md) governs the module and [migration](../../design/modules/CLOUD-MIGRATION.md) is the single active queue. The [reviewed commission](../cloud-cohort-design-2026-11-03/README.md) and [candidate design](../../design/modules/CLOUD-AUTHORED-SOURCE.md) do not become accepted through these numerical passes.

## Exact runnable proof

Build **7c719e6a164480db**, loaded and captured in the actual Mac in-app browser at `http://localhost:8150/build/7c719e6a164480db/index.html`, defaults to the selected production additional-scattering triple `(a,b,c)=(.5,.5,.5)`. The earlier **eac4ae3bc6a5a681** used Legacy `(1,.25,0)`; its source and observations remain separately identified. All source, clock, phase and integration bytes are unchanged between these builds. Only profile selection, its three existing uniforms and receipt/UI plumbing change. This is a bounded inherited renderer, not a completed production atmosphere path.

From the repository root:

```sh
python3 docs/evidence/cloud-cohort-pilot-2026-11-03/restore.py --output tmp/cloud-cohort-review --test
python3 tmp/cloud-cohort-review/serve.py --port 8153
```

Open `http://localhost:8153/`. Start paused at zero; use Mature100s, the named source windows, Replay, 1×/16× and capture/probe buttons. Restore eac4 with `--build eac4ae3bc6a5a681` and a fresh destination. [Build identities](builds.json) pin all 63/62 served files. The ZIPs retain exact code, metadata, recipes, tests and notices; the density/distance/noise inputs reuse SHA-verified compressed payloads from the [master](../cloud-authored-masters-2026-11-03/README.md) and [geometry](../cloud-geometry-operators-2026-11-03/README.md) evidence. Fresh restores verified both identities and their five/four tests. The current restore also materializes the historical comparison snapshot, sharing its identical large inputs by hard link.

## What actually happened

Native 1280×720 images at source100/170/220 show a recognizable detailed Cu ingredient. They do **not** show convincing continuing billows. The first replacement grows deep inside the old whole-cloud template. Most new material is hidden by the formation max. As the older, larger template erodes, a smaller whole-cloud replacement becomes dominant, producing a size pulse rather than local crown development. No failure is repaired by merely calling these elements new material or counting their birth/attachment events.

The two actual selected-profile clips are:

- [90 wall seconds at 1×, source130→220](observations/cohort-renewal-1x-90s-7c719e6a164480db-7b266289-3.webm): fixed world camera, 1,350 decoded frames. [Ten-second sampled native crops](observations/renewal-contact-10s.png) show almost the same twin-crest outline with slow tilt/translation.
- [40 wall seconds at 16×, source0→640](observations/cohort-accelerated-16x-40s-7c719e6a164480db-7b266289-4.webm): explicitly follows the weather region horizontally at fixed focus height, 600 decoded frames. [Three-second sampled views](observations/accelerated-contact-3s.png) expose the size pulse and eventual spatial loss to a small remnant/clear view.

Both decode completely, report no retained encoded-byte drops, and pass their **numerical** purpose gate. Root inspected actual held images and decoded temporal samples; this is not a claim of uninterrupted watched playback or fully witnessed motion. Continuing billowing, persistent-body stability and convincing moving breakup remain failed/unaccepted. No family, Cb, full-sky or weather-system promotion follows.

At8km the main cloud, receiver shadow and independently traced mirror cloud are visible in [capture7](observations/cohort-t220000-mode0-7c719e6a164480db-7b266289-7.png). [Main hidden](observations/cohort-t220000-mode0-7c719e6a164480db-7b266289-8.png) retains mirror and receiver; [shared zero](observations/cohort-t220000-mode0-7c719e6a164480db-7b266289-9.png) removes them. The3km reflection lies outside that camera's field of view. These held ablations do not qualify complete dynamic air/environment/rough-water coupling.

## Correctness, cost and review scope

Actual shared GPU/CPU source probes at source100(eac4),220(7c) and640.018(7c) pass the declared experimental f32/half-sampling envelope. The220 probe has8,024 points,263 positive, zero negative/nonfinite/ice values, maximum absolute q error0.0001261. The late probe has no positive effective q on its sampled points; that alone does not certify every native point is zero. Each JSON preserves source tuples, upload hashes, controls, profile, device and logical allocation facts.

Source tests cover inverse maps, fresh deformation history, wind applied once, actual burial/neck, finite retirement/population, causal control publication, initial-off and clear-to-wet fresh-ID recovery, unchanged paused/camera controls, quarter/eighth-step sampled convergence, common query and profile packing. Five actual assembled WGSL modules passed Naga30.0.1. A read-only independent review found no introduced profile/receipt defects; a pending mocked GPU fence retained the submitted profile while a selector edit aborted recording. It also closed the earlier causal forcing and recovery findings. Tests do not establish source art or portable precision.

[Motion analysis](motion-analysis.json) separates encoded frame timestamps, source clock, CPU work, submission cadence and queue-fence elapsed time. Logical GPU allocations total78,843,452 bytes (about75.19MiB), including one shared64MiB template pair; this is not measured process/device residency. Only the latest240 submission records survive (roughly8s), so their roughly34ms median/42ms p95 cadence in the normal clip is **not** a full60-second timing admission. Queue-fence elapsed is not timestamped GPU execution. No5ms/M2/512MiB or full-sky claim is made.

The design correction must distinguish finite lower-body coverage from compact rooted crown/billow content, and require new resolved support outside **all pre-existing uneroded potential** while that older material remains intact. Attachment/renewal and visible local growth are separate gates. Exact role templates, turnover coverage, sockets, lifetime/spacing, wind/sampling and cost must be derived and reviewed before the next build.

The [measured failure analysis](analysis/FINDINGS.md) qualifies the geometry diagnosis: atsource200, child2 has12.73% volume beyond the uneroded parent but0% front/.028% side projected novelty. At350, apparent growth against the direct parent largely lies inside other older potential. The [independent review](analysis/independent-review.md) separates this content/placement failure from max composition itself. Analysis recipes retain their original scratch path assumptions; restore/point-query evidence above remains portable. The605.81MiB and1953.67MiB failed offline analysis peaks are disclosed; the corrected ROI study peaked284.66MiB. These are diagnostic preparation costs, not runtime allocation.
