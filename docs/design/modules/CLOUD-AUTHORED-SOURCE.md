# Authored source replacement: finite material, persistent weather

3 November 2026. **Provisional candidate; the first reviewed Cu pilot fails visible billowing.** This replaces neither the selected renderer nor the public provider contracts. Complete Cb/family authoring, general source control and full-sky performance remain unfinished. [CLOUDS](CLOUDS.md) governs scope, [capabilities](CLOUD-CAPABILITIES.md) define success, and [migration](CLOUD-MIGRATION.md) remains the only active checkbox queue.

The [failed transport sources](../../evidence/cloud-front-growth-2026-11-03/README.md), [qualified static Cu ingredients](../../evidence/cloud-authored-masters-2026-11-03/README.md), [local erosion comparison](../../evidence/cloud-geometry-operators-2026-11-03/README.md) and [rejected frozen storm anatomy](../../evidence/cloud-cb-anatomy-2026-11-03/README.md) constrain this proposal. Correct transport did not supply a good storm. A good static Cu and a correct erosion operator do not supply renewal or moving breakup. A connected storm skeleton and a filled smooth column both fail appearance.

## Operating model

A persistent E1 weather cell owns a bounded population of finite-lived authored volume elements, called cohorts here. Each is newly condensed material with a stable ID, template/seed, birth time, local growth, erosion, phase and material map. The cell and its weather/storm causes persist while new billows replace old material. No complete lifecycle movie, Houdini dependency or fluid solver discovering the anatomy is required.

Finite lifetime is structural. A permanent material master under wind W_x = s y accumulates shear s t indefinitely. Clamping or reversing its map cancels wind or introduces resets. A newborn cohort therefore starts with fresh rest coordinates and zero accumulated differential wind. It inherits the parent's socket position and initial world velocity, **not** the parent's deformation, exposure or wind history. Otherwise strain would accumulate through generations.

At birth, let u_0 = v_socket − W(x_socket,t). Thereafter C_dot = W(C,t) + u. The internal velocity approaches the recipe's uplift/outflow continuously. This applies shared weather wind once. Surviving parent material keeps its own map and age. Attachment is actual geometric overlap, not a recursive constraint that moves the child through the parent forever. Optical separation changes neither ownership nor velocity; existing cohorts continue moving independently.

The anatomy is an authored producer with required data. Templates need actual detailed density, distance, coordinate calibration, bounds, sockets and admissible growth/flow ranges. Cu ingredients alone do not define rooted Cb towers, a fibrous anvil, a connected deck or head-fed cirrus. Those recipes must be supplied and visually qualified; naming a family is not a generator.

## Shared field, phase and optical ownership

For a finite zero-collared template, let rho(a) be decoded density and d(a) positive-inside distance in local metres. A cohort contributes

`q_j(x,t) = A_seed · rho(a) · smoothstep(-w,w,d(a) − M(a)·(E_age(a,t)+E_weather(a,t)))`,

where a = Phi_j_inverse(x,t), M is fixed material structure in [0.3,1], and all exposures/distances use the same local units. Outside the actual template domain, return zero. Native half/trilinear interpolation defines the point field; the imported distance is an opposite-class voxel-centre approximation, not an exact continuous isosurface distance.

A_seed only ramps a tiny buried seed before visible geometric expansion. It is one before the seed emerges; mature elements never fade by this multiplier. New billows change their own support and map, without scaling the whole formation.

Within one authored formation, m = max(q_j). Phase partitions that amount: for m > 0, eta_j = (q_j/m)^4, I = sum(eta_j I_j)/sum(eta_j), q_liquid = m(1−I), and q_ice = m I. At m = 0 return zero. Independent formations add their nonnegative coefficients once. This is an art-directed amount/overlap rule, not conserved water. Do not reapply the rejected procedural material remap to calibrated density.

Phase must be spatial. Added amount uses the declared local condensation phase, then freezing converts extant liquid. If amount grows from q0 to q1, mix the survivor phase and the added amount's phase before freezing; eroded amount does not leave an invisible ice reservoir. Cold upper material can outlive the lower body, but cannot cancel the finite lifetime. A buried child can influence weighted phase before it wins the density max; its optical transition must be continuous and qualified.

