/** Finite fit receipts, called by the quiescent viewer after a matching render.
 * No source stepping or product writes. A receipt is not a family/performance pass.
 */
import {compileShader,readTexture,hash} from './gpu.js';
const TOLERANCE=3e-5;

function requireValue(condition,message){if(!condition)throw Error('Scatter probe: '+message);}
function byteView(input){return input instanceof Uint8Array?input:new Uint8Array(input);}
function decodeFloat32(input){
  const bytes=byteView(input),view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  requireValue(bytes.byteLength%4===0,'non-f32 payload');
  return Array.from({length:bytes.byteLength/4},(_,i)=>view.getFloat32(i*4,true));
}
function lower(axis,value){let lo=0,hi=axis.length-1;while(hi-lo>1){const mid=(lo+hi)>>1;if(axis[mid]<=value)lo=mid;else hi=mid;}return lo;}
function tableReader(fit){
  const profile=fit.profile,nt=profile.counts.T,nm=profile.counts.mu;
  const axes=decodeFloat32(fit.recovery.axesBytes),values=decodeFloat32(fit.recovery.aBytes);
  requireValue(axes.length===nt+nm&&values.length===nt*nm,'table shape');
  const t=axes.slice(0,nt),m=axes.slice(nt);
  return {t,m,values,lookup(thickness,mu){
    requireValue(Number.isFinite(thickness)&&thickness>=0&&Number.isFinite(mu)&&mu>=0&&mu<=1,'CPU lookup input domain');
    if(thickness<profile.evaluation.T_below)return 0;
    const tc=Math.min(thickness,profile.evaluation.T_above),ti=lower(t,tc),mi=lower(m,mu);
    const ft=(tc-t[ti])/(t[ti+1]-t[ti]),fm=(mu-m[mi])/(m[mi+1]-m[mi]);
    const a=values[mi*nt+ti]*(1-ft)+values[mi*nt+ti+1]*ft;
    const b=values[(mi+1)*nt+ti]*(1-ft)+values[(mi+1)*nt+ti+1]*ft;
    return a*(1-fm)+b*fm;
  }};
}
function stats(rows,key){
  const values=rows.map(row=>row[key]).filter(Number.isFinite);
  return {sample_count:rows.length,finite_error_count:values.length,
    max_absolute_error:values.length?Math.max(...values):null,
    mean_absolute_error:values.length?values.reduce((a,b)=>a+b,0)/values.length:null};
}
function checkedSummary(rows,key){
  const summary=stats(rows,key);return {...summary,tolerance:TOLERANCE,
    pass:summary.finite_error_count===rows.length&&rows.length>0&&rows.every(row=>row.valid)&&summary.max_absolute_error<=TOLERANCE};
}
function lutCases(table){
  const rows=[];
  const add=(T,mu,kind)=>rows.push({requested_T:T,requested_mu:mu,T:Math.fround(T),mu:Math.fround(mu),kind});
  for(const T of [0,1e-4-1e-10,1e-4,1e-4+1e-10,.1,1,2,5,30,1000,1001,1e6]){
    add(T,0,'grazing/boundary');add(T,1,'normal/boundary');
  }
  for(const degrees of [2,5,15,30,75,90])add(2,Math.sin(degrees*Math.PI/180),'required-Sun');
  for(const i of [0,1,Math.floor(table.t.length/4),Math.floor(table.t.length/2),table.t.length-2]){
    const j=Math.min(table.m.length-2,Math.floor(table.m.length*.37));
    add((table.t[i]+table.t[i+1])*.5,(table.m[j]+table.m[j+1])*.5,'physical-axis-midpoint');
    add(table.t[i],table.m[j],'axis-node');
  }
  let random=0x4d595df4;
  const next=()=>{random=(Math.imul(random,1664525)+1013904223)>>>0;return random/4294967296;};
  for(let i=0;i<64;i++)add(10**(-5+next()*9),next(),'seeded-random');
  return rows;
}
function selectedColumns(nx,nz){
  const points=[[0,0],[nx-1,0],[0,nz-1],[nx-1,nz-1],
    [Math.floor(nx/2),0],[Math.floor(nx/2),nz-1],[0,Math.floor(nz/2)],[nx-1,Math.floor(nz/2)]];
  for(const [x,z] of [[.25,.25],[.75,.25],[.25,.75],[.75,.75],[.5,.5],[.375,.5],[.625,.5],[.5,.625]])points.push([Math.floor(x*nx),Math.floor(z*nz)]);
  return [...new Map(points.map(([x,z])=>[`${x},${z}`,[x,z]])).values()].slice(0,16);
}
function mapAt(values,nx,nz,x,z){return x<0||z<0||x>=nx||z>=nz?0:values[z*nx+x];}
function mapBilinear(values,nx,nz,u,v){
  const x=u-.5,z=v-.5,ix=Math.floor(x),iz=Math.floor(z),fx=x-ix,fz=z-iz;
  return (1-fz)*((1-fx)*mapAt(values,nx,nz,ix,iz)+fx*mapAt(values,nx,nz,ix+1,iz))+
    fz*((1-fx)*mapAt(values,nx,nz,ix,iz+1)+fx*mapAt(values,nx,nz,ix+1,iz+1));
}

