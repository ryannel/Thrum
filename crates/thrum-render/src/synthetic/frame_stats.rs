//! Renderer-owned interpretation of the bounded scene timestamp layout.

use super::PreparedSynthetic;

pub(crate) fn pass_durations(
    passes: &[crate::timing::PassTiming],
    presented: bool,
    prepared: &PreparedSynthetic,
) -> Option<([f64; 11], Option<f64>)> {
    const ALL: [&str; 6] = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic cluster build",
        "synthetic forward opaque",
        "synthetic transform history",
        "synthetic tone encode",
    ];
    const SHADOW_ONLY: [&str; 5] = [ALL[0], ALL[1], ALL[3], ALL[4], ALL[5]];
    const CLUSTER_ONLY: [&str; 4] = [ALL[2], ALL[3], ALL[4], ALL[5]];
    let shadows = prepared.shadowed();
    let clusters = prepared.gpu_clustered();
    let names: &[&str] = match (shadows, clusters) {
        (true, true) => &ALL,
        (true, false) => &SHADOW_ONLY,
        (false, true) => &CLUSTER_ONLY,
        (false, false) => &ALL[3..],
    };
    let mut atmosphere = [0.0; 5];
    let mut offset = 0;
    for (index, name) in [
        "atmosphere transmittance",
        "atmosphere multiple scattering",
        "atmosphere sky view",
        "atmosphere diffuse irradiance",
        "atmosphere aerial perspective",
    ]
    .into_iter()
    .enumerate()
    {
        if passes
            .get(offset)
            .is_some_and(|pass| pass.name.as_ref() == name)
        {
            atmosphere[index] = passes[offset].nanoseconds;
            offset += 1;
        }
    }
    (passes.len() == offset + names.len() + usize::from(presented)
        && passes[offset..]
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == *name)
        && (!presented || passes[offset + names.len()].name.as_ref() == crate::present::PASS_NAME))
        .then(|| {
            let mut scene = [0.0; 11];
            scene[6..].copy_from_slice(&atmosphere);
            for pass in passes[offset..].iter().take(names.len()) {
                let slot = match pass.name.as_ref() {
                    "synthetic Sun shadow near" => 0,
                    "synthetic Sun shadow far" => 1,
                    "synthetic cluster build" => 2,
                    "synthetic forward opaque" => 3,
                    "synthetic transform history" => 4,
                    _ => 5,
                };
                scene[slot] = pass.nanoseconds;
            }
            (
                scene,
                presented.then(|| passes[offset + names.len()].nanoseconds),
            )
        })
}
