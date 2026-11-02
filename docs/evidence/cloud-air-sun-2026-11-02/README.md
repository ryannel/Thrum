# Paused Sun-only air experiment

2 November2026. **Preservation checkpoint, not adopted full atmosphere or a continued implementation goal.**
This work predates the migration-planning request. Source and receipts are retained so rework can compare or discard it
without losing the experiment. The [bounded profile](../../design/modules/CLOUD-AIR-PROFILE.md) owns its limited domain.

Loaded proof identity: `1a7e7fa5b0259f1e87c641adb372b09a2aad9f0525460fe13f880a58f854081d`.
`loaded-profile/` retains the actual source/fixtures from that build. The [finite GPU receipt](finite-gpu-probe.json)
and held [vacuum](held-vacuum.png)/[Sun-air](held-sun-air.png) frames were captured before the planning scope change;
no new rendering or GPU measurement was performed for this preservation commit.

The helper supplies RGB Sun-to-top transmittance in a bounded Earth-tangent frame. It does not supply sky radiance,
multiple-scattering air, finite view segments, local fog, clouded environment or full-sky composition. The two frames
support only a held direct-light comparison; there is no accepted motion, complete-domain, runtime-cost or user-quality
result. The UI's old generic `{}` probe summary is unfinished; inspect the actual receipt, not that placeholder.

Preservation checks rerun2November:10 existing Python air/reference/cooker tests and9 Node payload/resource/probe tests
passed. These are finite CPU/mock checks, not GPU certification. Earlier numerical probe scope stays in the raw receipt;
no family, integrated-air or budget capability is promoted. The new migration may retain the direct helper as a
comparator and replace its runtime role. Prior one-source cloud cost failure remains unchanged.
