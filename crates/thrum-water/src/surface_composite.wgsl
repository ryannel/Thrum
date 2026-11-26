    if FOAM_ENABLED { opacity=max(opacity,whitewater.y); }
    return Output(vec4(colour,opacity),screen_motion,clamp(fresnel+0.1+whitewater.y,0.0,1.0));
}
