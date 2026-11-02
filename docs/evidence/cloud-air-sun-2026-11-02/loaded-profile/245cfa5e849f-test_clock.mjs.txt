import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const text=await readFile(new URL('./clock.js',import.meta.url),'utf8');
const {PreviewClock}=await import('data:text/javascript;base64,'+Buffer.from(text).toString('base64'));
const clock=new PreviewClock(1800,40);
clock.play(1000);assert.equal(clock.sample(2250),41.25);
// A delayed frame is not the clock: rate switches retain all elapsed old-rate time.
clock.setRate(64,2250);assert.equal(clock.sample(2250),41.25);assert.equal(clock.sample(3250),105.25);
clock.setRate(1,3500);assert.equal(clock.sample(3500),121.25);assert.equal(clock.sample(4500),122.25);
clock.pause(4500);assert.equal(clock.sample(60000),122.25);
clock.seek(10,60000);clock.play(60000);assert.equal(clock.sample(61000),11);
clock.seek(1799,61000);clock.setRate(64,61000);assert.equal(clock.sample(62000),1800);
assert.throws(()=>clock.setRate(0,62000));assert.throws(()=>clock.seek(NaN,62000));
console.log('Preview clock checks pass: rate changes, skip, pause, replay and finite end.');
