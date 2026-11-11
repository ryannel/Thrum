# M5 target evidence

11 November 2026. M5.1–M5.6 and M5.9–M5.10 pass on the reconciled source `1948d7ab`. M5 remains open while M5.7–M5.8 await the required 1× motion review. IINA access is confirmed in the resumed session, and the motion review is queued under the shared GPU lock. [Access record](m5/records/review-access.json).

| Item | Target proof |
|---|---|
| M5.1 | Metal passes identity ownership, profile replacement, actual device replacement, time scrub and true/apparent Sun continuity. [Repository check](m5/records/rebased/check-05.log.gz) |
| M5.2 | Contribution-owned Metal pipelines pass H1–H5, night and exposure 2 against independent references. Known fields assert ≥32 stored-step contrast and reject the wrong lookups. The 257-position precision record has zero samples outside `C/32 + 1e-5 M + Q`. Preserved P1–P11 retain their references, cases and bounds. [Handoff measurements](m5/records/rebased/metal-handoff.log.gz) |
| M5.3 | Hardware Chrome passes H1–H5 with independent browser references, live and known-field filtered decode, exposure 2 and validation scopes. H2 rejects 216 wrong lookups and H4 rejects 486. [Rows](m5/records/rebased/chrome-environment.json.gz), [hardware identity](m5/records/rebased/chrome-browser-gpu.json.gz) |
| M5.4 | Six Metal route stills match base/current/rerun byte for byte. Forward and cooked-volume source text match the base. The identical comparator rejects a one-byte image perturbation. [Metal parity](m5/records/rebased/metal-parity.json.gz) |
| M5.5 | Six Chrome engine-tap views match base/current/rerun in one browser process and cooked store. A rendered wrong-camera control differs. [Chrome parity](m5/records/rebased/chrome-parity.json.gz) |
| M5.6 | The shared production surface shade and cooked march pass zero, ln2, spatial and clear-support cases. Positive sky/emission and geometric shadow remain intact. The volume reference evaluates each march sample’s own world point. [Repository check](m5/records/rebased/check-05.log.gz) |
| M5.7–M5.8 | Whole-frame and native-size inspections are recorded for the completed views. The remaining captures and 1× review are queued. These items remain open. [Inspection record](m5/records/visual-inspection.json) |
| M5.9 | Mac OFF/ON/ON/OFF forward GPU medians are 1.1599, 1.2041, 1.2011 and 1.1740 ms. Whole-frame GPU medians including the optional surface blit are 3.8604, 3.9946, 3.9592 and 4.3120 ms. [Samples, builds and load](m5/records/rebased/metal-cost.json.gz), [CPU wall and submission intervals](m5/records/rebased/metal-cost-summary.json) |
| M5.10 | Chrome’s same-order RAF medians are 16.7 ms in each arm. Created synthetic/cooked-mesh/volume pipelines use 11/11/8 sampled textures off and 12/12/9 on, within the enabled limit of 16 on both targets. [Chrome costs and census](m5/records/rebased/chrome-cost.json.gz) |

Costs are observations on this M3 Max with the engine observer enabled. The records retain configuration, background load, console and permission observations. [Command records](m5/records/rebased/commands.json).

The parity base is `ba9549f5`, recorded after concurrent APPLE-PRESENTATION M1 integrated its motion inputs. Both fresh arms include those inputs. The original `a471fe5d` base remains preserved. [Current base](m5/base.json), [initial base](m5-h2/base.json).

The opt-in diagnostic uses a 1024² fp16 field on a 2000 m plane with `[160,40]` m/s world drift. Its analytic shape is a stand-in. A westward pan supplements the noon and 17.3 h routes for the ridge-shadow view. [West route](m5/west-route.json).
