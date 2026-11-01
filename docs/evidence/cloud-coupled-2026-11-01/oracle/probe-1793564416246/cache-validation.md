# Actual GPU cache validation

Source density half parity: 0 mismatched voxels.

| Quantity | Maximum absolute error |
| --- | ---: |
| max_world_tau_error | 0.600103014 |
| max_world_T_error | 0.0285895771 |
| max_node_tau_error | 0.028130019 |
| max_node_T_error | 0.00767436339 |
| max_reference_quadrature_delta | 0.00311119949 |
| max_ambient_radiance_error | 1.99102654 |
| max_ambient_relative_error | 0.00990828641 |

Mip0 is actual GPU density; mip1/2 references recursively average2x2x2 and roundhalf. Standalone mip readbacks checked. Physical node and sampled-query errors are reported separately; no acceptance threshold inferred.

Ambient comparison uses exact32³ cache-node positions against1m/0.5m cloud integration, analytic atmosphere/fog paths and the declared five-valid-axis/6 horizon quadrature. GPU quadrature and half storage are both included in error; use the captured shader identity for its exact recipe.

These bounded samples establish neither full-domain maxima nor visual acceptance. The original captured identity and every input hash are retained in the JSON.
