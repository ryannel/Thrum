# WATERS provenance

This is WATERS's record of what it copied or translated, from where and under what licence, and the one to
keep current. It was first copied from `PROVENANCE.md` in WATERS's working folder,
`.orchestrate/evidence/WATERS/`. The `design-proof/` files it names are kept in that working folder.

M1 copies or translates nothing from Broadsides or any other third-party source.
The CPU map, surface composition and tests were written for Thrum from the approved
[WATERS design](../plans/WATERS/DESIGN.md). The spectrum integrals and regular-wave
closed forms in the tests are independent mathematical oracles. Existing
`thrum-sea-model` evaluators are reused as a workspace dependency; its new helpers
expose band variances and amplitude scaling without changing its physical profile.

M1 adds no third-party dependencies, assets, GPL or share-alike material.

## M12

No file was copied or translated from Broadsides. The shore solver, exposure,
shoaling, section and sheet cut were written for Thrum from the approved design
and its first-party `design-proof/surf_view_proof.py`, `surf_revision_proof.py`
and `surf_cut_proof.py` reproductions. Those files remain the source of the
design examples, not a runtime dependency. New implementation lives in
`crates/thrum-water-model/src/{shore,section,cut,pyramid,surf}.rs`; the corresponding
tests and benchmark use analytic, unnamed geometry. `thrum-sea-model` only gains
a read-only choppiness accessor. No game preset or authored scenery is imported.

The existing permitted MIT/Apache-2.0 `divan = 0.1.21` is reused as a development
dependency for CPU benchmarks. Normal runtime dependencies remain CPU-only and
unchanged. No third-party code or assets, GPL or share-alike material are added.
