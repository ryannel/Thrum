# Actual GPU query validation

| Quantity | Result |
| --- | ---: |
| all_trace_valid | True |
| max_incident_closure_relative_error | 4.0447913756542496e-07 |
| max_incident_physical_relative_error | 0.45107930614568076 |
| max_gpu_view_S_relative_error_vs_1m_cached | 0.0012030322926606149 |
| max_gpu_view_T_absolute_error | 3.359572577738912e-05 |
| max_gpu_composed_relative_error_vs_1m_cached | 0.0012029622451962567 |
| max_mirror_full_relative_error | 5.2091441345493544e-05 |

Actual GPU query traces; independent float64 coefficient/source solves. Actual GPU terminal and ambient isolate view/light-cache transport; no image or performance claim.

Per-ray records separate GPU versus1m view integration with the same actual light/ambient caches, 4m versus1m view quadrature, and cached versus direct fine Sun depth. Mirror records compare the separate secondary ray and compose camera-to-plane foreground exactly once.
