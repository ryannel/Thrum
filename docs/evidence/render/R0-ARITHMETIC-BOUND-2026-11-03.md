# R0 light/storage bound: analytical falsifier and numeric check, 3 November 2026

This is supporting evidence for the [candidate R0 arithmetic envelope](../../design/research/R0-ARITHMETIC-ENVELOPE-2026-11-03.md),
developed from repository revision `2c2eae1`. The dependency-free
[Python oracle](R0-ARITHMETIC-BOUND-ORACLE.py) has SHA-256
`a72c6fd24c6b09075dce9f16616edf7e181f09f2acdc46159be1b80f993b524b`.
Run with Python 3.11 or newer:

```sh
python3 docs/evidence/render/R0-ARITHMETIC-BOUND-ORACLE.py
```

Observed output:

```text
normal D=77624.7245685; dielectric stored=75805.40; metal stored=1895134.88
illustrative fixture stored bound=22552.86; randomized max/analytical bound=0.962133; 100000 samples
```

The f64 evaluator independently computes the full GGX/Smith/Schlick normal
highlight and verifies the half-float overflow witness using Python's half
conversion. It checks the sufficient per-channel bound for the explicitly
illustrative fixture. A seeded 100,000-sample hemisphere search compares the
full expression with the derived inequality to catch transcription errors;
the analytical inequality, not the random search, covers all real-number
angles under its stated domains. No shader, texture, device, output image or
performance measurement was run. WGSL reassociation, geometry/radius
certification, actual authored source extrema, f32 operation bounds and f16
error tolerance remain open.
