/** Bounded 64-cell/66-column K48 Sun-depth candidate. No queue submission/clock.
 * Caller owns a quiescent, common source/Scene/light generation and invalidation.
 * Read metadata admission as compression-only, NOT a total .02 query certificate.
 */
export const DEEP_SUN_PROFILE=Object.freeze({nominalColumns:64,physicalColumns:66,
  knots:48,intervals:512,endpoints:513,exponents:Object.freeze([1,.25]),
  compressionAllocation:.005,tubeEpsilon:.004998,numericalReserve:.000002,fallbackCap:4096,
  backendUniformCertificate:false});
const COLUMNS=66*66,NODE_BYTES=COLUMNS*48*8,META_BYTES=COLUMNS*32;

function section(source,name) {
  const begin=`// BEGIN_DEEP_${name}`,end=`// END_DEEP_${name}`;
  if(source.split(begin).length!==2||source.split(end).length!==2)throw Error(`Deep Sun missing/duplicate ${name} section`);
  return source.split(begin)[1].split(end)[0];
}
/** Query expects Scene `scene`, tauDirect(p,Sun,count), errors atomic[4]. */
export function deepSunShaderSections(source) {
  const common=section(source,'COMMON');
  return {queryWGSL:common+section(source,'QUERY'),buildWGSL:common+section(source,'BUILD')};
}
async function shaderText() {
  const response=await fetch(new URL('./deep-sun.wgsl',import.meta.url),{cache:'no-store'});
  if(!response.ok)throw Error(`Deep Sun shader fetch ${response.status}`);
  return response.text();
}
async function sha256(text) {
  const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text));
  return [...new Uint8Array(hash)].map(x=>x.toString(16).padStart(2,'0')).join('');
}

