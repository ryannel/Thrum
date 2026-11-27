}
@group(0) @binding(0) var<uniform> settings: Settings;
@group(0) @binding(2) var<storage, read> components: array<Component>;
@group(0) @binding(3) var<storage, read_write> answers: array<Answer>;
@group(0) @binding(4) var cascades: texture_2d_array<f32>;

fn column_component(index: u32) -> Component { return components[index]; }

struct QueryCarriage {
    motion: CascadeMotion,
    gradient: CarriageGradient,
}

fn carriage_gradient(point: QueryPoint) -> CarriageGradient {
    var gradient = CarriageGradient(COLUMN_IDENTITY, COLUMN_IDENTITY, vec2(0.0));
    let track = point.track;
    let derivative = point.track_gradient;
    if track.w > 0.0 {
        let axis = u32(track.w) - 1u;
        gradient.first[0][axis] = derivative.x;
        gradient.first[1][axis] = derivative.z;
        gradient.blend = derivative.yw;
    } else if track.w < 0.0 {
        let renewal = cascade_renewal(settings.swash_margin.z, min(settings.swash_margin.w, track.z));
        let reference = mat2x2<f32>(derivative.xy, derivative.zw);
        let current = mat2x2<f32>(point.current_gradient.xy, point.current_gradient.zw);
        let residual = current - reference;
        let base = COLUMN_IDENTITY - reference * settings.swash_margin.z;
        gradient.first = base - residual * renewal.x;
        gradient.second = base - residual * renewal.y;
        if all(abs(point.level_current_bed.yz - track.xy) < vec2(1e-6)) {
            // The two carried places coincide here. Keep their blended
            // derivative, since the residual may grow on either side.
            gradient.first = base - residual * mix(renewal.y, renewal.x, renewal.z);
        }
    }
    return gradient;
}

fn local_query_carriage(point: QueryPoint) -> QueryCarriage {
    let motion = cascade_motion(point.position, point.level_current_bed.yz, point.track,
        settings.swash_margin.z, settings.swash_margin.w);
    return QueryCarriage(motion, carriage_gradient(point));
}

fn cascade_wave(point: QueryPoint, motion: CascadeMotion, band: u32, gradient: CarriageGradient) -> CarriedColumn {
    let extent = settings.lengths_chop[band];
    let lod = clamp(log2(max(1.0, point.radius * 512.0 / extent)), 0.0, 8.0);
    return carried_column(motion, extent, band, lod, gradient);
}

struct QueryWave { wave: Wave, cut_fine_nodes: u32, cut_coarse_nodes: u32, }

const QUERY_WIDTH = 64u;

