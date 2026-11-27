    // The cascades store derivatives in rest coordinates. The drawn horizontal
    // map compresses crests and stretches troughs; both normal and covariance
    // must be carried through the inverse of the summed horizontal Jacobian.
    let determinant=max(0.05,differential.x*differential.z-differential.y*differential.y);
    slope=vec2(slope.x*differential.z-slope.y*differential.y,slope.y*differential.x-slope.x*differential.y)/determinant;
    let a=differential.z;let b=-differential.y;let c=differential.x;
    covariance=vec3(
        a*a*covariance.x+2.0*a*b*covariance.y+b*b*covariance.z,
        a*b*covariance.x+(a*c+b*b)*covariance.y+b*c*covariance.z,
        b*b*covariance.x+2.0*b*c*covariance.y+c*c*covariance.z)/(determinant*determinant);
    if use_sheet_normal {
        // Interpolate normals of the displaced rest sheet before adding the
        // fine ripple slopes. The depth test still selects the nearest branch.
        let up = select(-surf_normal, surf_normal, surf_normal.y >= 0.0);
        slope += -up.xz / max(up.y, length(up) * 0.03 + 1e-20);
    }
    var whitewater=vec4(0.0);
    if FOAM_ENABLED {
        if SURF_CACHE_ENABLED && input.surf_weight > 0.0 {
            // The power input varies smoothly between the mesh nodes, but
            // limiting it at those nodes smears distant foam onto deep water.
            // Apply the same cap once here using the fragment's mapped bed.
            let cap = 0.15 * max(0.0, row.level - row.bed + row.swash);
            let long_loss = max(0.0, input.surf_normal_variance.w - cap * cap * input.surf_weight);
            row.loss = mix(row.loss, row.loss_without_long, input.surf_weight)
                + long_loss;
        }
        var face = input.breaking_face_weight;
        if SURF_SHORE_GRID_ENABLED {
            let distance = abs(input.world.xz - view.camera_xyz_time.xz);
            let blend = smoothstep(80.0, 140.0, max(distance.x, distance.y));
            if blend > 0.0 {
                face = mix(face, surf_shore_face(input.world.xz), blend);
            }
        }
        whitewater=foam_surface(input.world,row,max(length(dx),length(dy)),determinant,face);
        covariance*=1.0-0.65*whitewater.w;
    }
    let variance=max(0.0,covariance.x+covariance.z);
    let n=normalize(vec3(-slope.x,1.0,-slope.y));
    let v=normalize(view.camera_xyz_time.xyz-input.world);
    var screen_motion=(input.previous.xy/input.previous.w-input.current.xy/input.current.w)*vec2(0.5,-0.5)*view_aux.viewport_inv_viewport.xy;
    if water.level_dt_reset.z>0.5 {screen_motion=vec2(65504.0);}
    let foam_below = 1.0 - (1.0 - whitewater.y) * (1.0 - 0.65 * whitewater.x);
    let under = water_underside(input, n, v, variance, foam_below);
    if under.a == 1.0 {
        return Output(under, screen_motion, 1.0);
    }
    let nv=max(dot(n,v),0.001);
    // GGX's perceptual roughness is sqrt(alpha); alpha is the RMS unresolved slope.
    let roughness=clamp(pow(max(variance,0.000004),0.25),0.045,0.8);
    let reflected=reflected_sky(slope,covariance,v);
    let fresnel=clamp(reflected.w,0.0,1.0);
    // A dielectric interface splits the reflected and transmitted paths by R.
    // The environment lends its filtered radiance; its opaque-material DFG is not R.
    let paths=water_optical_paths(row,input.world,vec2<u32>(input.clip.xy),n,v,variance,reflected.rgb,fresnel);
    var light=paths.reflected;
    // Deep-water in-scatter, driven only by the shared hemispherical irradiance.
    let l=environment_sun_direction();
    let sunlight=water_sun(input.world);
    let incident=environment_diffuse(vec3(0.0,1.0,0.0))+sunlight*max(l.y,0.0);
    if OPTICS_ENABLED {
        light+=(1.0-fresnel)*(incident*water.scatter.rgb/3.14159265*(1.0-paths.transmission)
            + paths.transmission*paths.behind);
    } else {
        light+=(1.0-fresnel)*incident*water.scatter.rgb/3.14159265;
    }
    // Height in RMS-wave units and horizontal compression locate the thin
    // crest. Its extra in-scatter is bounded and uses the same lent irradiance
    // as the body; no independent light, pattern, or geometric sharpening.
    let crest_height=max(0.0,input.world.y-row.level)/max(water.scatter.w,0.001);
    let crest=1.0-exp(-crest_height*max(0.0,1.0-determinant));
    light+=(1.0-fresnel)*incident*water.scatter.rgb*crest;
    if OPTICS_ENABLED {
        // Forward scattering is concentrated toward the observer when the
        // thin, compressed part of the crest stands between lens and Sun.
        let forward = pow(max(dot(l, -v), 0.0), 8.0);
        let low_sun = 1.0 - smoothstep(0.14, 0.46, l.y);
        light += (1.0-fresnel) * sunlight * vec3(0.024, 0.11, 0.088)
            * (crest * forward * low_sun);
    }
    let h=normalize(l+v);let nh=max(dot(n,h),0.0);let nl=max(dot(n,l),0.0);
    let a2=pow(roughness,4.0);let d=a2/(3.14159265*pow(nh*nh*(a2-1.0)+1.0,2.0));
    let gv=2.0*nv/(nv+sqrt(a2+(1.0-a2)*nv*nv));
    let gl=2.0*nl/(nl+sqrt(a2+(1.0-a2)*nl*nl));
    let fh=water_fresnel(max(dot(v,h),0.0));
    if OPTICS_ENABLED {
        var glint = d*gv*gl*fh/(4.0*nv+0.00001);
        if hollow_surface {
            // A weak, wider lobe for the unresolved capillary slopes of a
            // puddle. Both lobes follow the resolved normal from the solver.
            let wide = max(a2, 0.0169);
            let wd = wide / (3.14159265 * pow(nh*nh*(wide-1.0)+1.0, 2.0));
            let wg_v = 2.0*nv / (nv + sqrt(wide + (1.0-wide)*nv*nv));
            let wg_l = 2.0*nl / (nl + sqrt(wide + (1.0-wide)*nl*nl));
            glint = mix(glint, wd*wg_v*wg_l*fh/(4.0*nv+0.00001), 0.36);
        }
        // The unresolved solar lobe keeps a graded peak even on glassy water.
        // Limit its response before lent irradiance so cloud shade stays linear.
        let peak = select(0.55, 0.25, hollow_surface);
        light += sunlight * (glint / (1.0 + glint / peak));
    } else {
        light+=sunlight*d*gv*gl*fh/(4.0*nv+0.00001);
    }
    let uv=input.clip.xy*view_aux.viewport_inv_viewport.zw;
    let grazing = grazing_body_path(input.world, v, uv, incident);
    light = mix(light, grazing.rgb, grazing.a);
    if FOAM_ENABLED {
        light+=incident*vec3(0.035,0.19,0.19)*whitewater.x*(1.0-fresnel);
        light=mix(light,incident*vec3(0.86,0.90,0.89)/3.14159265,whitewater.y);
    }
