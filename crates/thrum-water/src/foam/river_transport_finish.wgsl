            var drift = row.row.water.yz + 0.027 * wind;
            let swash = min(map.swash.x * length(energy_before), map.origin_spacing_margin.w);
            if level == 0u && row.has_bed && swash > 0.0 && depth < 0.27 * swash {
                let thickness_slope = column.slope + wake.yz - swash_bed_slope(map_cell(point));
                let plain = swash_drift(thickness_slope, column.velocity.y + wake.w, depth, swash);
                drift += shore_uprush(level, local, plain);
            }
            let max_shift = select(0.999, 2.0, level == 0u && dt > 0.024);
            let shift = clamp(drift * dt / spacing, vec2(-max_shift), vec2(max_shift));
            let diffusion = min(0.24, (0.016 + 0.14 * stocks.x + 0.30 * stocks.y) * dt / (spacing * spacing));
            movement = vec4(shift, diffusion, shore.x + river_birth);
        } else if level == 0u && stranded_shore(local) {
            // Air and film collapse as the stranded bubbles drain on exposed sand.
            stocks = vec4(stocks.xyz * exp(-dt / 1.8), stocks.w);
        }
    }
    parcels[foam_index(0u, level, local)] = Parcel(stocks, movement);
}
