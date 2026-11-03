import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import {SourceModel,DEFAULT_CONTROLS} from '../cloud-system-proof/src/source/model.js';
const admissionMetadata=JSON.parse(await fs.readFile(new URL('../cloud-system-proof/assets/admission/admission.json',import.meta.url)));
const initial={...DEFAULT_CONTROLS};
const turn={...initial,meanX:0,meanZ:6,shearX:0,shearZ:-.0015,uplift:.25,dryGain:0};
const drying={...turn,frontX:522,frontStart:500,frontSpeed:35,dryGain:1};
const programme={initial,commands:[{time:150,controls:turn},{time:500,controls:drying}],scope:'Two future commands entered at actual browser source100; applied to existing matter, not a loading clip. This file reproduces the same physical activation ticks.'};
const model=new SourceModel({admissionMetadata},initial);
for(const c of programme.commands)model.enqueue(c.controls,c.time);
const snapshots=[];
for(const target of [100,149.75,150,150.25,220,499.75,500,500.25,550,590,640]){
 while(model.time<target-1e-8)model.step();
 const state=model.metadata();snapshots.push({time:model.time,region:[...model.region],controls:{...model.controls},parts:state.parts,stats:state.stats,events:state.events});
 if(target===150||target===220){assert.ok(Math.abs(model.region[0]-521.76)<1e-7);assert.ok(Math.abs(model.region[2]-6.3912*(target-150))<1e-7);}
 if(target===220){const old=model.current.find(p=>p.active&&p.ID===1);assert.ok(Math.abs(old.K[0]-.948)<1e-6);const fresh=model.current.find(p=>p.active&&p.role==='BILLOW'&&p.born===170);assert.equal(fresh.K[0],0,'new billow does not inherit pre-turn shear');}
 if(target<500)assert.equal(Math.max(0,...model.current.map(p=>p.maxE)),0,'future drying must not affect existing matter early');
}
await fs.writeFile(new URL('programme.json',import.meta.url),JSON.stringify(programme,null,2)+'\n');
await fs.writeFile(new URL('cpu-replay.json',import.meta.url),JSON.stringify({scope:'CPU deterministic reproduction and independent analytic R/K checks, not browser/GPU/art or timing qualification',snapshots},null,2)+'\n');
console.log('Analytic region wind and retained/fresh shear checks pass; final active',model.current.filter(p=>p.active).length,'births',model.stats.births);
