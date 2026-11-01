import {compileShader} from './gpu.js';

export const FIELD_PARAMETERS_WGSL = `
struct FieldParameters {
  lo: vec4<f32>, extent: vec4<f32>, grid: vec4<u32>, source: vec4<f32>
}
@group(0) @binding(0) var<uniform> field: FieldParameters;
@group(0) @binding(1) var outputField: texture_storage_3d<rgba16float, write>;
fn roundHalf(value: f32) -> f32 {
  return unpack2x16float(pack2x16float(vec2<f32>(value, 0.))).x;
}
`;

// A complete physical texel, including guards/padding, has exactly one writer.
export const FIELD_DISPATCH_WGSL = `
@compute @workgroup_size(4, 4, 4)
fn fillField(@builtin(global_invocation_id) id: vec3<u32>) {
  let physical = textureDimensions(outputField);
  if (any(id >= physical)) { return; }
  let xy = vec2<i32>(id.xy) - vec2<i32>(1);
  let zBase = (i32(id.z) - 1) * 4;
  var value = vec4<f32>(0.);
  for (var channel = 0; channel < 4; channel++) {
    let logical = vec3<i32>(xy, zBase + channel);
    if (all(logical >= vec3<i32>(0)) && all(logical < vec3<i32>(field.grid.xyz))) {
      let q = field.lo.xyz + (vec3<f32>(logical) + .5) / vec3<f32>(field.grid.xyz) * field.extent.xyz;
      value[channel] = roundHalf(canonicalSigma(q, field.source.x));
    }
  }
  textureStore(outputField, vec3<i32>(id), value);
}
`;

export const PACKED_LOAD_WGSL = `
fn loadPackedCell(input: texture_3d<f32>, cell: vec3<i32>, dimensions: vec3<u32>) -> f32 {
  if (any(cell < vec3<i32>(0)) || any(cell >= vec3<i32>(dimensions))) { return 0.; }
  let group = cell.z / 4;
  let channel = cell.z % 4;
  return textureLoad(input, vec3<i32>(cell.xy + vec2<i32>(1), group + 1), 0)[channel];
}
// u is boundary-space: logical cell i has its centre at i+.5.
fn samplePacked(input: texture_3d<f32>, filtered: sampler, u: vec3<f32>, dimensions: vec3<u32>) -> f32 {
  let q = u.z - .5;
  let z0 = i32(floor(q));
  let z1 = z0 + 1;
  let physical = vec3<f32>(textureDimensions(input));
  let uvXY = (u.xy + vec2<f32>(1.)) / physical.xy;
  var values = vec2<f32>(0.);
  for (var index = 0; index < 2; index++) {
    let z = select(z0, z1, index == 1);
    if (z >= 0 && z < i32(dimensions.z)) {
      let group = z / 4;
      let channel = z % 4;
      let uv = vec3<f32>(uvXY, (f32(group) + 1.5) / physical.z);
      values[index] = textureSampleLevel(input, filtered, uv, 0.)[channel];
    }
  }
  return mix(values.x, values.y, fract(q));
}
fn samplePackedWorld(input: texture_3d<f32>, filtered: sampler, world: vec3<f32>, lo: vec3<f32>, pitch: vec3<f32>, dimensions: vec3<u32>) -> f32 {
  return samplePacked(input,filtered,(world-lo)/pitch,dimensions);
}
`;

const DENSE_WGSL = `
@group(0) @binding(2) var<storage, read> denseBits: array<u32>;
fn canonicalSigma(q: vec3<f32>, age: f32) -> f32 {
  let c = vec3<u32>(floor((q-field.lo.xyz)/field.extent.xyz*vec3<f32>(field.grid.xyz)));
  let i = (c.z*field.grid.y+c.y)*field.grid.x+c.x;
  return unpack2x16float(denseBits[i]).x;
}
`;

const MIP_WGSL = FIELD_PARAMETERS_WGSL + PACKED_LOAD_WGSL + `
@group(0) @binding(2) var parent: texture_3d<f32>;
@group(0) @binding(3) var<uniform> parentDimensions: vec4<u32>;
fn canonicalSigma(q: vec3<f32>, age: f32) -> f32 {
  let c = vec3<i32>(floor((q-field.lo.xyz)/field.extent.xyz*vec3<f32>(field.grid.xyz)));
  var sum = 0.;
  for (var z=0; z<2; z++) { for (var y=0; y<2; y++) { for (var x=0; x<2; x++) {
    sum += loadPackedCell(parent, c*2+vec3<i32>(x,y,z), parentDimensions.xyz);
  } } }
  return sum*.125;
}
` + FIELD_DISPATCH_WGSL;

