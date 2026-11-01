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

Ambient physical-error checks apply only within the declared local support [112.0, 32.0, 80.0, 400.0, 256.0, 432.0]; outside zeros are intentionally inactive.

Baseline full-array and retained-ambient parity:

```json
{
  "density": {
    "mismatched_half_values": 0,
    "max_absolute_error": 0.0
  },
  "mip1": {
    "mismatched_half_values": 0,
    "max_absolute_error": 0.0
  },
  "mip2": {
    "mismatched_half_values": 0,
    "max_absolute_error": 0.0
  },
  "prefix": {
    "mismatched_half_values": 0,
    "max_absolute_error": 0.0
  },
  "ambient": {
    "retained_nodes": 5544,
    "inactive_nodes": 27224,
    "retained_mismatched_half_values": 0,
    "retained_max_absolute_error": 0.0,
    "inactive_nonzero_half_values": 0,
    "baseline_path": "docs/evidence/cloud-coupled-2026-11-01/probes/probe-1793564729432",
    "baseline_ambient_sha256": "4e4d8c2aebaa1969acb4cf1339b4af1afd5f05f920eb2b9bff10820bbb434186"
  }
}
```
