        slope += out.wake.yz;
        rate += out.wake.w;
        velocity.y += out.wake.w;
        out.height_slope_rate = vec4(height, slope, rate);
        out.velocity_depth = vec4(velocity, depth);
        out.body_flags.y = u32((point.flags & 1u) == 0u || height > row.w);
        for (var layer = 0u; layer < 4u; layer++) {
            if gains[layer] * settings.heights[layer] > 0.0 { out.body_flags.w |= 1u << layer; }
        }
    }
    answers[index] = out;
}
