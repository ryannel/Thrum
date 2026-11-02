import assert from 'node:assert/strict';
import {ScalarField} from './scalar-field.js';
import {CuSourceField} from './cu-source.js';
globalThis.GPUTextureUsage={TEXTURE_BINDING:1,STORAGE_BINDING:2,COPY_SRC:4};
globalThis.GPUBufferUsage={UNIFORM:1,COPY_DST:2,STORAGE:4};
function mock(failBuffer=false,failShader=1){const resources=[];let shaderCalls=0;return {resources,device:{
 createTexture(){const resource={createView(){return {};},destroyed:0,destroy(){this.destroyed++;}};resources.push(resource);return resource;},
 createBuffer(){if(failBuffer)throw Error('rejected buffer');const resource={destroyed:0,destroy(){this.destroyed++;}};resources.push(resource);return resource;},
 queue:{writeBuffer(){}},createBindGroup(){return {};},async createComputePipelineAsync(){return {getBindGroupLayout(){return {};}};},
 createShaderModule(){const call=++shaderCalls;return {async getCompilationInfo(){return {messages:call===failShader?[{type:'error',message:'rejected shader'}]:[]};}};}
}};}
for(const failBuffer of [false,true])for(const factory of [device=>ScalarField.create(device,[128,128,128]),device=>CuSourceField.create(device,'hierarchy',0,128,2)]){
 const {device,resources}=mock(failBuffer);await assert.rejects(()=>factory(device),/rejected/);assert.ok(resources.length);assert.ok(resources.every(resource=>resource.destroyed===1));
}
{const {device,resources}=mock(false,2);await assert.rejects(()=>CuSourceField.create(device,'hierarchy',0,128,2),/rejected shader/);assert.ok(resources.length>8);assert.ok(resources.every(resource=>resource.destroyed===1));}
console.log('Five rejected partial creations release every allocated resource exactly once (including failure after nested field succeeds).');
