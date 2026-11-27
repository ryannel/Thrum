// GPU-owned overlay classification, haze and encoding tags. Only mode is
// supplied by the host, before any chart or reader is recorded.
struct ChartLighting {
    mode: u32,
    cloudy: u32,
    haze: f32,
    classifications: u32,
}

struct ReflectionTags {
    tags: array<vec4<u32>, 2>,
}
