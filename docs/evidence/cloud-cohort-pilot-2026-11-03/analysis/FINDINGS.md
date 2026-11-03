# Frozen 7c cohort diagnosis — 2026-11-03

This is a read-only diagnosis of `../cloud-cohort-proof`, build `7c719e6a164480db`. The commissioned native movies failed CF01 billowing. No served source, renderer, assets, camera, κ or clock was changed. The proposed remedy in `TWO-ROLE-CARD.md` is a design candidate, not a selected or implemented source.

## Decisive data

The counterfactual parent is sampled at **the same actual pose, shear, yaw, scale and age** as the observed parent, with only age/weather loss removed. It retains the ordinary reference-distance taper. Therefore parent erosion cannot count as child growth. `all OLD` is the max of every older cohort present in this snapshot, each with loss removed. These times have exactly zero weather exposure.

At q > .005:

| Source time | Child / age | Child volume outside direct parent potential | Outside all OLD potential | New front / side projection versus all OLD |
|---|---:|---:|---:|---:|
| 170 | 2 / 40 | .53% native (.45% world) | .53% native | 0% / 0% |
| 200 | 2 / 70 | 12.73% world | 12.73% world | 0% / .028% |
| 220 | 2 / 90 | 27.96% native | 27.96% native | 4.56% / 2.22% |
| 280 | 2 / 150 | 29.71% native | 29.71% native | 5.20% / 2.32% |
| 280 | 3 / 20 | 0% | 0% | 0% / 0% |
| 350 | 3 / 90 | 32.75% native | 5.33% native | 0% / .345% |

“Native” is full bounded 256³ material-voxel-centre quadrature mapped into world space, with determinant volume weights; a second half-native-cell phase is retained. “World” is an 8m world grid covering 5,662,020 points at each time. Projections use the latter. These are sample estimates, not continuous-support theorems. Projection percentages use each child's thresholded projection as denominator. All OLD projection testing is independent of the renderer camera.

The 200s check is especially useful: root-parent erosion begins at age200, so this is before any root loss has occurred. Child2 adds depth-facing volume, but adds no front silhouette and only 8m of side protrusion; neither view has any novel area >=32m outside parent projection. The top projection adds 9.02%. At350, child3's apparent gain against its immediate parent is mostly already inside the older root's counterfactual shape: zero front novelty, maximum side protrusion22.6m, and zero >=32m area. Merely checking direct-parent volume would accept this failing replacement.

The joint actual q>.005 volume is 291.27, 290.81, 300.11, 133.52 and149.04 million m³ at100/170/220/280/350. Corresponding all-current uneroded potential is291.27,290.81,324.97,329.91,332.54 million m³. Hence the major later change is replacement during parent loss, not sustained exposed local-lobe growth.

## Structural cause

The field is q(X,t)=max_j q_j(X,t), where q_j=A_j rho(a_j) smoothstep(-8,8,d(a_j)-M(a_j)E_j), M=.3+.7N. The support of max is a union. **Max itself does not prohibit new billows**, and mature child2 is not wholly nested: by220 it has appreciable volume outside the parent. A blanket “all children remain buried” diagnosis is false.

The producer nevertheless creates the wrong interaction:

1. The source content is a complete, shallow, multi-crest Cu formation, not an individual rooted billow. Its native positive-density bounds span about318×166×466 local metres: at scale4, approximately1.27×.66×1.86km before shear. A child is another whole formation at .65–1 scale.
2. Admission selects the nearest certified site to the region centre, from eight sites clustered near the deepest material anchor. Child2's selected root is only35.78m from the parent's anchor; child3's is about30m away. This is tiny compared with the kilometre formation. The certificate proves a positive ball/neck, not exposed crown placement.
3. With default forcing the child centre follows the parent's material socket to numerical precision (separation<7e-13m in these snapshots). Their root velocity matches, root height evolves together and yaw is fixed. There is no separate outward component trajectory. New scale initially expands entirely inside the parent; later yaw/shear expose mainly depth-facing pieces.
4. Both full templates continue competing under max. Their interior maxima change density/lighting; the old silhouette remains dominant. When its age erosion becomes large, a smaller whole-template winner appears. This predicts the observed near-identical twin crest followed by whole-formation size pulses.

The existing finite source, fresh-rest tracking, single W integration, loss, attachment tests and shared optical readers are useful retained mechanisms. Relaxing the socket radius, summing opacity, increasing noise or changing κ does not supply component-scale content and an outward placement constraint.

Exact implementation locations in frozen `../cloud-cohort-proof/src/cohort-model.js`: constants/growth2–7; maps16–20; age loss21; prospective burial/neck23; full-template scale/yaw29; nearest deep-site selection30–31; certificate monitoring39–42; immutable-time presentation45–46. `src/cohort-oracle.js:5` and `shaders/cohort-source.wgsl` evaluate the max field. See `snapshots.json` for exact source SHA pins.

## Reproduction and limits

Run from this worktree, without a browser:

```
node tmp/cloud-cohort-diagnosis/export.mjs
/opt/homebrew/bin/python3.14 tmp/cloud-cohort-diagnosis/measure.py
node tmp/cloud-cohort-diagnosis/crosscheck.mjs
node tmp/cloud-cohort-diagnosis/supplement.mjs
/opt/homebrew/bin/python3.14 tmp/cloud-cohort-diagnosis/supplement.py
```

`metrics.json` contains all5 requested times, thresholds1e-7/.001/.005/.01/.04, phase-separated native estimates, mass, support and projection statistics, exact maps/neck history, tool versions and source pins. `supplement.json` adds200s and projections against all OLD; it also contains two bounded ROI content screens. `crosscheck.json` compares55 queried points to the frozen JS oracle: max absolute q difference2.9243e-7 (declared diagnostic tolerance2e-6), with all source pins unchanged. This checks the Python evaluator, not hardware or renderer equivalence. The root's actual GPU probe is separate evidence.

`registered-opacity-correct-axes.png` uses fixed-scale optical line integrals, κ=.019, removes only common region XZ translation, retains all vertical motion/shear/yaw and correctly displays Y vertically in both views. It is a scientific projection, not native beauty or art acceptance. The earlier `registered-opacity.png` had transposed side-image axes; its scalar metrics were unaffected. Do not promote that earlier image.

Cost failure is recorded separately: first full-world analysis peaked635,240,448B (605.81MiB), exceeding its planned512MiB. The first supplementary **feature-screen** broadcast full-volume float64 masks/EDT and peaked2,048,573,440B (1953.67MiB); it is retained as `supplement-v1-high-memory.py/.json`, not a recommended recipe. No 3D output was retained. Corrected ROI-only `supplement.py` operates on at most80×56×84 and84×56×68 component subarrays; its whole streaming run peaked298,483,712B (284.66MiB), including the128MiB rho/d input arrays. Additional live work is therefore below256MiB. Full-volume reallocations/searches stop here. All retained diagnosis outputs total under1MiB. No runtime performance claim follows from this offline CPU study.