Authored q needs a declared optical profile and physical scale. Raw imported values are not automatically kilograms of liquid per cubic metre. Carry the calibrated κ with the ingredient and use it once in every coefficient consumer. The [actual covered-source diagnosis](../../evidence/cloud-light-diagnosis-2026-11-03/README.md) found the inherited κ=.0045 differed from the qualified Cu input’s .019; this suppresses small-lobe optical contrast. Correct calibration does not qualify anatomy, and changing κ to conceal missing growth/cap shape is not a source repair.

Every main, light, receiver, environment and reflected query uses the same source identity, effective point field and presentation time. A hidden main view does not stop retained consumers. Public weather causes remain CPU/E1 facts; pixels never create rain, lightning or wind authority.

## Growth, renewal and guaranteed retirement

A new element begins at a certified interior socket with positive minimum scale. During the seed ramp its entire support stays buried. Once its seed density is established, small geometric features may emerge before they are independently resolved; this requires current sampling and visual inspection. Do not require the entire child volume to stay buried until a large neck forms: review proved that condition infeasible for the chosen master.

For established interior balls, a sufficient connection is

`min(r_parent, r_child, (r_parent+r_child−separation)/2) >= h_ref`,

with both balls above the declared attachment density over the establishment interval. Tiny positive tails do not qualify. Initial containment, later neck width and visible attachment are separate checks. Intentional adverse weather can cancel establishment and cause early breakup; that outcome cannot be counted as successful wet renewal.

The age exposure is irreversible. Wet weather may admit new cohorts, but cannot rejuvenate old ones indefinitely. Choose its positive lower rate/deadline so that, by T_max,

`M_min · E_age >= max_decoded_distance + transition_width + error_margin`.

Drying adds nonnegative exposure. Retirement follows actual zero contribution, never a lifetime cutoff that clips visible material. Dominated cohorts remain stored until genuinely zero because they may become visible again when a newer maximum erodes. The population bound is ceil(max_birth_rate · T_max) plus explicit establishment overlap. Exceeding it is overload, not silent suppression of required renewal.

## Material motion and family extension

The general candidate uses a monotone vertical map, positive horizontal scales and height-dependent wind displacement. The inverse, footprint Jacobian and prospective support must be explicit. Integrate differential wind at each label's actual evolving height. A finite age bounds history; it does not by itself prove acceptable strain under every wind.

The Cu pilot uses a simpler exact affine-height representation described below. Cb would require broad rising material, upper slowing/spreading and spatial freezing of that same material. Keeping the cap as the cohort's upper subset avoids a duplicated detached disk and ownership transfer. The exact cap map, directional anatomy and runtime programme are **not selected yet**. A flattened rounded Cu crown fails even if the phase arithmetic passes.

Sc/St require overlapping bank/sheet content with controlled seams and local holes. Ci requires head-fed directional tail anatomy and differential loss/fall/shear. They share source and optical contracts, not necessarily one shape generator. New authoring work follows published whole-volume shape/detail operations; static success still does not prove its runtime deformation.

## Sampling and cost baseline

Direct instanced sparse-template evaluation is a complete reference/default sampling path. Conservative world-brick candidate lists identify all contributing cohorts; evaluate their native point fields and formation max before transport. A global final-q volume or an unspecified adaptive up-rez scheme is unnecessary for mathematical completeness. A final-field cache can be a later measured optimization.

Static storage can use RG16FLOAT density/distance, 8³ cores and 10³ guarded slots. Core admission includes the one-texel dilation of positive density; merely padding nonzero cores loses cross-core trilinear support. Preserve exact addressing and outside-domain zero. Actual 256³ counts are 2,464 admitted cores for 03 and 2,927 for 08: useful slot payloads of 9,856,000 and 11,708,000 bytes, plus 131,072-byte indices each. These are measured data counts, not rounded physical atlas residency or GPU timings. Future dense sheets may be much less sparse.

MAX hierarchies provide conservative skip bounds, never average material. `max(filtered parts)` does not equal `filter(max parts)`. Direct native queries followed by ray/pixel quadrature avoid that false identity; convergence and reconstruction remain rendering gates. Any coarse field must average the already combined field and compare with the direct reference.

Count candidates per brick separately from cohorts per cell. Lists must be complete, with exact overflow traversal or explicit unavailable/over-budget status. A naive 1280×720×96 march with eight candidates would exceed 700 million cohort evaluations before light/reflection work. Occupancy, interval skipping, production view resolutions, shared light caches and actual overlap distributions are essential. The method can still fail cost admission.

