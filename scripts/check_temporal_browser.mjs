// W1-W7 read the game's submitted diagnostic, never an endpoint side render.
import fs from 'node:fs/promises';
import path from 'node:path';
import {createHash} from 'node:crypto';

export async function qualifyTemporal(cdp, origin, output, timeout, assetsOrigin, waitFor, files, directory) {
  for (const [url,file,type] of [
    ['/temporal.html','temporal.html','text/html'],
    ['/temporal-witness.json','temporal-witness.json','application/json'],
  ]) files.set(url,{bytes:await fs.readFile(path.join(directory,file)),type});
  const definition=await fs.readFile(new URL('../crates/thrum-render/src/synthetic/temporal/witness.json',import.meta.url));
  const fixture=JSON.parse(definition);
  if(!files.get('/temporal-witness.json').bytes.equals(definition))throw Error('stale temporal fixture');
  const {targetId}=await cdp.call('Target.createTarget',{url:'about:blank'});
  const {sessionId}=await cdp.call('Target.attachToTarget',{targetId,flatten:true});
  const faults=[];
  let wasmResponse;
  cdp.event=p=>{
    if(p.sessionId!==sessionId)return;
    if(p.method==='Runtime.exceptionThrown')faults.push(p.params);
    if(p.method==='Runtime.consoleAPICalled' && p.params.type==='error'){
      const message=p.params.args.map(a=>a.value??a.description).join(' ');
      if(!message.includes('THRUM_TEMPORAL_ACQUIRE_ONCE'))faults.push(message);
    }
    if(p.method==='Network.responseReceived' && p.params.response.url===`${origin}/v1_game_bg.wasm`)wasmResponse=p.params;
  };
  const evaluate=async expression=>{
    const r=await cdp.call('Runtime.evaluate',{expression,returnByValue:true},sessionId);
    if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value;
  };
  try {
    await cdp.call('Runtime.enable',{},sessionId);await cdp.call('Page.enable',{},sessionId);
    await cdp.call('Network.enable',{maxResourceBufferSize:files.get('/v1_game_bg.wasm').bytes.length*2,maxTotalBufferSize:256<<20},sessionId);
    await cdp.call('Network.setCacheDisabled',{cacheDisabled:true},sessionId);
    await cdp.call('Target.activateTarget',{targetId});
    await cdp.call('Page.navigate',{url:`${origin}/temporal.html?assets=${encodeURIComponent(assetsOrigin)}`},sessionId);
    const record=await waitFor(async()=>{
      const r=await evaluate('window.thrumTemporal');
      if(r?.error){await fs.writeFile(path.join(output,'temporal-failed.json'),JSON.stringify(r,null,2));throw Error(r.error);}return r?.done?r:undefined;
    },timeout,'submitted temporal witness');
    await fs.writeFile(path.join(output,'temporal.record.json'),JSON.stringify(record,null,2));
    const clip=await evaluate('(()=>{const r=document.querySelector("canvas").getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,scale:1}})()');
    const shot=await cdp.call('Page.captureScreenshot',{format:'png',clip},sessionId);
    await fs.writeFile(path.join(output,'temporal-C.png'),Buffer.from(shot.data,'base64'));
    if(!wasmResponse || wasmResponse.response.status!==200)throw Error('missing loaded Wasm response');
    const body=await cdp.call('Network.getResponseBody',{requestId:wasmResponse.requestId},sessionId);
    const loadedWasm=createHash('sha256').update(Buffer.from(body.body,body.base64Encoded?'base64':'utf8')).digest('hex');
    if(loadedWasm!==files.get('/v1_game_bg.wasm').sha256)throw Error('loaded Wasm differs from built artifact');
    record.actualWasmSha256=loadedWasm;
    const r=record.rows;
    const require=(condition,row,detail)=>{if(!condition)throw Error(`${row}: ${detail}`);};
    require(r.W1.one>0,'W1','invalid geometry was never marked');
    require(r.W2.one===0 && r.W2.geometryMotion===0,'W2','steady geometry did not clear reactive and motion');
    require(r.W3.partial>0 && r.W3.zero>0,'W3','cooked volume alpha or uncovered zero missing');
    require(r.W4sun.sun_probe[0]===255 && r.W5sun.sun_probe[0]===0,'W4/W5','Sun probe did not mark then clear');
    require(r.W5stars?.starCleared>0,'W4/W5','star-only edit did not mark then clear');
    require(r.W6.failures===1 && r.W6.C.identity.history_predecessor===r.W6.A.identity.history_sample,'W6','abandoned view became predecessor');
    require(r.W6.C.identity.history_sample===r.W6.A.identity.history_sample+1,'W6','abandoned view advanced sample cursor');
    require(r.W7.geometryMotion>0,'W7','camera pan relative to A wrote no motion');
    require(record.omissions===r.W6.failures,'capture','unexplained capture omission beyond the deliberately cancelled reservation');
    require(record.frames.every((f,i)=>i===0 || f.metadata.submission_id===record.frames[i-1].metadata.submission_id+1),'capture','missing submitted image');
    require(!faults.length,'runtime',JSON.stringify(faults));
    await fs.writeFile(path.join(output,'temporal.json'),JSON.stringify({fixture,record,faults,scope:'Actual hardware Chrome game frame path, submitted reactive/depth/motion diagnostic through T2; no panel or cadence claim'},null,2));
    return {case:'temporal',passed:true,rows:Object.keys(fixture.rows)};
  } finally { await cdp.call('Target.closeTarget',{targetId}); }
}
