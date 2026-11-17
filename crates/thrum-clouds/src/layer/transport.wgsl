struct CloudTransport {scattering:vec3<f32>,transmittance:f32,optical_depth:f32}
fn cloud_removed(tau:f32)->f32 {
    return select(1.0-exp(-tau),tau*(1.0-tau*(0.5-tau/6.0)),tau<0.01);
}
fn cloud_beer(extinction:f32,source_per_m:vec3<f32>,distance:f32)->CloudTransport {
    let tau=extinction*distance;
    let transmittance=exp(-tau);
    // expm1's small-tau branch keeps thin edges and zero extinction finite.
    let removed=cloud_removed(tau);
    var integral=distance;
    if extinction>0.0 {integral=removed/extinction;}
    return CloudTransport(source_per_m*integral,transmittance,tau);
}
fn cloud_with_air(segment:CloudTransport,air_transmittance:vec3<f32>,air_scatter:vec3<f32>)->CloudTransport {
    return CloudTransport(air_transmittance*segment.scattering+cloud_removed(segment.optical_depth)*air_scatter,segment.transmittance,segment.optical_depth);
}
fn cloud_then(near:CloudTransport,far:CloudTransport)->CloudTransport {
    let optical_depth=near.optical_depth+far.optical_depth;
    return CloudTransport(near.scattering+near.transmittance*far.scattering,exp(-optical_depth),optical_depth);
}
