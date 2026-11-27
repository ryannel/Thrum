            let screen = solve_horizontal(x.xz, y.xz, shift, mesh_det);
            let rest = input.rest + screen.x * rest_x + screen.y * rest_y;
            let column = input.world.xz + shift;
            let sample = surface_pixel(river_ownership_point(column, rest, transformed), input.body);
            if !sample.owned { break; }
            let raster = input.world.y + dot(mesh_gradient, screen);
            let datum = mesh_level + dot(mesh_gradient - retained_gradient, screen);
            let sheet = river_sheet_height(column, spacing, sample, raster, datum, transformed,
                river_refinement(column, surface_body(input.body), spacing));
            let residual = (sheet - input.world.y) - height;
            let error = abs(residual);
            // Keep the best owned intersection if the iteration crosses a
            // crease or runs out of steps on a grazing ray.
            if error < best_error {
                best_error = error;
                surface.rest = rest;
                surface.world = input.world + vec3(shift.x, height, shift.y);
                row = sample;
                let corrected = fma(m[2].zw, vec2(shift.y),
                    fma(m[1].zw, vec2(height), fma(m[0].zw, vec2(shift.x), clip.zw)));
                depth = clamp(corrected.x / corrected.y, 0.0, 1.0);
            }
