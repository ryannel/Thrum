import {compileShader, hash, readBuffer, readTexture} from './gpu.js';
import {ScalarField, PACKED_LOAD_WGSL} from './scalar-field.js';
import {BEER_WGSL} from './transport.js';

const status = document.querySelector('#status');
const output = document.querySelector('#report');
const runButton = document.querySelector('#run');
const QUERY_SHADER = PACKED_LOAD_WGSL + BEER_WGSL + `
@group(0) @binding(0) var input: texture_3d<f32>;
@group(0) @binding(1) var filtered: sampler;
struct QueryParameters { dimensions:vec4<u32>, lo:vec4<f32>, pitch:vec4<f32> }
@group(0) @binding(2) var<uniform> queryParameters: QueryParameters;
@group(0) @binding(3) var<storage, read> queries: array<vec4<f32>>;
@group(0) @binding(4) var<storage, read_write> results: array<vec4<f32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x>=arrayLength(&queries)) { return; }
  let query=queries[id.x];
  let sigma=samplePackedWorld(input,filtered,query.xyz,queryParameters.lo.xyz,queryParameters.pitch.xyz,queryParameters.dimensions.xyz);
  let segment=homogeneousScalar(vec4<f32>(sigma,query.w,.002,2.));
  results[id.x]=vec4<f32>(sigma,segment.yzw);
}
`;
const ROUND_SHADER = `
@group(0) @binding(0) var<storage,read> inputs:array<f32>;
@group(0) @binding(1) var<storage,read_write> outputs:array<u32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
  if (id.x<arrayLength(&inputs)) { outputs[id.x]=pack2x16float(vec2<f32>(inputs[id.x],0.))&65535u; }
}
`;

const OPERATOR_SHADER = BEER_WGSL + `
@group(0) @binding(0) var<storage,read> inputs:array<vec4<f32>>;
@group(0) @binding(1) var<storage,read_write> outputs:array<vec4<f32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
  if (id.x<arrayLength(&inputs)) { outputs[id.x]=homogeneousScalar(inputs[id.x]); }
}
`;
async function operatorCases(device,cases) {
  const size=cases.length*16;
  const input=device.createBuffer({size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const output=device.createBuffer({size,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(input,0,new Float32Array(cases.flatMap(row=>[row.sigma_per_m,row.length_m,row.source_radiance_per_m,row.incoming_radiance])));
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,OPERATOR_SHADER),entryPoint:'main'}});
  const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:input}},{binding:1,resource:{buffer:output}}]});
  const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();
  pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(cases.length/64));pass.end();device.queue.submit([encoder.finish()]);
  const values=new Float32Array((await readBuffer(device,output,size)).buffer);
  input.destroy();output.destroy();
  return cases.map((row,i)=>{
    const expected=[row.tau,row.T,row.S,row.outgoing_radiance],actual=Array.from(values.slice(i*4,i*4+4));
    const errors=expected.map((value,c)=>Math.abs(value-actual[c]));
    return {input:row,actual,errors,pass:errors.every(error=>error<1e-5)};
  });
}

function fromHex(value) { return Uint8Array.from(value.match(/../g).map(x=>parseInt(x,16))); }
function check(condition, message) { if (!condition) throw Error(message); }