Control lattice pitch follows mapped material-edge lengths, including shear. Account for CPU updates, uploads, current/prior states, template atlases, indices, light caches and transient swaps. Full mixed-sky population/control dimensions and the 96 MiB source pool remain open. No per-cloud dense 3D allocation or budget pass follows from one ingredient. The optional 20 MiB velocity cache remains a separate flow-provider experiment; this analytic producer allocates none when unused.

## Reviewed Cu interaction pilot

The [retained commission and review dispositions](../../evidence/cloud-cohort-design-2026-11-03/README.md) pin the implementation. This is a liquid Cu interaction, not a full source selection:

- One cell, at most eight preallocated cohorts, one shared 03/256 density/distance pair. No full old PDE/rest allocation or per-view source step.
- Tiny seed size 32 m; four growth-seconds of density startup, then geometric expansion through growth time 100 s. Growth time integrates a two-second wet/lift response. Replacements arrive at 130-second intervals while regional forcing permits them. After real clear, wet forcing may start a fresh tiny parentless seed under the same cell ID, with a new cohort ID and the same minimum cooldown from the last actual birth. Old surviving material never resets; failed attached admissions remain reported.
- Target scale 0.65–1; b = 4·target_scale·growth enters every map, inverse, wind integral and bound. New rest and differential-wind state start at birth; socket position/initial velocity are inherited correctly.
- Mean wind magnitude 0–12 m/s and height-gradient magnitude at most 0.002/s. Fixed 17³ exposure controls meet the 200 m pitch screen: the maximum mapped material edge is at most 2,523.7 m, giving 157.7 m node spacing.
- Age exposure is `endpoint·smoothstep(200,360,age)` in source seconds. Weather loss is one local metre per second times the persistent trailing dry profile. Wet weather does not erase accumulated exposure. The actual .04 attachment mask and all native guard vertices are independently checked.
- The dry front starts at source 420 s, with width 800 m and default speed 50 m/s. Its value inhibits admissions at the moving condensation region and growth at each cohort centre, while local material loss samples the full mapped lattice.
- One fixed quarter-second source clock and actual prior/current parameter interpolation feed every consumer. Presentation lags the **committed** clock by at most one interval; requested-clock backlog and display latency are separate. Presentation forcing comes from its actual half-open interval, never a newer committed control change. Rate 16 cannot discard state updates silently.

Actual native birth/growth, multiple replacement intervals, changed wind/front forcing, clear-to-wet recovery and moving source-off remnants must be inspected, with normal and accelerated recordings. Source opacity/material diagnostics, independent reflected rays, receiver shadows, CPU work, uploads, resources and actual cadence accompany the beauty views. No alpha scaffold, periodic clone stack, frozen mature outline, reset, clipping or dissolve-only shutdown qualifies. The [first actual Cu pilot](../../evidence/cloud-cohort-pilot-2026-11-03/README.md) passes bounded source/query/clock checks but fails billowing: near-root whole-Cu replacements largely hide early growth, then change the dominant whole-cloud envelope as older material erodes. Some new support does emerge; this is not complete geometric nesting. A finite lower-body coverage role and compact rooted crown/billow content, with useful camera-facing new support before old erosion, are the next design correction. Exact content, turnover, sockets and programme require derivation/review before another build. No complete source or lifecycle is accepted.


## Revised BODY and local-crown integration

The [held component evidence](../../evidence/cloud-component-proof-2026-11-03/README.md) now separates the lower-body role from compact crown growth. V1 proved visible local emergence but exposed clipped anatomy and is rejected. V2 keeps the natural compressed BODY roof and one rounded crown basis at unequal scales; actual held images provisionally qualify these inputs for integration. They do not establish a continuing source programme.

The [complete integration commission](../../evidence/cloud-component-proof-2026-11-03/INTEGRATION-COMMISSION.md) supersedes the first pilot's content/cadence constants for this bounded Cu build. It specifies BODY renewal at100s with300s life, crowns from70s at50s cadence with150s life, fixed quarter-second source state, bounded affine wind, irreversible age/front exposure and eight preallocated optical slots including retained publication states. Fresh material keeps its own deformation history. BODY and crowns remain one cell combined by max, with one calibrated coefficient queried by all optical consumers.

