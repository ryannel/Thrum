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
