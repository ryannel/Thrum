# M1 phase measurements

On this M3 Max, `cargo test --locked --offline -p thrum-cloud-model --test
sun_disc_phase -- --nocapture` measures the phase, with a 0.2667° solar radius.
The retained test uses independent sphere quadrature (3,000 midpoint samples in
u = sin(theta/2)^(1/2) for the model, 100,000 for the table) and an independent
100,000-ring midpoint integral of the point phase for the disc centre.

| Diameter µm | Model integral | f32 table integral | Table mean cosine | Closed form mean | Centre sr⁻¹ | Largest relative error at 0°, 0.25°, 0.5°, 2° |
| --- | --- | --- | --- | --- | --- | --- |
| 5 | 0.999999957 | 1.000019456 | 0.843421093 | 0.843419504 | 99.320564 | 0.000051395 |
| 20 | 0.999999968 | 1.000078458 | 0.879313898 | 0.879305631 | 1791.801300 | 0.000386029 |
| 50 | 0.999999972 | 1.000133762 | 0.891202250 | 0.891188808 | 4323.680849 | 0.001400534 |

Measured first, then asserted: absolute integral/mean error ≤ 0.00015, relative
forward-angle error ≤ 0.002. The table has 1,025 f32 entries (4,100 bytes), the
same count as design proof part G. It is linear in u, with interpolation performed
entirely in f32 from sin²(theta/2). A 0.37/0.63 blend of the 5/50 µm tables also
passes the energy bound. The model disc quadrature uses 24 Gauss radial nodes
and 48 azimuth nodes; its energy/mean bound is 0.000002 and its centre relative
bound is 0.0000002. These tolerances reject both the energy-losing widened lobe
and the point phase at the centre. No table renormalisation hides either error.

Source revision and final verification are recorded in [M1](M1.md).