Prepared finite socket/head catalogues and conservative support bounds must make repeated connected growth feasible without per-birth volume scans or camera-selected anatomy. A head must develop a resolved exterior patch beyond all older loss-free potential; two held successful placements cannot stand in for this runtime qualification. Actual BODY size/lease matters for renewal at its root; global BODY maturity is required for crown sockets, not intrinsically for a zero-offset BODY root. Condensation supply controls growth and admissions; explicit dry-front exposure controls drying. Supply off does not automatically erase the cloud, and old exposure never rewinds on supply recovery.

Build the complete interaction before expanding content regimes: birth, continuing billows, two BODY turnovers, changing wind, spatial loss with moving remnants, clear-to-wet recovery and shared retained consumers. Judge normal and accelerated recordings alongside numerical source/clock/query tests. If the catalogue fails continuous healthy births or turnover produces global size pulses, this integration remains failed rather than silently changing cadence or cue thresholds. Full atmosphere reconstruction, Cb phase/anvil and full-sky cost remain governed by the single migration queue.


The [integrated Cu checkpoint](../../evidence/cloud-system-pilot-2026-11-03/README.md) now runs this programme through the shared optical query. Actual normal growth samples show a left head then a right head developing while the lower body persists; normal drying samples narrow the source into moving wisps before clear. Matched raw/effective diagnostics distinguish support loss from an uneroded ingredient. The accelerated0–640s recording covers repeated roles and retirement. Source/catalogue/clock/query tests and held main-off consumers pass their bounded scope. Broad/soft head anatomy and some size-change character remain; these results do not complete CF01, Cb or the production atmosphere/cost path. Next qualify the remaining forcing/view/cost cases rather than promoting the whole module from this first coupled prototype.


The current prepared exterior-cue certificate bounds older loss-free q>.005 and a new q>=.041 patch; it is not separation from every strict-positive tail. Original native q>0 bounds remain authoritative for rendering. Treat this thresholded cue as engineering admission plus actual visual judgment, not a strict support theorem. The selected source clock is .25s; the [hardened c3e0 lab](../../evidence/cloud-lab-hardening-2026-11-03/README.md) now rejects unsupported nonquarter SourceClock construction. Direct-model timestep refinement remains separate.


## Crown anatomy gate after integration

Native2km front/side/oblique384-ray views still reject the broad oval crown/upright side wall. The
[one narrower v3 envelope screen](../../evidence/cloud-crown-screen-2026-11-03/README.md) is rejected before distance
cooking: it trades the oval for a smaller mitten/peanut, loses substantial amount and retains the column in composite.
Do not adopt its provisional certificate or search more primitive radii. Keep v2 as the integration/cost reference,
not accepted source art.

A [bounded next geometric-displacement proposal](../../evidence/cloud-crown-screen-2026-11-03/NEXT-OPERATOR.md)
uses one fixed, material-space two-band surface-conditioned envelope displacement, preserving the existing rooted
bridge neighbourhood and optical calibration. It has explicit native/world scales, hash/interpolation, support and
resource bounds, and rejection criteria. This is a proposed pre-cook screen, not authored content or a disclosed
Decima lifecycle. Maximum displacement does not imply typical visible structure: the expected perturbation can remain
hidden by the broad density/distance profiles. If that single controlled screen remains a rippled oval, reject the
recipe and return to natural component selection rather than tune a seed/amplitude search. No new runtime noise,
κ compensation, crown asset, distance field, catalogue or final art acceptance is selected by this proposal.


The [single middle-scale displacement](../../evidence/cloud-midshape-screen-2026-11-03/README.md) has now been
executed and rejected: its completed front remains an oval with modest scallops; the reservation guard stops the
remaining views. Do not recook this recipe. [Native source inspection and a3D upper-core gate](../../evidence/cloud-natural-head-selection-2026-11-03/README.md)
identify a connected natural head, distinct from the neighbouring bank aboveY300. The next candidate uses that
measured structure plus a compact selector and protected root, rather than redefining the visible upper shape with
an enclosing primitive. It must still qualify the actual selected halo/lower joins under birthQ and the complete
new rho/d/metadata/lifecycle. No new source asset is adopted by a component-label pass.


## Natural-crown integration and remaining admission correction

