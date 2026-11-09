const query = new URL(location.href).searchParams;
const arm = query.get('arm');
const wait = () => new Promise(resolve => setTimeout(resolve, 2));
window.cookedDraw = {done:false};
try {
  if (document.hidden) throw Error('hidden page');
  const module = await import(`/${arm}/v1_game.js`);
  await module.default();
  const canvas = document.querySelector('canvas');
  canvas.width=1920; canvas.height=1080;
  const game = await module.create_browser_game(canvas, false);
  window.cookedGame = game;
  game.set_visible(true, 0);
  game.set_paused(true, 0);
  game.attach_cooked_valley(await module.fetch_cooked_valley(location.origin));
  game.set_time_of_day(Number(query.get('solar'))/24);
  game.set_camera_yaw(Number(query.get('yaw'))*Math.PI/180);
  const deadline=performance.now()+60000;
  let warm=0;
  while(warm<16) {
    if(performance.now()>deadline) throw Error('warmup deadline');
    if(!game.route_slot_ready()) {await wait();continue;}
    game.frame(0);
    await wait();
    const state=JSON.parse(game.cooked_status());
    if(state.errors.length) throw Error(JSON.stringify(state));
    if(state.drawn===1) warm++;
  }
  while(!game.route_slot_ready()) await wait();
  game.begin_submitted_capture(1);
  game.frame(0);
  let image;
  while(!(image=game.next_submitted_image())) {
    if(performance.now()>deadline) throw Error('submitted readback deadline');
    await wait();
  }
  const metadata=JSON.parse(image.metadata);
  const status=JSON.parse(game.cooked_status());
  if(!metadata.surface_presented || metadata.tick!==0 || image.pixels.length!==1920*1080*4) throw Error('invalid submitted image');
  const response=await fetch('/capture',{method:'POST',headers:{'X-Frame':JSON.stringify({metadata,status})},body:image.pixels});
  if(!response.ok) throw Error('image export failed');
  window.cookedDraw={done:true,metadata,status};
} catch(error) {window.cookedDraw={done:true,error:String(error)};}
