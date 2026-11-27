            if i > 0u && (residual < 0.0) != (previous_residual < 0.0) {
                bracketed = true;
                opposite_height = previous_height;
                opposite_residual = previous_residual;
            }
            var next = height + residual;
            if bracketed {
                // A secant must stay within the crossing. Bisect when the
                // secant crowds either endpoint so the bracket keeps shrinking.
                let fraction = residual / (residual - opposite_residual);
                next = mix(height, opposite_height, select(0.5, fraction,
                    fraction >= 0.1 && fraction <= 0.9));
            } else if i > 0u && abs(previous_residual - residual) > 0.000002 {
                // At bank angles a short wave can rise faster than the ray.
                // Keep the sign of the measured slope through those crests.
                let scale = (height - previous_height) / (previous_residual - residual);
                next = height + residual * clamp(scale, -4.0, 4.0);
            }
            previous_height = height;
            previous_residual = residual;
            height = next;
        }
    }
    let shaded = shade_water(surface, row);
    return RiverOutput(shaded.colour, shaded.motion, shaded.reactive, depth);
}