export class DeepSun {
  static async create(device,compileShader) {
    if(typeof compileShader!=='function')throw TypeError('DeepSun.create requires compileShader(device, code)');
    const source=await shaderText(),sections=deepSunShaderSections(source),result=new DeepSun();
    result.device=device;result.profile=DEEP_SUN_PROFILE;result.queryWGSL=sections.queryWGSL;
    result.identity={shader:await sha256(source),profile:'thrum.deep-sun.tau-tube.k48.64x64.512.v1'};
    result.bytes={nodes:NODE_BYTES,metadata:META_BYTES,metadataStride:32,totalGPU:NODE_BYTES+META_BYTES,
      privateState:'O(1) scalar/vector state; compiler allocation/spills unmeasured'};
    result.metadataSchema={admission:['count','compressionValid','nominalPrimaryBound','nominalKappaBound'],
      work:['coefficientRequests','endpointVisits','anchorBuilds','failureBits'],
      failureBits:{frame:1,coefficient:2,depthTau:4,capacity:8,tubeArithmetic:16},
      note:'On valid columns emittedPieces=count-1, reprocess=endpointVisits-coefficientRequests. Bounds include once-only 2e-6 proposed reserve; no uniform backend certificate.'};
    result.workCeilings={columns:COLUMNS,densitySamples:COLUMNS*512,inputEndpoints:COLUMNS*513,
      endpointVisits:COLUMNS*1024,anchorBuilds:COLUMNS*48,
      anchorExponentials:COLUMNS*48*2,anchorLogEvaluations:COLUMNS*48*2,
      directFallbackRequests:4096,directFallbackDensitySamples:4096*512,
      note:'Logical ceilings, not measured GPU operations. Arithmetic/capacity failure may stop a column early. Each coefficient sample uses two packed filtered texture reads; small-r log uses an eight-term series.'};
    result.destroyed=false;
    try {
      for(const size of [NODE_BYTES,META_BYTES]) {
        if(size>device.limits.maxStorageBufferBindingSize||size>device.limits.maxBufferSize)throw RangeError('Deep Sun buffers exceed adapter limits');
      }
      const usage=GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC;
      result.nodes=device.createBuffer({label:'deep Sun ordinary depth/tau K48',size:NODE_BYTES,usage});
      result.metadata=device.createBuffer({label:'deep Sun count/admission/two errors',size:META_BYTES,usage});
      result.sampler=device.createSampler({minFilter:'linear',magFilter:'linear',mipmapFilter:'nearest',addressModeU:'clamp-to-edge',addressModeV:'clamp-to-edge',addressModeW:'clamp-to-edge'});
      result.inputLayout=device.createBindGroupLayout({entries:[
        {binding:0,visibility:GPUShaderStage.COMPUTE,texture:{sampleType:'float',viewDimension:'3d'}},
        {binding:1,visibility:GPUShaderStage.COMPUTE,sampler:{type:'filtering'}},
        {binding:4,visibility:GPUShaderStage.COMPUTE,buffer:{type:'uniform',minBindingSize:192}},
      ]});
      result.outputLayout=device.createBindGroupLayout({entries:[
        {binding:4,visibility:GPUShaderStage.COMPUTE,buffer:{type:'storage',minBindingSize:NODE_BYTES}},
        {binding:5,visibility:GPUShaderStage.COMPUTE,buffer:{type:'storage',minBindingSize:META_BYTES}},
      ]});
      result.outputGroup=device.createBindGroup({layout:result.outputLayout,entries:[
        {binding:4,resource:{buffer:result.nodes}},{binding:5,resource:{buffer:result.metadata}},
      ]});
      const layout=device.createPipelineLayout({bindGroupLayouts:[result.inputLayout,result.outputLayout]});
      result.pipeline=await device.createComputePipelineAsync({layout,compute:{module:await compileShader(device,sections.buildWGSL),entryPoint:'buildDeepSun'}});
      return result;
    }catch(error){result.destroy();throw error;}
  }
  layoutEntries(visibility=GPUShaderStage.FRAGMENT) {
    return [{binding:11,visibility,buffer:{type:'read-only-storage',minBindingSize:NODE_BYTES}},
      {binding:12,visibility,buffer:{type:'read-only-storage',minBindingSize:META_BYTES}}];
  }
  bindings() {
    if(this.destroyed)throw Error('Deep Sun retired');
    return [{binding:11,resource:{buffer:this.nodes}},{binding:12,resource:{buffer:this.metadata}}];
  }
  createInputGroup(fieldTexture,sceneBuffer) {
    if(this.destroyed)throw Error('Deep Sun retired');
    if(fieldTexture.format!=='rgba16float'||fieldTexture.dimension!=='3d'||fieldTexture.width<3||fieldTexture.height<3||fieldTexture.depthOrArrayLayers<3)throw RangeError('Deep Sun requires canonical packed RGBA16F field');
    if(sceneBuffer.size<192)throw RangeError('Deep Sun requires Scene192 ABI');
    return this.device.createBindGroup({layout:this.inputLayout,entries:[
      {binding:0,resource:fieldTexture.createView()},{binding:1,resource:this.sampler},{binding:4,resource:{buffer:sceneBuffer}},
    ]});
  }
  configure(fieldTexture,sceneBuffer) {
    this.inputGroup=this.createInputGroup(fieldTexture,sceneBuffer);
    this.fieldTexture=fieldTexture;this.sceneBuffer=sceneBuffer;return this;
  }
  encode(pass,inputGroup=this.inputGroup) {
    if(this.destroyed||!inputGroup)throw Error('Configure active Deep Sun before encode');
    pass.setPipeline(this.pipeline);pass.setBindGroup(0,inputGroup);pass.setBindGroup(1,this.outputGroup);
    pass.dispatchWorkgroups(Math.ceil(66/4),Math.ceil(66/4));
  }
  destroy() {
    if(this.destroyed)return;this.destroyed=true;this.nodes?.destroy();this.metadata?.destroy();
    this.inputGroup=null;this.fieldTexture=null;this.sceneBuffer=null;
  }
}
