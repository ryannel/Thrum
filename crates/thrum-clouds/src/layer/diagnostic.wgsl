// Test observer only: evaluate the actual trace functions along its centre ray.
// Eight vec4s per occupied sample; the first eight vec4s describe the frame.
@group(3) @binding(0) var<storage,read_write> observation:array<vec4<f32>>;
@compute @workgroup_size(1)
fn observe() {
    let camera=atmosphere.camera_altitude.xyz;
    let clip=cloud.inverse_raster*vec4(0.0,0.0,0.0,1.0);
    let ray=normalize(clip.xyz/clip.w-camera);
    let air_camera=camera-vec3(cloud.drift.x,0.0,cloud.drift.y);
    let sun=atmosphere.sun_lux.xyz;
    let phase=cloud_phases(ray,sun);
    let light_angles=cloud_light_angles(-ray);
    let sun_alignment=dot(ray,mapping.path_border.xyz);
    observation[0]=vec4(camera,environment_storage_scale());
    observation[1]=vec4(ray,atmosphere.sun_lux.w);
    observation[2]=vec4(sun,cloud.erosion.z);
    observation[3]=vec4(environment_sky_radiance(ray),cloud.heights.z);
    observation[4]=cloud_ambient[0];observation[5]=cloud_ambient[1];
    observation[6]=vec4(phase.xy,cloud.scattering.xy);
    let sun_base=sun_irradiance(vec3(camera.x,cloud.heights.x,camera.z));
    let sun_top=sun_irradiance(vec3(camera.x,cloud.heights.y,camera.z));
    var distance=max(0.0,(cloud.heights.x-camera.y)/ray.y);
    let end=min(cloud.heights.w,(cloud.heights.y-camera.y)/ray.y);
    var count=0u;
    var transmission=1.0;
    for(var i=0u;i<768u && distance<end && transmission>0.001;i++) {
        let step=min(end-distance,max(cloud.heights.z,distance*0.002));
        let at=distance+step*0.5;
        let point=air_camera+ray*at;
        let column=cloud_column(point);
        let sigma=cloud_density(point,at/f32(textureDimensions(transport).y));
        if sigma>0.0 {
            let world=camera+ray*at;
            let sun_depths=cloud_local_sun(point,world,at,at/f32(textureDimensions(transport).y),sun_alignment,false);
            let solar=mix(sun_base,sun_top,clamp((point.y-cloud.heights.x)/(cloud.heights.y-cloud.heights.x),0.0,1.0));
            let source=cloud_source(solar,phase,sun_depths,column.yz,light_angles,at,transmission);
            let segment=cloud_beer(sigma,sigma*source,step);
            let air=environment_aerial(world,vec2(0.5));
            let offset=8u+count*8u;
            observation[offset]=vec4(world,at);
            observation[offset+1u]=vec4(sigma,sun_depths.x,step,transmission);
            observation[offset+2u]=column;
            observation[offset+3u]=vec4(solar,sun_depths.y);
            observation[offset+4u]=vec4(source,0.0);
            observation[offset+5u]=vec4(segment.scattering,segment.transmittance);
            observation[offset+6u]=vec4(air.transmission,0.0);
            observation[offset+7u]=vec4(air.scatter,0.0);
            transmission*=segment.transmittance;
            count++;
        }
        distance+=step;
    }
    observation[7]=vec4(f32(count),cloud.heights.xy,cloud.physical.w);
    // Independent high-resolution sky quadrature inputs; no hemisphere kernel.
    for(var hemisphere=0u;hemisphere<2u;hemisphere++) {
        for(var y=0u;y<64u;y++) {for(var x=0u;x<128u;x++) {
            let mu=(f32(y)+0.5)/64.0;
            let phi=(f32(x)+0.5)*(2.0*PI/128.0);
            let r=sqrt(1.0-mu*mu);
            let direction=vec3(r*cos(phi),select(mu,-mu,hemisphere==1u),r*sin(phi));
            observation[6152u+hemisphere*8192u+y*128u+x]=vec4(environment_sky_radiance(direction),mu);
        }}
    }
}
