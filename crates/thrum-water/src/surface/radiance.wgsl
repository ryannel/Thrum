    var light=paths.reflected;
    // Deep-water in-scatter, driven only by the shared hemispherical irradiance.
    let l=environment_sun_direction();
    let sunlight=water_sun(input.world);
    let incident=environment_diffuse(vec3(0.0,1.0,0.0))+sunlight*max(l.y,0.0);
    let low_sun = 1.0 - smoothstep(0.14, 0.46, l.y);
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
    var crest=1.0-exp(-crest_height*max(0.0,1.0-determinant));
    if hollow_surface {
        // Interaction waves displace vertically and do not contribute to
        // the horizontal Jacobian. Use their resolved positive relief for
        // the shallow crest's broad in-scatter at grazing light.
        crest += 0.42 * low_sun * smoothstep(0.0003, 0.003, wake.x);
    }
    light+=(1.0-fresnel)*incident*water.scatter.rgb*crest;
    if OPTICS_ENABLED {
        // Forward scattering is concentrated toward the observer when the
        // thin, compressed part of the crest stands between lens and Sun.
        let forward = pow(max(dot(l, -v), 0.0), 8.0);
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
            // Broaden the capillary lobe at grazing illumination so it
            // resolves the curved front over several pixels.
            let wide = max(a2, mix(0.0169, 0.0324, low_sun));
            let wd = wide / (3.14159265 * pow(nh*nh*(wide-1.0)+1.0, 2.0));
            let wg_v = 2.0*nv / (nv + sqrt(wide + (1.0-wide)*nv*nv));
            let wg_l = 2.0*nl / (nl + sqrt(wide + (1.0-wide)*nl*nl));
            glint = mix(glint, wd*wg_v*wg_l*fh/(4.0*nv+0.00001), mix(0.36, 0.70, low_sun));
        }
        // The unresolved solar lobe keeps a graded peak even on glassy water.
        // Limit its response before lent irradiance so cloud shade stays linear.
        let peak = select(0.55, mix(0.25, 0.0065, low_sun), hollow_surface);
        light += sunlight * (glint / (1.0 + glint / peak));
    } else {
        light+=sunlight*d*gv*gl*fh/(4.0*nv+0.00001);
    }
    let uv=input.clip.xy*view_aux.viewport_inv_viewport.zw;
    if FOAM_ENABLED {
        light+=incident*vec3(0.035,0.19,0.19)*whitewater.x*(1.0-fresnel);
        light=mix(light,incident*vec3(0.86,0.90,0.89)/3.14159265,whitewater.y);
    }
