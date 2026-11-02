/** Bounded liquid fit adapter. No self-submission, source clock or private view state.
 * Create/replace/destroy only at the caller's quiescent boundary. Root owns the
 * source/grid/Sun/table cache key and submits this after the matching packed field.
 */
export const SCATTER_FIT_PINS = Object.freeze({
  profile: '8de788d28c53935f803f3e78085aaf46eec4f8fcb346c857da64522693ea8604',
  A: '35aab6271d4119719c7ca536f1c22fc2cce492e7e7c8abab88f9fe71909a4ed4',
  axes: 'efb148cbf3b059d8526aee2fe47eab3dc76d0bc0c82956f89fd33f33b16a21e8',
});
const MARKER = '// END_SCATTER_TABLE_LOOKUP';
const A_BOUND = 3.724;

function bytes(value) {
  return value instanceof Uint8Array ? value : new Uint8Array(value);
}
async function sha256(value) {
  const hash = await crypto.subtle.digest('SHA-256', bytes(value));
  return [...new Uint8Array(hash)].map(x => x.toString(16).padStart(2, '0')).join('');
}
async function fetchBytes(url) {
  const response = await fetch(url, {cache:'no-store'});
  if (!response.ok) throw Error(`Scatter fit fetch failed: ${response.status} ${url}`);
  return new Uint8Array(await response.arrayBuffer());
}
function requireValue(condition, message) {
  if (!condition) throw Error('Scatter fit admission: ' + message);
}

/** CPU admission helper for independent tests; validates before any GPU allocation. */
export async function verifyScatterPackage(profileInput, aInput, axesInput) {
  const profileBytes=bytes(profileInput),aBytes=bytes(aInput),axesBytes=bytes(axesInput);
  const hashes=await Promise.all([sha256(profileBytes),sha256(aBytes),sha256(axesBytes)]);
  requireValue(hashes[0]===SCATTER_FIT_PINS.profile&&hashes[1]===SCATTER_FIT_PINS.A&&hashes[2]===SCATTER_FIT_PINS.axes,'pinned package SHA mismatch');
  const profile=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(profileBytes));
  requireValue(profile.schema==='thrum.cloud-scatter-fit.v1'&&profile.empirical_certificate===false,'schema/evidence level');
  const p=profile.parameters;
  requireValue(p.phase==='normalized-HG'&&p.g===.86&&p.omega===1&&p.kappa===.25&&p.rho_opaque_art_target===.8,'liquid phase/profile key');
  const nt=profile.counts.T,nm=profile.counts.mu;
  requireValue(Number.isInteger(nt)&&nt>=3&&nt<=256&&Number.isInteger(nm)&&nm>=2&&nm<=256,'axis counts');
  requireValue(profile.files.A.filename==='A.f32'&&profile.files.axes.filename==='axes.f32','file names');
  requireValue(profile.files.A.sha256===hashes[1]&&profile.files.axes.sha256===hashes[2],'manifest binary SHA');
  requireValue(aBytes.byteLength===4*nt*nm&&axesBytes.byteLength===4*(nt+nm),'binary shape');
  requireValue(profile.files.A.bytes===aBytes.byteLength&&profile.files.axes.bytes===axesBytes.byteLength,'manifest byte counts');
  requireValue(profile.evaluation.T_below===1e-4&&profile.evaluation.T_above===1000,'boundary policy');
  requireValue(profile.hard_bounds.A_exclusive_max===A_BOUND&&profile.hard_bounds.source_ratio_max_cd_m2===2000000,'hard source bounds');
  const axisView=new DataView(axesBytes.buffer,axesBytes.byteOffset,axesBytes.byteLength);
  for(const [name,offset,count,last] of [['T',0,nt,1000],['mu',nt,nm,1]]) {
    const declared=profile.axes[name];requireValue(Array.isArray(declared)&&declared.length===count,'declared axis length');
    let previous=-Infinity;
    for(let i=0;i<count;i++) {
      const value=axisView.getFloat32((offset+i)*4,true);
      requireValue(Number.isFinite(value)&&value>previous&&value===declared[i],'exact finite ordered float32 axis');
      if(i===0)requireValue(value===0,'axis origin');
      if(i===count-1)requireValue(value===last,'axis end');
      previous=value;
    }
  }
  const values=new DataView(aBytes.buffer,aBytes.byteOffset,aBytes.byteLength);
  let minimum=Infinity,maximum=-Infinity;
  for(let i=0;i<nt*nm;i++) {
    const value=values.getFloat32(i*4,true);
    requireValue(Number.isFinite(value)&&value>=0&&value<A_BOUND,'finite coefficient range');
    minimum=Math.min(minimum,value);maximum=Math.max(maximum,value);
  }
  requireValue(minimum===profile.hard_bounds.actual_A_min&&maximum===profile.hard_bounds.actual_A_max,'declared coefficient extrema');
  return {profile,profileBytes,aBytes,axesBytes,identity:{profile:hashes[0],A:hashes[1],axes:hashes[2]}};
}