The [natural-core screen](../../evidence/cloud-natural-crown-screen-2026-11-03/README.md) and
[fresh optical cook](../../evidence/cloud-natural-crown-cook-2026-11-03/README.md) now produce a fixed rho/d input,
closed-cell support, positive root/head chains and a frame derived from its actual crest. The
[actual ae5533ec5335b4ca diagnostic](../../evidence/cloud-natural-integration-2026-11-03/README.md) changes prepared
inputs only; source and GPU equations remain17f. Actual native views show more irregular crests but still large tall
masses; sampled normal renewal shows local emergence and retirement, while default drying breaks into moving remnants.
Repeated large shapes remain noticeable. None of this accepts the artwork or full lifecycle.

[Unchanged-source replays](../../evidence/cloud-natural-admission-2026-11-03/README.md) fail healthy renewal first at370s.
Attachment passes, but all eight sites fail the exterior certificate. Many head samples really overlap older potential;
two sites have a nominal certified head that fails only after independent future-motion inflation. Removing uncertainty
is a diagnostic ablation, not a permissible fix. No threshold, cadence, scale, reference membership or lease changed.

A separately preserved **optional** correction is a pairwise shared-cause bound. It is off the delivery path: arbitrary-future exterior guarantees exceed the required experienced behaviour and introduce costly interval/barrier machinery without repairing actual overlaps. The original rule and its failures remain historical evidence. Its continuous derivation is: For a fixed material
point in each of two poses, let `v_i` be its constant birth-frame offset, `b_i` its growth scale, `C_i` its centre and
`u_i` its internal centre velocity. Subtracting the implemented horizontal equations gives
`d(x_i-x_j)/dt = S(t)*(Ciy-Cjy + b_i*viy-b_j*vjy) + (uih-ujh) + b_i'*vih-b_j'*vjh`.
Common mean wind cancels exactly. Relative future shear uncertainty can therefore be bounded by
`.002 * integral_0^45 abs(Ciy-Cjy + b_i*viy-b_j*vjy) dt`, retaining known initial deformation/internal motion and
all growth residuals. Two crown centres share the same future uplift target; a BODY/crown pair retains a bounded
uplift response difference. The full leaf/head-ball domains and admitted arbitrary future controls must be bounded,
not just their centres or current wind. The runtime uses stepwise five-node Simpson growth quadrature: a strict certificate must include its numerical/roundoff residual or bound those exact discrete updates. Independent equation review found no continuous-model error. This is not yet an executable certificate or evidence that all births can pass.
Independent review, unchanged replay cases and actual motion must qualify the correction before adopting this input.


## Production placement and observed establishment

Select the [separately versioned `cu-held-predict-observe-v1` direction](../../evidence/cloud-observed-policy-design-2026-11-03/README.md) for the next bounded source proof. Its detailed contract has been independently challenged and corrected for publication-retained state/time and external clock-epoch ownership. An [isolated opt-in CPU implementation](../../evidence/cloud-observed-policy-cpu-2026-11-03/README.md) passes functional replay/default-path checks and independent review, but its first whole-step4ms host gate fails: maxima4.766750ms single wet,5.234000ms single dry and8.634250ms two wet cells. Healthy420/820s opportunities still decline. Keep runtime adoption and actual optical experience open; lower percentiles do not override the failed maximum. Subsequent [exact work-removal candidates](../../evidence/cloud-observed-policy-work-2026-11-03/README.md) preserve the same source states and improve the observed wet-case cost, but the latest single-dry4.203417ms and coupled-dry7.425500ms maxima still fail. The independently reviewed on-demand observation API is functional. Production adoption remains blocked. An explicitly authorized visual-reference build may run the exact reviewed12777301 source to judge anatomy and lifecycle while preserving both cost failures in its UI/receipts; this is not cost or runtime-provider admission. The subsequent [weather-zero candidate91b967](../../evidence/cloud-observed-policy-weather-zero-2026-11-03/README.md) preserves the tested source outcomes and passes both unchanged CPU protocols (hardest3.979667ms,20.333µs margin). Select it only for a fresh optical assembly; the127 reference remains exact and cost-failing. Neither this host screen nor functional equivalence admits source art, GPU cost, full skies or a Thrum provider. This changes an engineering admission rule, not the visible-growth behaviour floor. Original370s and other failures remain failed under their original rule. No current asset or source-art acceptance follows.

The [natural coupled visual reference](../../evidence/cloud-natural-coupled-reference-2026-11-03/README.md) now records the exact127 source through normal320→430, repeated16×0→1200 and two drying views. The370s birth is actually exposed at415s;420s still declines. Sampled native images show local new crowns over persistent lower material, but broad/soft anatomy, few blunt upper masses, shrinking/paling remnants and synchronized twin formations fail the desired experience. These are content/programme shortcomings, not repaired by the96→384-ray comparison or a CPU cost pass.

