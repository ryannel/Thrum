        let renewal = cascade_renewal(settings.swash_margin.z, min(settings.swash_margin.w, track.z));
        let reference = mat2x2<f32>(derivative.xy, derivative.zw);
        let current = mat2x2<f32>(point.current_gradient.xy, point.current_gradient.zw);
        let residual = current - reference;
        let base = COLUMN_IDENTITY - reference * settings.swash_margin.z;
