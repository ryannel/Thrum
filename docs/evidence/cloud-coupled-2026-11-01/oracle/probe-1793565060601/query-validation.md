# Actual GPU query validation

| Quantity | Result |
| --- | ---: |
| all_trace_valid | True |
| max_incident_closure_relative_error | 4.0447913756542496e-07 |
| max_incident_physical_relative_error | 0.45107930614568076 |
| max_gpu_view_S_relative_error_vs_1m_cached | 0.01293293605100209 |
| max_gpu_view_T_absolute_error | 1.8457590078568842e-06 |
| max_gpu_composed_relative_error_vs_1m_cached | 0.006196374136285862 |
| max_mirror_full_relative_error | 0.0007527208110534382 |

Actual GPU query traces; independent float64 coefficient/source solves. Actual GPU terminal and ambient isolate view/light-cache transport; no image or performance claim.

Per-ray records separate GPU versus1m view integration with the same actual light/ambient caches, 4m versus1m view quadrature, and cached versus direct fine Sun depth. Mirror records compare the separate secondary ray and compose camera-to-plane foreground exactly once.
