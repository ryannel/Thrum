// Standalone proof adapters; this is not a new engine provider framework.
import * as coarse from './convective.js';
import {ScalarField,FIELD_PARAMETERS_WGSL,FIELD_DISPATCH_WGSL} from './scalar-field.js';
import {compileShader} from './gpu.js';

export class CuSourceField {
  static async create(device,kind,variant,side=256,maxLevel=2) {
    const source=new CuSourceField();
    source.device=device;source.kind=kind;source.maxLevel=maxLevel;
    try {
    source.module=kind==='hierarchy'?await import('./convective-hierarchy.js'):coarse;
    source.recipe=kind==='hierarchy'?source.module.createHierarchyRecipe(variant,1729):coarse.createCuRecipe(variant,1729);
    source.shader=kind==='hierarchy'?source.module.HIERARCHY_WGSL:coarse.CONVECTIVE_WGSL;
    source.field=await ScalarField.create(device,[side,side,side]);
    const initial=source.makeFrame(650);
    source.nodes=device.createBuffer({size:kind==='hierarchy'?256*64:initial.nodes.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    if(kind==='hierarchy') {
      source.metadata=device.createBuffer({size:initial.metadata.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
      source.ids=device.createBuffer({size:262144*4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    }
    source.pipeline=await device.createComputePipelineAsync({layout:'auto',compute:{module:await compileShader(device,FIELD_PARAMETERS_WGSL+source.shader+FIELD_DISPATCH_WGSL),entryPoint:'fillField'}});
    source.group=device.createBindGroup({layout:source.pipeline.getBindGroupLayout(0),entries:[
      {binding:0,resource:{buffer:source.field.levels[0].uniform}},{binding:1,resource:source.field.levels[0].texture.createView()},...source.sourceEntries()
    ]});
    source.sigmaMax=kind==='hierarchy'?source.recipe.sigma_m_inv:Math.max(...source.recipe.cohorts.map(node=>node.sigma_m_inv));
    return source;
    } catch(error) {source.destroy();throw error;}
  }

  makeFrame(time) {
    if(this.kind==='hierarchy')return this.module.hierarchyFrame(this.recipe,time,this.field.levels[0].dimensions,{maxLevel:this.maxLevel});
    return {nodes:coarse.cuFrame(this.recipe,time),sourceParams:new Float32Array([time,0,0,0]),stats:{profile:'rejected coarse graph',live_node_capacity:this.recipe.cohorts.length}};
  }

  prepare(time) {
    if(this.frame&&this.frame.sourceParams[0]===Math.fround(time))return this.frame;
    const frame=this.makeFrame(time);
    if(this.kind==='hierarchy') {
      this.module.validateHierarchyFrame(frame,{recipeFingerprint:this.recipe.fingerprint,sourceTime:time,dimensions:this.field.levels[0].dimensions,bounds:this.recipe.bounds,maxLevel:this.maxLevel,sourceParams:[Math.fround(time),this.recipe.sigma_m_inv,this.recipe.w_edge_m,this.recipe.base_feather_m]});
      const u=new Uint32Array(new Float32Array([time]).buffer);
      if(frame.metadata[7]!==u[0] || frame.metadata[8]!==this.recipe.fingerprint || frame.metadata[14]!==1 || frame.metadata[15]!==this.maxLevel || this.field.levels[0].dimensions.some((n,k)=>frame.metadata[9+k]!==n))throw Error('Incompatible source/index generation');
      if(frame.nodes.byteLength>256*64 || frame.ids.byteLength>262144*4)throw Error('Source input pool capacity exceeded');
      this.device.queue.writeBuffer(this.metadata,0,frame.metadata);
      if(frame.ids.byteLength)this.device.queue.writeBuffer(this.ids,0,frame.ids);
    }
    if(frame.nodes.byteLength)this.device.queue.writeBuffer(this.nodes,0,frame.nodes);
    this.field.setParameters(this.recipe.bounds.min,this.recipe.bounds.extent,frame.sourceParams[0],frame.sourceParams,this.kind==='hierarchy'?this.recipe.fingerprint:0);
    this.frame=frame;this.needsBuild=true;
    return frame;
  }

  sourceEntries() {
    const entries=[{binding:2,resource:{buffer:this.nodes}}];
    if(this.kind==='hierarchy')entries.push({binding:3,resource:{buffer:this.metadata}},{binding:4,resource:{buffer:this.ids}});
    return entries;
  }
  probeEntries() {return [{binding:0,resource:{buffer:this.field.levels[0].uniform}},...this.sourceEntries()];}
  sample(point,time) {return this.kind==='hierarchy'?this.module.sampleHierarchy(this.recipe,point,time,{maxLevel:this.maxLevel}):coarse.sampleCu(this.recipe,point,time);}
  encode(pass) {if(this.needsBuild)this.field.encode(pass,this.pipeline,this.group);}
  markSubmitted() {this.needsBuild=false;}
  destroy() {
    this.field?.destroy();this.nodes?.destroy();this.metadata?.destroy();this.ids?.destroy();
  }
}
