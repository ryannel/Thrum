# Actual GPU query validation

| Quantity | Result |
| --- | ---: |
| all_trace_valid | True |
| max_incident_closure_relative_error | 0.0 |
| max_incident_physical_relative_error | 0.0 |
| max_gpu_view_S_relative_error_vs_1m_cached | 0.0 |
| max_gpu_view_T_absolute_error | 1.6414217959281885e-06 |
| max_gpu_composed_relative_error_vs_1m_cached | 0.0 |
| max_mirror_full_relative_error | 0.0 |

Actual GPU query traces; independent float64 coefficient/source solves. Actual GPU terminal and ambient isolate view/light-cache transport; no image or performance claim.

Per-ray records separate GPU versus1m view integration with the same actual light/ambient caches, 4m versus1m view quadrature, and cached versus direct fine Sun depth. Mirror records compare the separate secondary ray and compose camera-to-plane foreground exactly once.