/** Extract declarations + manual lookup, without Scene/output bindings or an entry point. */
export function scatterLookupWGSL(source) {
  requireValue(source.split(MARKER).length===2,'shader lookup marker');
  return source.split(MARKER)[0];
}

export class ScatterFit {
  static async create(device, compileShader) {
    if(typeof compileShader!=='function')throw TypeError('ScatterFit.create requires compileShader(device,code)');
    const base=new URL('./fixtures/scatter-liquid/',import.meta.url);
    const [manifest,a,axes,shaderBytes]=await Promise.all([
      fetchBytes(new URL('profile.json',base)),fetchBytes(new URL('A.f32',base)),
      fetchBytes(new URL('axes.f32',base)),fetchBytes(new URL('./scatter-fit.wgsl',import.meta.url)),
    ]);
    const admitted=await verifyScatterPackage(manifest,a,axes);
    const source=new TextDecoder('utf-8',{fatal:true}).decode(shaderBytes);
    const result=new ScatterFit();
    result.device=device;result.profile=admitted.profile;result.recovery=admitted;
    result.identity={...admitted.identity,shader:await sha256(shaderBytes)};
    result.lookupWGSL=scatterLookupWGSL(source);result.products=new Set();result.destroyed=false;
    const nt=admitted.profile.counts.T,nm=admitted.profile.counts.mu;
    result.bytes={table:nt*nm*4,axes:axes.byteLength,profile:32,
      sharedGPU:nt*nm*4+axes.byteLength+32,cpuRecoveryPayload:manifest.byteLength+a.byteLength+axes.byteLength};
    try {
      result.table=device.createTexture({label:'liquid A [mu,T]',size:[nt,nm],format:'r32float',
        usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST});
      result.tableView=result.table.createView();
      // Pad upload rows explicitly; do not rely on buffer-copy alignment exemptions.
      const stride=Math.ceil(nt*4/256)*256,upload=new Uint8Array(stride*nm);
      for(let row=0;row<nm;row++)upload.set(a.subarray(row*nt*4,(row+1)*nt*4),row*stride);
      device.queue.writeTexture({texture:result.table},upload,{bytesPerRow:stride,rowsPerImage:nm},[nt,nm]);
      result.axes=device.createBuffer({label:'scatter T then mu axes',size:axes.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
      device.queue.writeBuffer(result.axes,0,axes);
      result.uniform=device.createBuffer({label:'scatter profile counts/limits',size:32,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
      const record=new ArrayBuffer(32);new Uint32Array(record).set([nt,nm,0,0]);
      new Float32Array(record).set([1e-4,1000,A_BOUND,.25],4);device.queue.writeBuffer(result.uniform,0,record);
      result.inputLayout=device.createBindGroupLayout({entries:[
        {binding:0,visibility:GPUShaderStage.COMPUTE,texture:{sampleType:'unfilterable-float',viewDimension:'3d'}},
        {binding:4,visibility:GPUShaderStage.COMPUTE,buffer:{type:'uniform',minBindingSize:192}},
        ...result.tableLayoutEntries(GPUShaderStage.COMPUTE),
      ]});
      result.outputLayout=device.createBindGroupLayout({entries:[2,3].map(binding=>({binding,visibility:GPUShaderStage.COMPUTE,
        storageTexture:{access:'write-only',format:'r32float',viewDimension:'2d'}}))});
      const layout=device.createPipelineLayout({bindGroupLayouts:[result.inputLayout,result.outputLayout]});
      result.pipeline=await device.createComputePipelineAsync({layout,compute:{module:await compileShader(device,source),entryPoint:'buildScatterColumns'}});
      return result;
    } catch(error) {result.destroy();throw error;}
  }

  tableLayoutEntries(visibility=GPUShaderStage.COMPUTE) {
    return [
      {binding:8,visibility,texture:{sampleType:'unfilterable-float',viewDimension:'2d'}},
      {binding:9,visibility,buffer:{type:'read-only-storage',minBindingSize:this.bytes.axes}},
      {binding:10,visibility,buffer:{type:'uniform',minBindingSize:32}},
    ];
  }
  tableBindings() {
    if(this.destroyed)throw Error('ScatterFit retired');
    return [{binding:8,resource:this.tableView},{binding:9,resource:{buffer:this.axes}},{binding:10,resource:{buffer:this.uniform}}];
  }
  createProducts(dimensions) {
    if(this.destroyed)throw Error('ScatterFit retired');
    if(!Array.isArray(dimensions)||dimensions.length!==3||!dimensions.every(x=>Number.isInteger(x)&&x>0))throw RangeError('positive integer [nx,ny,nz] required');
    const [nx,ny,nz]=dimensions,limit=this.device.limits.maxTextureDimension2D;
    if(nx>limit||nz>limit)throw RangeError('scatter column texture exceeds adapter limits');
    const usage=GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC;
    const body=this.device.createTexture({label:'owner world-Y body column',size:[nx,nz],format:'r32float',usage});
    let bank;
    try {bank=this.device.createTexture({label:'owner fitted A column',size:[nx,nz],format:'r32float',usage});}
    catch(error){body.destroy();throw error;}
    const fit=this,product={dimensions:[nx,ny,nz],body,bank,bodyView:body.createView(),bankView:bank.createView(),
      bytes:{body:nx*nz*4,bank:nx*nz*4,total:nx*nz*8},destroyed:false};
    try {
      product.outputGroup=this.device.createBindGroup({layout:this.outputLayout,entries:[
        {binding:2,resource:product.bodyView},{binding:3,resource:product.bankView},
      ]});
    } catch(error){body.destroy();bank.destroy();throw error;}
    product.configure=(fieldTexture,sceneBuffer)=>{
      if(product.destroyed||fit.destroyed)throw Error('scatter products retired');
      if(fieldTexture.width!==nx+2||fieldTexture.height!==ny+2||fieldTexture.depthOrArrayLayers!==Math.ceil(nz/4)+2||fieldTexture.format!=='rgba16float'||fieldTexture.dimension!=='3d')throw RangeError('packed field shape/format mismatch');
      if(sceneBuffer.size<192)throw RangeError('scatter Scene must provide the 192-byte ABI');
      if(product.fieldTexture===fieldTexture&&product.sceneBuffer===sceneBuffer)return product;
      product.inputGroup=fit.device.createBindGroup({layout:fit.inputLayout,entries:[
        {binding:0,resource:fieldTexture.createView()},{binding:4,resource:{buffer:sceneBuffer}},...fit.tableBindings(),
      ]});
      product.fieldTexture=fieldTexture;product.sceneBuffer=sceneBuffer;return product;
    };
    product.encode=pass=>{
      if(product.destroyed||fit.destroyed||!product.inputGroup)throw Error('configure active scatter products before encode');
      pass.setPipeline(fit.pipeline);pass.setBindGroup(0,product.inputGroup);pass.setBindGroup(1,product.outputGroup);
      pass.dispatchWorkgroups(Math.ceil(nx/8),Math.ceil(Math.ceil(nz/4)/8));
    };
    product.destroy=()=>{if(product.destroyed)return;product.destroyed=true;body.destroy();bank.destroy();fit.products.delete(product);};
    this.products.add(product);return product;
  }
  destroy() {
    if(this.destroyed)return;this.destroyed=true;
    for(const product of this.products??[])product.destroy();
    this.table?.destroy();this.axes?.destroy();this.uniform?.destroy();
  }
}
