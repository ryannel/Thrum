# Supplemental simulator inspection plan

Written before viewing the reviewer capture. M2 has no `inspect` proof; this supplements M2.2 because the automated script checks state and presentation progress but only saves its screenshot.

- Confirm the running diagnostic scene contains the ground/grid and foreground cubes rather than a blank or fault frame.
- Match the capture to the current checkout and the requested Occlusion / resolved Direct / MissingIndirectExecution report.
- Check the actual Direct draw path submits all active instances; distinguish visual corroboration from pixel equality or a physical-device performance claim.

The script does not exercise rotation or background/foreground; assess that coverage separately rather than infer it from presentation counters.
