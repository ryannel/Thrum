# Prepared-source and fitted-light integration review

2 November2026. Reviewed actual prepared-cu.js, scatter-fit.js/WGSL, CuSourceField adapter and cu-preview.js/WGSL/cache
wiring against the selected source/light contracts. No GPU, source/canonical edit or commit. CPU-only checks admitted
the exact pinned package, extracted the lookup and rejected a one-byte-corrupt A payload. This reviews current code,
not a renderer acceptance or a rerun of historical Q1 evidence.

## Finding and correction observed during review

**FIT-R1 P2 — candidate preparation changed the retained source clock before publication.** The first inspected
chooseRecipe set candidate age/duration/slider maxima before next.prepare, product allocation and new bind groups
could fail. Keeping the previous source on failure then left it with the candidate's clock (1800s hierarchy versus
240s prepared source). Root corrected this during review: nextAge is staged locally, next.prepare/new products/groups
complete first, then the successful source/product swap commits clock duration/seek and UI limits. This resolves the
identified control-flow defect **by code inspection**. No UI allocation-failure injection was executed here.

No remaining concrete binding, ordinary-shadow or reflection-composition defect was found in the scoped path.
The numerical/cache/native gates below remain unrun; no new physical certification requirement is introduced.

## Producer/binding/consumer checks

PreparedCu verifies the exact manifest and density hashes before GPU allocation, retains the240s interval and direct
interpolation of the two admitted density frames, and explicitly identifies its static detailed geometry/limited
lifecycle role. Sparse pointer/bitmask decoding agrees in structure with the retained source shader. Binding6 is raw
density and8 the64-byte prepared Params block. Its unused flow declaration is unreachable from canonicalSigma, so the
source field's automatic layout omits it and no unnecessary flow buffer is allocated. The source point probe moved its
own bindings to10/11 to avoid the prepared uniform. Endpoint ages produce an empty field; finite preparation is
validated. CPU versus GPU/resampling error still needs actual probes in this new adapter.

Source prepare/field work precedes matching column/prefix/ambient work in one ordered graph. Source replacement occurs
at a quiescent boundary; old source and fitted products retire after prior work completes. The complete cache key
contains source kind/variant/seed/maxLevel/grid/time, Sun selection, light profile and fit identity; replacing the
source clears the key. Camera/main/reflection changes do not invent another source age. Ordinary geometry/light/shadow
queries retain the same density. Source-hidden main views can still request reflected cloud/receiver effects.

ScatterFit's package check verifies pinned profile/binary hashes, exact float32 axes/row order/counts, declared
liquidHG(.86)/albedo1/kappa.25/rho.8 and finite A<3.724 before allocation. It correctly declares R32float unfilterable,
uses textureLoad/manual actual physical-T/mu interpolation and supplies explicit layouts matching the builder's
0density/4Scene/8table/9axes/10profile plus group1outputs2/3. Uniform byte layouts/counts agree. Texture upload rows are
padded; body/bank product bytes are separately recorded. World-Y alignment is explicit; no rotated/sheared column
provider is silently claimed by this adapter.

The column builder reads each packed Z-group through every valid Y, multiplies by world Y pitch and emits four
independent scalar columns. For the trilinearly reconstructed zero-extended field, each hat integrates to one pitch
when its half-voxel collars are included. The preview now traces those same collars, so the reduction and visible
finite-field support are compatible. Direct canonical T_body versus field/cache error is still a separate comparison.

In fitted mode, the shader restores primary coefficient1 and adds only the isotropic A/(4pi)*exp(-.25tau) source.
Ordinary receiver shadow remains exp(-tau). Main and mirror legs call the same cloudSource and independently trace
source/world position; camera-to-interface media applies once. Sun-off prevents bank preparation/use, and switching
back to fitted Sun mode changes the cache key and rebuilds before drawing. The new2e6 source exit bound is present.
This is the vacuum/external-sky fixture, not full SKY-G/all-layer/reflection-probe/overcast-surface qualification.

## Important unrun spatial-cache approximation

The builder stores A(T_i) at each body-column texel, then bankAt bilinearly interpolates **A**. This is not algebraically
identical to bilinearly interpolating T_body and evaluating A at the query. The selected contract permits an approximate
cache, but its error is distinct from the cooker's0.020958table error and must not be omitted from source-factor claims.

A read-only example using the actual package at mu0=.5 gives mean(A(0),A(10))=1.02210528518 versus A(5)=2.01660539948,
a difference0.99450011429. This is a discriminating possible clear/thick edge, not a measurement that the current master
contains that exact error. Compare the actual cached A/radiance with the canonical column-plus-LUT query at body/map
edges and steep gradients. If it visibly fails, compute the lookup from interpolated T or qualify another cache;
no uniform interval certificate/refinement campaign is required merely to try the labelled native comparison.
Convex A interpolation preserves the hard range, but does not prove the exact slab cap or optical equivalence.

The first body cache in the design was128²; current product size follows the chosen field X/Z grid and also allocates
a separate A map. At256³ that is256²body+bank (524288raw bytes), and reduction visits16.8M scalar coefficients/4.2M
packed texels, not the128³ example's2.1M scalar visits. Actual product bytes are recorded; report real work and generation
reuse instead of inheriting the smaller illustrative cost. There is no performance pass here.

## Reviewed identities and limits

Relevant SHA-256 at inspection:

- prepared-cu.js:373c025ee3882c24e512c121aa17356d2ecb95ef901da562db7b31d60a27e0c7;
- scatter-fit.js:de3cb5d0a49febe58167ab6bf4bbf3393505bd9b653b28aa866ad48d39691808;
- scatter-fit.wgsl:2889371121a5179dca85518e2c8767ae243b6026cbfcd7baa2bcbd85615ced01;
- cu-preview.js:c3513d0a66fa19eeee4683b9399a61f8879138acd7c25e10327709b9f797e2c2;
- cu-preview.wgsl:37f5d96dd200b5b7744b00340d608b2fc44603e0cb1b7e2b0effd058f7b24294.

The CPU package guard accepts the exact reviewed data and rejects corrupted bytes, but WGSL compilation/manual lookup,
column reduction, rendered primary/bank/source-off modes, actual mirror/shadow coherence and device behavior remain
GPU gates. The current receiver diffuse term is still a declared constant fixture; it cannot close cloudy surface
irradiance. Primary-only ice dispatch is a design seam, not an exercised ice renderer in this liquid-only preview.
The retained master is a comparator/fallback foundation, not a new four-variant/mature-renewal/lifecycle family pass.
No conclusion here reverses the failed authored hierarchy appearance or supplies missing motion/full-sky evidence.