async function queryField(device, pipeline, level, samples, expectedGeometry) {
  const uniform=device.createBuffer({size:48,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
  const input=device.createBuffer({size:samples.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const result=device.createBuffer({size:samples.length*16,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  const parameters=new ArrayBuffer(48),u=new Uint32Array(parameters),f=new Float32Array(parameters);
  u.set([...level.dimensions,0],0);f.set([...level.lo,0],4);f.set([...level.voxelPitch,0],8);
  device.queue.writeBuffer(uniform,0,parameters);
  device.queue.writeBuffer(input,0,new Float32Array(samples.flatMap(sample=>[...sample.boundary_xyz.map((value,k)=>expectedGeometry.lo[k]+value*expectedGeometry.pitch[k]),100])));
  const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[
    {binding:0,resource:level.texture.createView()},
    {binding:1,resource:device.createSampler({minFilter:'linear',magFilter:'linear'})},
    {binding:2,resource:{buffer:uniform}}, {binding:3,resource:{buffer:input}}, {binding:4,resource:{buffer:result}}
  ]});
  const encoder=device.createCommandEncoder(), pass=encoder.beginComputePass();
  pass.setPipeline(pipeline); pass.setBindGroup(0,group); pass.dispatchWorkgroups(Math.ceil(samples.length/64));
  pass.end(); device.queue.submit([encoder.finish()]);
  const values=new Float32Array((await readBuffer(device,result,samples.length*16)).buffer);
  uniform.destroy(); input.destroy(); result.destroy();
  return values;
}

async function roundCases(device, cases) {
  const input=device.createBuffer({size:cases.length*4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
  const result=device.createBuffer({size:cases.length*4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
  device.queue.writeBuffer(input,0,new Float32Array(cases.map(x=>x.input)));
  const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,ROUND_SHADER),entryPoint:'main'}});
  const group=device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[
    {binding:0,resource:{buffer:input}},{binding:1,resource:{buffer:result}}
  ]});
  const encoder=device.createCommandEncoder(),pass=encoder.beginComputePass();
  pass.setPipeline(pipeline);pass.setBindGroup(0,group);pass.dispatchWorkgroups(Math.ceil(cases.length/64));pass.end();
  device.queue.submit([encoder.finish()]);
  const values=new Uint32Array((await readBuffer(device,result,cases.length*4)).buffer);
  input.destroy();result.destroy();
  return cases.map((reference,i)=>({input:reference.input,expected:reference.half_bits,actual:values[i],pass:values[i]===reference.half_bits}));
}

runButton.addEventListener('click', async()=>{
  runButton.disabled=true; status.textContent='Running independent GPU/reference comparisons…';
  let device;
  const report={schema:'thrum.cloud-sky-packing-proof.v1',scope:'R0 scalar storage/sampling/operator only; no cloud-family or timing qualification',failures:[],fixtures:[]};
  try {
    const referenceText=await(await fetch('fixtures/packing.json',{cache:'no-store'})).text();
    const reference=JSON.parse(referenceText);
    report.reference_sha256=await hash(new TextEncoder().encode(referenceText));
    const identity=await(await fetch('/identity',{cache:'no-store'})).json();
    check(identity.build_id===document.querySelector('meta[name="proof-build"]')?.content,'Host build changed: reload the proof before testing');
    report.build_id=identity.build_id;
    report.loaded_code=identity.files;
    check(report.reference_sha256===identity.files['scripts/cloud_sky_proof/fixtures/packing.json'],'Fixture differs from frozen build');
    report.query_shader_sha256=await hash(new TextEncoder().encode(QUERY_SHADER));
    const adapter=await navigator.gpu?.requestAdapter({powerPreference:'high-performance'});
    check(adapter,'WebGPU adapter unavailable');
    device=await adapter.requestDevice();
    device.addEventListener('uncapturederror',event=>report.failures.push('GPU validation: '+event.error.message));
    report.adapter={vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device};
    report.operators=await operatorCases(device,reference.constant_slabs);
    for (const row of report.operators) if (!row.pass) report.failures.push('constant Beer operator differs from independent reference');
    report.rounding=await roundCases(device,reference.rounding);
    for (const row of report.rounding) if (!row.pass) report.failures.push(`half round ${row.input}: ${row.actual} != ${row.expected}`);
    const pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,QUERY_SHADER),entryPoint:'main'}});
    for (const fixture of reference.fixtures) {
      status.textContent='Checking '+fixture.id;
      const field=await ScalarField.create(device,fixture.dimensions_xyz);
      field.setParameters([125.5,-33.25,4],fixture.dimensions_xyz.map((value,k)=>value*[1.25,.75,2.5][k]));
      await field.buildFromHalfBits(fixture.levels[0].dense_half_bits);
      const result={id:fixture.id,physical_guarded_texture_bytes:field.levels.reduce((sum,level)=>sum+level.bytes,0),levels:[]};
      for (let index=0;index<field.levels.length;index++) {
        const level=field.levels[index],expected=fixture.levels[index];
        const actual=await readTexture(device,level.texture,level.physical),bytes=fromHex(expected.rgba_half_le_hex);
        check(actual.length===bytes.length,'Physical byte length mismatch');
        let mismatches=0;
        for (let at=0;at<actual.length;at++) if (actual[at]!==bytes[at]) mismatches++;
        const expectedGeometry={lo:[125.5,-33.25,4],pitch:[1.25,.75,2.5].map(value=>value*2**expected.level)};
        expectedGeometry.extent=expected.logical_dimensions_xyz.map((value,k)=>value*expectedGeometry.pitch[k]);
        check(level.lo.every((value,k)=>value===expectedGeometry.lo[k]),'World origin differs from fixed fixture');
        check(level.voxelPitch.every((value,k)=>value===expectedGeometry.pitch[k]),'Mip pitch must double independently of odd dimensions');
        check(level.extent.every((value,k)=>value===expectedGeometry.extent[k]),'Odd mip extent must retain the zero-padded last parent block');
        if(fixture.id==='ramp-7x5x9'&&index===1) {
          check(level.voxelPitch[2]===5 && level.extent[2]===25 && level.lo[2]+4.5*level.voxelPitch[2]===26.5,'Odd z9 last-child centre moved');
        }
        const values=await queryField(device,pipeline,level,expected.samples,expectedGeometry);
        let maxCoefficientError=0,maxTError=0,maxSError=0,maxLoutError=0;
        const errors=[];
        const peak=Math.max(...expected.dense_half_bits.map(bits=>halfValue(bits)));
        // Eight-bit subtexel weight error per axis is an explicit portable filtering envelope.
        const coefficientBound=3*peak/256+2e-7;
        for (let at=0;at<expected.samples.length;at++) {
          const sigma=expected.samples[at].coefficient,tau=sigma*100,T=Math.exp(-tau);
          const S=sigma===0?.2:.002*(-Math.expm1(-tau))/sigma;
          const error=Math.abs(values[at*4]-sigma),terror=Math.abs(values[at*4+1]-T);
          maxCoefficientError=Math.max(maxCoefficientError,error);maxTError=Math.max(maxTError,terror);
          maxSError=Math.max(maxSError,Math.abs(values[at*4+2]-S));
          maxLoutError=Math.max(maxLoutError,Math.abs(values[at*4+3]-(S+2*T)));
          if (error>coefficientBound || terror>.002 || Math.abs(values[at*4+2]-S)>.002 || Math.abs(values[at*4+3]-(S+2*T))>.006 || !Number.isFinite(values[at*4+3])) errors.push({sample:expected.samples[at].boundary_xyz,actual:Array.from(values.slice(at*4,at*4+4)),sigma,T,error,terror});
        }
        const row={level:index,dimensions:level.dimensions,physical:level.physical,world_lo:level.lo,world_pitch:level.voxelPitch,world_extent:level.extent,byte_mismatches:mismatches,samples:expected.samples.length,max_coefficient_error:maxCoefficientError,coefficient_bound:coefficientBound,max_T_error:maxTError,max_S_error:maxSError,max_Lout_error:maxLoutError,errors};
        result.levels.push(row);
        if (mismatches || errors.length) report.failures.push(`${fixture.id}/level${index}: ${mismatches} byte mismatches; ${errors.length} sampler/operator failures`);
      }
      report.fixtures.push(result);field.destroy();
    }
    report.pass=report.failures.length===0;
    report.sample_count=report.fixtures.reduce((sum,row)=>sum+row.levels.reduce((n,l)=>n+l.samples,0),0);
    status.textContent=report.pass?'PASS · exact packed bytes/mips; sampled operators within the stated filter envelope':'FAIL · '+report.failures.join('; ');
  } catch (error) { report.pass=false;report.failures.push(error.stack);status.textContent='FAIL · '+error.message; }
  finally {
    device?.destroy();
    output.textContent=JSON.stringify(report,null,2);
    try {
      const response=await fetch('/capture',{method:'POST',headers:{'X-Capture-Name':`packing-${Date.now()}.json`},body:JSON.stringify(report,null,2)});
      if (!response.ok) throw Error(String(response.status));
    } catch (error) { status.textContent+=' · receipt save failed '+error.message; }
    finally { runButton.disabled=false; }
  }

});

function halfValue(bits) {
  const exponent=(bits>>10)&31,fraction=bits&1023;
  return exponent===0?fraction*2**-24:(1+fraction/1024)*2**(exponent-15);
}
