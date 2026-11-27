        out.wake = wake_at(point.position, min(point.carry.y, 0.0)) + point.dent;
        out.wake.x += max(point.carry.y, 0.0) * out.wake.w;
        if any(out.wake != vec4(0.0)) { out.body_flags.w |= 16u; }
