                if raw_ev < parameters.z { flags |= FLOOR | CLAMP; }
                if raw_ev > parameters.w { flags |= CLAMP; }
            }
        }
    }
    if policy.atmosphere!=0u && environment_light.horizontal_irradiance.w>1.0 { flags|=SKY_SH_NEGATIVE_PROBE; }
    if policy.pre_exposure < 0.0 { storage_scale = -policy.pre_exposure; }
    result.values = vec4(storage_scale, inverse, ev, lux);
    result.metadata = vec4(flags, serials.current_lo, serials.current_hi, policy.display_selectors);
}
