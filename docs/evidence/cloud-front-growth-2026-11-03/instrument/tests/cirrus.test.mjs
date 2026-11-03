import assert from 'node:assert/strict';
import {cirrusControls, CIRRUS_HEADS} from '../src/cirrus.js';
const a=cirrusControls(), b=cirrusControls();
assert.deepEqual(a,b);
assert.notDeepEqual(a,cirrusControls(72));
const widths=new Set(), lengths=new Set(), altitudes=new Set();
for(let i=0;i<CIRRUS_HEADS;i++){
  const [x,z,w,weight,L,c,s,y]=a.subarray(i*8,i*8+8);
  assert.ok([x,z,w,weight,L,c,s,y].every(Number.isFinite));
  assert.ok(Math.abs(c*c+s*s-1)<1e-6);
  assert.ok(w>=160 && w<=330 && L>=1700 && L<=3900);
  assert.ok(weight>0 && weight<=1 && y>=9500 && y<=10500);
  // Full seeded centreline, including the bounded lateral bend, stays inside chart.
  for(const t of [0,.5,1]){
    const bend=(i%2===0?1:-1)*.09*L*t*t;
    const px=x+c*L*t-s*bend, pz=z+s*L*t+c*bend;
    assert.ok(Math.abs(px)+w*2<6500 && Math.abs(pz)+w*2<6000);
  }
  widths.add(w);lengths.add(L);altitudes.add(y);
}
assert.equal(widths.size,CIRRUS_HEADS);
assert.equal(lengths.size,CIRRUS_HEADS);
assert.equal(altitudes.size,CIRRUS_HEADS);
assert.ok(a.subarray(CIRRUS_HEADS*8).every(v=>v===0));
for(const wind of [-12,0,12])assert.ok(cirrusControls(71,wind).every(Number.isFinite));
console.log('Finite, nonrepeating cirrus controls, chart clearance and replay checks passed');
