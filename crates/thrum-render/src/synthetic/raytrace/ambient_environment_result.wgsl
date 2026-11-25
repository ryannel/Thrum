
    if traced == 1.0 { return diffuse+ao*environment*ess; }
    let combined = ao * traced;
    let specular = clamp(pow(nov + combined, exp2(-16.0 * r - 1.0)) - 1.0 + combined, 0.0, 1.0);
    return diffuse*traced + min(ao, specular)*environment*ess;
}