export async function checkScatterFit(device,fit,products,sourceObject,state){
  requireValue(products&&!products.destroyed&&!fit.destroyed,'active products required');
  requireValue(sourceObject&&typeof sourceObject.sample==='function','canonical CPU source sampler required');
  requireValue(state&&Number.isFinite(state.source_time_s),'finite committed source time required');
  requireValue(state.light_model===undefined||String(state.light_model)==='2','fitted light model 2 required');
  const dims=[...products.dimensions], [nx,ny,nz]=dims,recipe=sourceObject.recipe,frame=sourceObject.frame;
  requireValue(dims.length===3&&dims.every(x=>Number.isInteger(x)&&x>0),'logical dimensions');
  const time=state.source_time_s,bounds=sourceObject.fieldBounds??recipe.bounds;
  for(const v of [bounds.min,bounds.extent,state.lo_m,state.extent_m,state.sun_direction])requireValue(v?.length===3&&v.every(Number.isFinite),'finite XYZ state/bounds');
  requireValue(bounds.extent.every(x=>x>0)&&state.extent_m.every(x=>x>0),'positive source extents');
  requireValue(Math.abs(Math.hypot(...state.sun_direction)-1)<1e-6,'unit Sun direction');
  requireValue(!frame||frame.sourceParams[0]===Math.fround(time),'matching source generation');
  const identity={...fit.identity},mu=Math.abs(Math.fround(state.sun_direction[1]));
  const actualHashes=await Promise.all([hash(fit.recovery.aBytes),hash(fit.recovery.axesBytes)]);
  requireValue(actualHashes[0]===identity.A&&actualHashes[1]===identity.axes,'CPU recovery/table identity changed');
  const table=tableReader(fit),queries=lutCases(table);
  const code=fit.lookupWGSL+`
@group(0) @binding(12) var<storage,read> scatterProbeInputs:array<vec4f>;
@group(0) @binding(13) var<storage,read_write> scatterProbeOutputs:array<vec4f>;
@compute @workgroup_size(64) fn checkScatterLookup(@builtin(global_invocation_id) id:vec3u){
 if(id.x<arrayLength(&scatterProbeInputs)){
  let q=scatterProbeInputs[id.x];scatterProbeOutputs[id.x]=vec4f(scatterFitA(q.x,q.y),q.x,q.y,1.);
 }
}`;
  let input,output,read,mapped=false;
  try {
    const size=queries.length*16;
    input=device.createBuffer({label:'fit probe inputs',size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    output=device.createBuffer({label:'fit probe outputs',size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
    read=device.createBuffer({label:'fit probe readback',size,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});
    device.queue.writeBuffer(input,0,new Float32Array(queries.flatMap(row=>[row.T,row.mu,0,0])));
    const layout=device.createBindGroupLayout({entries:[...fit.tableLayoutEntries(GPUShaderStage.COMPUTE),
      {binding:12,visibility:GPUShaderStage.COMPUTE,buffer:{type:'read-only-storage',minBindingSize:size}},
      {binding:13,visibility:GPUShaderStage.COMPUTE,buffer:{type:'storage',minBindingSize:size}}]});
    const pipeline=device.createComputePipelineAsync({layout:device.createPipelineLayout({bindGroupLayouts:[layout]}),
      compute:{module:await compileShader(device,code),entryPoint:'checkScatterLookup'}});
    const group=device.createBindGroup({layout,entries:[...fit.tableBindings(),{binding:12,resource:{buffer:input}},{binding:13,resource:{buffer:output}}]});
    const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();pass.setPipeline(await pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(queries.length/64));pass.end();
    encoder.copyBufferToBuffer(output,0,read,0,size);device.queue.submit([encoder.finish()]);
    await read.mapAsync(GPUMapMode.READ);mapped=true;
    const values=new Float32Array(read.getMappedRange()).slice();read.unmap();mapped=false;
    const lutRows=queries.map((row,i)=>{
      const expected=table.lookup(row.T,row.mu),actual=values[i*4];
      const valid=Number.isFinite(actual)&&actual>=0&&actual<3.724&&values[i*4+1]===row.T&&values[i*4+2]===row.mu&&values[i*4+3]===1;
      return {...row,expected_A:expected,actual_A:Number.isFinite(actual)?actual:null,valid,absolute_error:valid?Math.abs(expected-actual):null};
    });
    const [bodyBytes,bankBytes]=await Promise.all([readTexture(device,products.body,[nx,nz,1],4),readTexture(device,products.bank,[nx,nz,1],4)]);
    const body=decodeFloat32(bodyBytes),bank=decodeFloat32(bankBytes);
    requireValue(body.length===nx*nz&&bank.length===nx*nz,'column readback shape');
    const selected=selectedColumns(nx,nz),bodyRows=[];
    const hy=bounds.extent[1]/ny,ylo=bounds.min[1]-hy*.5,span=bounds.extent[1]+hy,step=span/512;
    let maxSampledSigma=0;
    for(const [ix,iz] of selected){
      const qx=bounds.min[0]+(ix+.5)/nx*bounds.extent[0],qz=bounds.min[2]+(iz+.5)/nz*bounds.extent[2];
      let sum=0;
      for(let i=0;i<512;i++){
        const sigma=sourceObject.sample([qx,ylo+(i+.5)*step,qz],time);
        requireValue(Number.isFinite(sigma)&&sigma>=0,'canonical coefficient outside finite nonnegative domain');
        sum+=sigma;maxSampledSigma=Math.max(maxSampledSigma,sigma);
      }
      const canonical=sum*step,gpuBody=mapAt(body,nx,nz,ix,iz),gpuA=mapAt(bank,nx,nz,ix,iz);
      const valid=Number.isFinite(gpuBody)&&gpuBody>=0&&Number.isFinite(gpuA)&&gpuA>=0&&gpuA<3.724;
      const fromGpuBody=valid?table.lookup(gpuBody,mu):null,fromCanonical=table.lookup(canonical,mu);
      bodyRows.push({column_xz:[ix,iz],local_xz_m:[qx,qz],world_xz_m:[state.lo_m[0]+(ix+.5)/nx*state.extent_m[0],state.lo_m[2]+(iz+.5)/nz*state.extent_m[2]],
        canonical_512_T:canonical,gpu_T:Number.isFinite(gpuBody)?gpuBody:null,gpu_A:Number.isFinite(gpuA)?gpuA:null,valid,
        canonical_A:fromCanonical,cpu_A_from_gpu_T:fromGpuBody,
        body_absolute_error:Number.isFinite(gpuBody)?Math.abs(gpuBody-canonical):null,
        A_vs_canonical_body_error:valid?Math.abs(gpuA-fromCanonical):null,
        A_storage_generation_error:valid?Math.abs(gpuA-fromGpuBody):null});
      await new Promise(resolve=>setTimeout(resolve,0));
    }
    const midpointInputs=selected.filter(([x,z])=>x<nx-1&&z<nz-1).map(([x,z])=>[x+1,z+1]);
    midpointInputs.push([0,.5],[nx,nz-.5],[nx*.5,nz*.5]);
    const midpointRows=midpointInputs.slice(0,20).map(([u,v])=>{
      const T=mapBilinear(body,nx,nz,u,v),a=mapBilinear(bank,nx,nz,u,v);
      const valid=Number.isFinite(T)&&T>=0&&Number.isFinite(a)&&a>=0&&a<3.724;
      const expected=valid?table.lookup(T,mu):null;
      return {logical_boundary_xz:[u,v],interpolated_gpu_T:Number.isFinite(T)?T:null,interpolated_gpu_A:Number.isFinite(a)?a:null,
        A_of_interpolated_T:expected,valid,nonlinear_approximation_error:valid?Math.abs(a-expected):null};
    });
    const generationValid=sourceObject.recipe===recipe&&sourceObject.frame===frame&&!products.destroyed&&!fit.destroyed&&Object.keys(identity).every(k=>fit.identity[k]===identity[k]);
    const lut=checkedSummary(lutRows,'absolute_error'),storage=checkedSummary(bodyRows,'A_storage_generation_error');
    return {schema:'thrum.cloud-scatter-probe.v1',scope:'Finite LUT/upload/storage checks and sampled body/cache approximation errors; no appearance, full-SKY-G or performance pass',
      valid_generation:generationValid,quiescent_caller_required:true,profile_identity:identity,probe_shader_sha256:await hash(new TextEncoder().encode(code)),
      source:{kind:sourceObject.kind,recipe_id:recipe.id??recipe.schema??null,recipe_fingerprint:recipe.fingerprint??null,recipe_sha256:await hash(new TextEncoder().encode(JSON.stringify(recipe))),max_level:sourceObject.maxLevel??null,source_time_s:time},
      state:{lo_m:[...state.lo_m],extent_m:[...state.extent_m],sun_direction:[...state.sun_direction],mu0_gpu_f32:mu,dimensions:dims,light_model:state.light_model??'caller-asserted-2'},
      LUT:{...lut,rows:lutRows},centre_bank_storage:{...storage,comparison:'GPU A vs double physical-axis lookup of actual GPU T'},
      body_quality:{status:'UNQUALIFIED: no invented body/A quality threshold',integration:'512 midpoint samples over current field half-voxel Y halo, direct unchanged canonical source',y_range_local_m:[ylo,ylo+span],step_m:step,max_sampled_sigma_per_m:maxSampledSigma,
        T_error:stats(bodyRows,'body_absolute_error'),A_vs_canonical_T_error:stats(bodyRows,'A_vs_canonical_body_error'),rows:bodyRows},
      A_map_nonlinearity:{status:'UNQUALIFIED: interpolation approximation, not LUT arithmetic failure',zero_outside_logical_grid:true,...stats(midpointRows,'nonlinear_approximation_error'),rows:midpointRows},
      implementation_checks_pass:generationValid&&lut.pass&&storage.pass,
      bytes:{shared_fit:fit.bytes,products:products.bytes,LUT_input:size,LUT_output:size,LUT_readback:size,body_readback:bodyBytes.byteLength,bank_readback:bankBytes.byteLength,column_copy_staging_each:Math.ceil(nx*4/256)*256*nz},
      limits:['Canonical integral is within the requested current-field halo, not a convergence certificate for complete source support','GPU body integrates the sampled half field; canonical discrepancy includes field/resolution/half error and CPU quadrature','No source is advanced or changed; table-fit empirical error is distinct from this CPU/GPU interpolation comparison']};
  } finally {
    if(mapped)read?.unmap();input?.destroy();output?.destroy();read?.destroy();
  }
}
