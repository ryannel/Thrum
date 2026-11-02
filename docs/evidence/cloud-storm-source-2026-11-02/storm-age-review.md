# Cb mixed-phase birth and age closure

2 November 2026. Bounded read-only review of the diagnostic correction in `shaders/source.wgsl` and the added `src/gpu-reference.js` fixtures. This is a source-accounting review, not a new visual or performance receipt.

## Disposition

The correction is sound and closes the reported scalar-decay phantom-birth defect. Let `a_j=max(q_adv,j,0)`, and let the actual liquid/ice decay rates be `D_j=phaseDecay[j]`. With local supply off, the exponential reaction returns `r_j=a_j exp(-D_j dt)`. The shader now computes `survived_j` from exactly that expression. Cb freezing then applies `(-δ,+δ)` to the two channels in the same cell, with `δ<=q_l` and `δ<=1-q_i`, so it does not change their sum. Therefore `total_after - sum(survived)=0` in the real-valued source-off case, including unequal phase losses. The former scalar `decay` could understate the longer-lived ice survivor and falsely report the difference as new material.

For positive source, `amountAdded=max(total_after-sum(survived),0)` is the net post-reaction source contribution; freeze does not alter it. The age update then weights previous transported age by surviving total and gives new contribution age `dt`, yielding `previousAge+dt` when `amountAdded=0` and `total>.002`. This is consistent with the intended diagnostic semantics. It does not make age a conserved physical moment: `previousAge` comes from first-order forward transport while q uses limited MacCormack, and cells with `total<=.002` deliberately reset age to zero. Those limits do not affect q or the source-off birth count.

## Fixture scope and receipt boundary

`mixedPhaseNoPhantomBirth` uploads `q_l=q_i=.2`, age100, dry .7 and lift0, then checks output `w≈0`; `mixedPhaseAgeWithoutBirth` checks age100.25 within one RGBA16F age step. This directly exercises unequal phase decay, which caused the defect. Its 5.0–5.4 km test slab is below the 9.6 km freeze band, so the fixture does not itself combine active freezing and age. The separate `phaseTransferConservesProxy` fixture checks paired transfer with q_sum. Together the equations and separate GPU cases support the fix; a later cap-band mixed-phase no-source fixture would guard against a future transfer change, but is not a blocker to this correction.

The parent is rerunning the GPU fixtures and 480→600 s shutdown observation. Until those receipts return, this review establishes source-code correctness only. A zero reported birth count after shutdown must still be distinguished from any material introduced by pulses before the source was turned off.
