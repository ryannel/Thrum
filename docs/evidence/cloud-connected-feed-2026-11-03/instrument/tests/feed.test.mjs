import assert from 'node:assert/strict';
import { SourceDriver } from '../src/source.js';
import { ThermalDriver } from '../src/thermals.js';
const weather={moist:1,lift:2,wind:3,windIntegral:0};
const source=new SourceDriver(71);source.reset(0,weather,false,true);
let maximum=0;
for(let tick=1;tick<=14400;tick++){
 const t=tick*.25;source.update(t,.25,{...weather,windIntegral:3*t});maximum=Math.max(maximum,source.pulses.length);
 for(const p of source.pulses)assert.ok(Math.hypot(2*p.x,2*p.z)<=200+1e-9,'Every real source offset lies in shared pickup catchment');
}
assert.ok(source.serial>800&&maximum<32,'Long-run maximum forcing retains bounded live source records');
source.reset(0,weather,false,false);source.spawn(0);assert.ok(Math.hypot(2*source.pulses[0].x,2*source.pulses[0].z)>200,'Diagnostic legacy placement remains available');
const pulse={serial:1,id:'cell:71/source:1',birth:0,x:0,z:0,radius:430};
const thermal=new ThermalDriver();thermal.reset(0,{...weather,lift:1},[pulse],true,true);thermal.update(180,180,{...weather,lift:1},[pulse]);
assert.ok(thermal.data[1]-3*thermal.data[5]>1320,'Fixture mature roll support excludes root');
assert.equal(thermal.bands[2*49],1,'Mature conduit still admits the root height band');
const legacy=new ThermalDriver();legacy.reset(0,{...weather,lift:1},[pulse],true,false);legacy.update(180,180,{...weather,lift:1},[pulse]);assert.equal(legacy.bands[2*49],0,'Old head-only bands remain diagnostic');
console.log('Bounded catchment, maximum forcing capacity, legacy reset and mature conduit admission passed');