export function physicalDimensions(dimensions) {
  return [dimensions[0]+2, dimensions[1]+2, Math.ceil(dimensions[2]/4)+2];
}

export class ScalarField {
  static async create(device, dimensions, levelCount = 3) {
    const result = new ScalarField();
    result.device = device;
    result.levels = [];
    let current = [...dimensions];
    for (let level=0; level<levelCount; level++) {
      const physical = physicalDimensions(current);
      const texture = device.createTexture({label: `scalar level ${level}`, dimension:'3d', size:physical,
        format:'rgba16float', usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.STORAGE_BINDING|GPUTextureUsage.COPY_SRC});
      const uniform = device.createBuffer({size:64, usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
      result.levels.push({dimensions:[...current], physical, texture, uniform,
        bytes:physical.reduce((a,b)=>a*b,1)*8});
      current = current.map(x=>Math.ceil(x/2));
    }
    result.mipPipeline = await device.createComputePipelineAsync({layout:'auto',
      compute:{module:await compileShader(device,MIP_WGSL),entryPoint:'fillField'}});
    result.mipGroups = [];
    for (let level=1; level<levelCount; level++) {
      const parent=result.levels[level-1], child=result.levels[level];
      const dimensions=device.createBuffer({size:16,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});
      device.queue.writeBuffer(dimensions,0,new Uint32Array([...parent.dimensions,0]));
      const group=device.createBindGroup({layout:result.mipPipeline.getBindGroupLayout(0),entries:[
        {binding:0,resource:{buffer:child.uniform}}, {binding:1,resource:child.texture.createView()},
        {binding:2,resource:parent.texture.createView()}, {binding:3,resource:{buffer:dimensions}}
      ]});
      result.mipGroups.push({group,dimensions});
    }
    return result;
  }

  setParameters(lo, extent, sourceTime = 0) {
    const base=this.levels[0].dimensions;
    for (let index=0; index<this.levels.length; index++) {
      const level=this.levels[index];
      // Ceil odd mips retain a partial last parent block: pitch doubles, the padded extent grows.
      level.lo=[...lo];
      level.voxelPitch=extent.map((value,k)=>value/base[k]*2**index);
      level.extent=level.dimensions.map((value,k)=>value*level.voxelPitch[k]);
      const data=new ArrayBuffer(64),f=new Float32Array(data),u=new Uint32Array(data);
      f.set([...lo,0],0); f.set([...level.extent,0],4); u.set([...level.dimensions,0],8); f[12]=sourceTime;
      this.device.queue.writeBuffer(level.uniform,0,data);
    }
  }

  async buildFromHalfBits(bits) {
    const input=this.device.createBuffer({size:Math.max(4,bits.length*4),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
    this.device.queue.writeBuffer(input,0,new Uint32Array(bits));
    const pipeline=await this.device.createComputePipelineAsync({layout:'auto',
      compute:{module:await compileShader(this.device,FIELD_PARAMETERS_WGSL+DENSE_WGSL+FIELD_DISPATCH_WGSL),entryPoint:'fillField'}});
    const group=this.device.createBindGroup({layout:pipeline.getBindGroupLayout(0),entries:[
      {binding:0,resource:{buffer:this.levels[0].uniform}}, {binding:1,resource:this.levels[0].texture.createView()},
      {binding:2,resource:{buffer:input}}
    ]});
    const encoder=this.device.createCommandEncoder(),pass=encoder.beginComputePass();
    this.encode(pass,pipeline,group);
    pass.end(); this.device.queue.submit([encoder.finish()]); await this.device.queue.onSubmittedWorkDone(); input.destroy();
  }

  encode(pass, pipeline, group) {
    pass.setPipeline(pipeline); pass.setBindGroup(0,group);
    const dispatch=level=>pass.dispatchWorkgroups(...level.physical.map(x=>Math.ceil(x/4)));
    dispatch(this.levels[0]);
    for (let index=0; index<this.mipGroups.length; index++) {
      pass.setPipeline(this.mipPipeline); pass.setBindGroup(0,this.mipGroups[index].group); dispatch(this.levels[index+1]);
    }
  }

  destroy() {
    for (const level of this.levels) { level.texture.destroy(); level.uniform.destroy(); }
    for (const mip of this.mipGroups) mip.dimensions.destroy();
  }
}