The [fresh source91 assembly](../../evidence/cloud-assembled-source-2026-11-03/README.md) has also run through normal110s, accelerated75s and moving-front90s with the same renderer/native inputs. Actual source100 decoded pixels match127 exactly; finite CPU/GPU probes, main-hidden mirrors and the remote blocker/receiver comparison pass their bounded contracts. This admits the integration checkpoint only. Cache accuracy, complete air/fog/environment, believable breakup and distinct formation anatomy/programmes remain required work; no CF01, provider or full-sky acceptance follows.


Keep the existing safe seed burial/root/neck,45s conditional attachment lease, fixed source physics, eight-slot lifetime/retirement, one wind/clock and shared native optical field. Predict candidate placement through180 actual quarter-step geometry updates under current committed controls held constant, with the declared moving front but no future queued-command knowledge. Choose only an attachment-safe, spatially exposed predicted endpoint; prediction is explicitly conditional, not a promise about arbitrary future forcing. Declined opportunities remain recorded.

At birth freeze exactly the original all-old ID set. Normalize same-ID previous-positive/current-zero members to their exact current birth-barrier maps; preserve prior state as provenance. The qualified producer proves prior-only-positive membership unreachable and rejects incompatible injected states. Advance loss-suppressed CPU-only reference maps once under each actual completed interval, including retained/retired IDs, before monitoring/evaluating/cleaning the45s record. Those maps never enter rendering or become parents. Actual observation uses all four existing heads, unchanged `.041` positive/`.005` old cue thresholds and6m/resolution requirements. Positive native witnesses may prove occlusion; sampled zeros cannot prove emptiness. Record exposed, occluded, uncertified, weather-cancelled and interrupted/invalid outcomes distinctly. Never relocate/delete/fade/rewind a child to conceal a wrong prediction.

Raw records belong to their source-model lifetime and local IDs. SourceClock remains the only epoch owner; qualified IDs are bound only by explicitly supplied publication/capture namespaces, including explicit epoch0. Reset/seek replaces/replays records; presentation cannot leak committed-future outcomes. Legacy mode remains an exact separately selectable reference. Pure native source queries read pinned shared R16 inputs, not renderer state.

The new direction needs a predeclared whole synchronous-step4ms host gate including allocation, the original single-wet/single-dry/two-wet cases and the separate two-cell dry900 case at roots[0,0]/[2400,-1200], exact legacy differentials and the same six forcing/recovery replays. No async source transaction is introduced. Actual normal320→410 motion must span the original370 failure, then continue through the415s observation deadline and next420s opportunity (use a320→430 window). A clip stopping at410s cannot prove the new370s birth's completed45s observation. Include repeated turnovers, wind response, breakup, native views and retained optical consumers. Healthy declines/cue deficits remain open renewal findings. A plausible still, predicted endpoint or changed success label cannot complete CF01, Cb or the system.


## Native authoring correction: rejected amount/base/edge recipe

The [single fixed pre-distance authoring screen](../../evidence/cloud-natural-content-preflight-2026-11-03/README.md) rejects the proposed monotone density transfer, multiplicative condensation gate and subtractive edge bands on the current smoke-derived guide. All5633fixed central-footprint profiles reach the selected substantial peak, but only56.70%have a10→90rise≤24worldm (required80%); their50%height range is54.19m (required≤16). Stronger pre-distance optical amount does not make a coherent level base where the guide's dense material begins at different heights. Native and fixed-pose scalar projections also retain a broad bank and few large masses. These are scalar feasibility views, not newly rendered clouds or a final-q topology certificate.

Stop this candidate before signed EDT, support/certificate cook or runtime integration. The exact failed recipe, first guarded run, complete fixed slices and original diagnostic omissions/correction are retained; no seed/threshold/pose retry or light/κ compensation was used. Current source91/6bd and127/7bf remain exact. The next content decision needs an explicit3D macro/base authoring producer whose bulk structure supplies the required unequal billows/saddles and condensation face; source detail may use imported guides, but the old guide cannot define the target. That replacement is under bounded design challenge, not selected or implemented by this rejection. Keep all shared-reader, new-field recertification, true growth/breakup and cost gates open.
