# R0 F3 embedded-identity index microprobe

3 November 2026. **Isolated cost evidence, not an admitted source profile.**
The [candidate render-source grammar](../../design/research/R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md)
requires a global typed UUID collision index separate from required-reference
count. This probe asks what a compact index and one straightforward JSON
extraction cost on dense, bounded render roots. It does not select a new
`EmbeddedIdentities` admission limit.

## Inputs and method

An isolated release Rust program generated candidate `thrum.render.scene@1`
roots with canonical distinct UUIDs, one camera/mesh/material, identity
instances, and static point lights. The mixed form gives 20% of `N` embedded
entries to instances (two typed mesh/material edges each) and 80% to lights;
the light-heavy form puts all `N` in lights. Every root also has one
active-camera edge. An affine permutation of IDs avoids measuring an already
sorted input. The program parses each record with `serde_json::Value`,
extracts compact 24-byte `(UUID, kind, owner)` IDs and 24-byte typed edges,
sorts the global ID vector, rejects duplicate IDs, and binary-searches every
edge for kind and same-root ownership. `overlap` retains an old sorted index
while rebuilding a second index from the same source bytes. Each repetition
is a fresh process; the table uses medians of three processes. Darwin
`getrusage` reports process maximum resident bytes.

Independent Python JSON traversal counted exactly `13N+66` value nodes in a
mixed root and `12N+66` in a light-heavy root. At the largest mixed case,
one record is 11,985,709 bytes / 975,066 nodes; at the largest light-heavy
case, 12,201,710 bytes / 996,066 nodes. Both fit the existing 16-MiB and
one-million-node record bounds. The two mixed roots have 60,002 typed edges,
under the current 65,536 aggregate required-edge bound; four light-heavy
roots have four edges. All roots are below the 4,096 declared-record bound.
The numerous lights exceed R0's **four-light recipe** and would be F4
unsupported, but their count is not an F3 source-grammar limit. These files
are not a complete `project.json`/scene/accepted-store candidate.

| Roots × entries/root | IDs | Edges | JSON MiB | Median peak RSS MiB | Parse ms | Extract ms | Sort ms | Resolve ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 × 1,000 mixed | 1,004 | 401 | 0.15 | 3.20 | 0.56 | 0.07 | 0.02 | 0.01 |
| 1 × 16,000 mixed | 16,004 | 6,401 | 2.44 | 22.83 | 7.66 | 1.35 | 0.34 | 0.18 |
| 1 × 32,000 mixed | 32,004 | 12,801 | 4.88 | 43.33 | 14.82 | 3.12 | 0.72 | 0.41 |
| 1 × 75,000 mixed | 75,004 | 30,001 | 11.43 | 101.06 | 34.34 | 7.74 | 1.78 | 1.13 |
| 2 × 75,000 mixed | 150,008 | 60,002 | 22.86 | 103.80 | 66.36 | 14.96 | 3.77 | 2.41 |
| 4 × 83,000 lights | 332,016 | 4 | 46.55 | 111.09 | 132.60 | 26.77 | 8.81 | <0.01 |
| 4 × 83,000 lights, old index held | 332,016 new | 4 | 46.55 | 118.73 | 124.39 | 26.55 | 8.81 | <0.01 |

The 332,016-ID case's index vector reserved 524,288 × 24 bytes, but only
committed pages contribute to RSS. Holding the old sorted index raised median
peak RSS by 7.64 MiB in this control. One 12-MB record's generic JSON tree
dominates the measured peak: one mixed root already reached 101.06 MiB.
This suggests a streaming typed summary decoder can materially reduce F3
peak, but this probe does **not** measure that implementation. The reported
times exclude fixture generation, complete strict validation, SQLite stage
or copy, exact triangle predicates, texture decoding, product cooking and
old/new accepted-store overlap. Repeated processes ran against a warm local
file cache; they are not cold-I/O or latency-tail guarantees.

Four prototype mutation controls terminated before success: duplicate IDs
within a root, a duplicate across roots, an active-camera edge to a mesh,
and an active-camera edge to another root's camera. The prototype uses
assertions, not production typed diagnostics. `serde_json::Value` does not
enforce the future grammar's duplicate-key/unknown-field rules and no actual
F3 parser was changed. The retained `mutate.py` deterministically regenerates
the four altered JSON inputs from the small generated fixtures and checks
their exact failure classes; the archive's `mutants.json` records those checks
without process-specific panic thread IDs.

## Capacity decision

Keep `EmbeddedIdentities` distinct from `RequiredReferences` and expose its
observed/allowed count in F3 refusal receipts. The experiment shows that a
compact in-memory index for 332,016 identities is plausible on this one
host and fixture shape; it does **not** justify selecting 332,016, 65,536 or
another numeric limit. Select the first profile count only after the shared
strict typed decoder, adversarial dense records with geometry/textures,
old/new pinned overlap, source-stage work and a target edit-latency budget
are measured together. If that exceeds the selected profile budget, use a
smaller measured profile or a bounded external sort with its own disk and
recovery account. Do not relabel nested containment as an edge to make the
existing reference cap appear sufficient.

## Reproduction and provenance

The selected [source, lock, mutation generator, reproduction notes, raw runs and mutant records](R0-F3-EMBEDDED-INDEX-PROBE-2026-11-03.tar.gz)
are retained in a small archive; generated large JSON fixtures are
regenerable and excluded. The source SHA-256 is recorded in the archive's
`SHA256SUMS`. Toolchain: Rust 1.99.1 (`58cf8f9c3`), `serde_json 1.0.154`,
macOS 26.6.2, Apple M3 Max, 36 GB RAM. No network lookup, Broadsides input,
renderer or device pipeline was used. The archive hash and extraction check
are: SHA-256 `ddf081c71fb383c31af6151467bb46807037d3dfe2d785903c86310e860d1a89`
(8.2 KiB). A fresh extraction listed the eight expected members and
`shasum -a 256 -c SHA256SUMS` passed for all seven protected files. Rebuilding
that extraction offline and running its `mutate.py` regenerated a byte-identical
`mutants.json`. The
standalone source SHA-256 is
`b3a7099389d66fd7fce53bb1f3c82b47d5b9ae8f6d866de32dc88289146c3cd6`.
