let span = min(settings.swash_margin.w, track.z);
        let renewal = flow_renewal(point.position, settings.swash_margin.z, span);
        let phase_gradient = span * flow_phase_gradient(point.position);
        // The offset changes both departure Jacobians and the blend slope.
        let speed = point.level_current_bed.yz - track.xy;
        let phase_shear = mat2x2<f32>(speed * phase_gradient.x, speed * phase_gradient.y);
        if any(abs(speed) >= vec2(1e-6)) {
            gradient.blend = renewal.w * phase_gradient;
        }
        let reference = mat2x2<f32>(derivative.xy, derivative.zw);
        let current = mat2x2<f32>(point.current_gradient.xy, point.current_gradient.zw);
        let residual = current - reference;
        let base = COLUMN_IDENTITY - reference * settings.swash_margin.z - phase_shear;
