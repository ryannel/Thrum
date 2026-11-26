    if FOAM_ENABLED { opacity=max(opacity,whitewater.y); }
    var reactive = clamp(fresnel + 0.1 + whitewater.y, 0.0, 1.0);
    if under.a > 0.0 {
        // Mix in premultiplied form and return straight alpha for the sheet's
        // blend state. At zero coverage this approaches the dry composition.
        let alpha = mix(opacity, 1.0, under.a);
        let radiance = mix(colour * opacity, under.rgb, under.a) / alpha;
        reactive = max(reactive, max(under.a, 4.0 * under.a * (1.0 - under.a)));
        return Output(vec4(radiance, alpha), screen_motion, reactive);
    }
    return Output(vec4(colour, opacity), screen_motion, reactive);
}
