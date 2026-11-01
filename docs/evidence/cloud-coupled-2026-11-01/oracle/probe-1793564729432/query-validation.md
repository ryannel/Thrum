# Actual GPU query validation

| Quantity | Result |
| --- | ---: |
| all_trace_valid | True |
| max_incident_closure_relative_error | 4.0447913756542496e-07 |
| max_incident_physical_relative_error | 0.45107930614568076 |
| max_gpu_view_S_relative_error_vs_1m_cached | 0.01293293605100209 |
| max_gpu_view_T_absolute_error | 1.6414217959281885e-06 |
| max_gpu_composed_relative_error_vs_1m_cached | 0.006196374136285862 |
| max_mirror_full_relative_error | 0.0007527208110534382 |

Actual GPU query traces; independent float64 coefficient/source solves. Actual GPU terminal and ambient isolate view/light-cache transport; no image or performance claim.

Per-ray records separate GPU versus1m view integration with the same actual light/ambient caches, 4m versus1m view quadrature, and cached versus direct fine Sun depth. Mirror records compare the separate secondary ray and compose camera-to-plane foreground exactly once.

Baseline query parity:

```json
{
  "baseline": "tmp/cloud-coupled-proof/captures/probe-1793564416246",
  "queries": [
    {
      "name": "dense",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": true,
      "ambient_difference": [
        0.0,
        0.0,
        0.0
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "edge",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": true,
      "ambient_difference": [
        0.0,
        0.0,
        0.0
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "receiver",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "receiver2",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1285.400390625,
        -2094.7265625,
        -3334.9609375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "outside",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1427.001953125,
        -2325.439453125,
        -3706.0546875
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "occluded",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        0.0,
        0.0,
        0.0
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "main-0.5,0.36",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "main-0.5,0.42",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "main-0.53,0.62",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "mirror-secondary-0.53,0.62",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1412.353515625,
        -2301.025390625,
        -3666.9921875
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "main-0.5,0.85",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "mirror-secondary-0.5,0.85",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "main-0.7,0.85",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    },
    {
      "name": "mirror-secondary-0.7,0.85",
      "S_T_terminal_composed_mismatches": 0,
      "S_T_terminal_composed_max_error": 0.0,
      "common_incident_prefix_sigma_mismatches": 0,
      "ambient_required": false,
      "ambient_difference": [
        -1466.064453125,
        -2388.916015625,
        -3806.15234375
      ],
      "note": "Ambient at zero cloud coefficient is not consumed by transport."
    }
  ]
}
```
