import init, { create_scene } from './scene.js';
import { mountControls } from './thrum-controls.js';
const canvas = document.querySelector('canvas');
const status = document.querySelector('#status');
const problems = document.querySelector('#problems');
const report = error => {
  problems.textContent = String(error);
};
try {
  await init();
  const config = await fetch('config.json').then(r => r.json());
  const query = new URLSearchParams(location.search);
  const game = await create_scene(canvas, query.get('store') ?? config.store);
  window.game = game;
  const view = mountControls(document.querySelector('#controls'), game.controls_json(), {
    edit: (key, value) => game.edit_control(key, value),
    changes: () => game.control_changes(), onError: report,
  });
  if (query.has('params')) {
    const url = query.get('params');
    try {
      const response = await fetch(url);
      if (!response.ok) throw Error(`HTTP ${response.status}`);
      const refusal = game.apply_params(await response.text());
      if (refusal) report(`${url}: ${refusal}`);
      view.refresh();
    }
    catch (error) {
      report(`Parameter file ${url}: ${error}; fix its path or contents and reopen the URL.`);
    }
  }
  document.addEventListener('visibilitychange', () => game.set_visible(!document.hidden, performance.now()));
  function frame(now) {
    try {
      game.frame(now);
      const nextStatus = game.report();
      if (status.textContent !== nextStatus) status.textContent = nextStatus;
      view.refresh();
      requestAnimationFrame(frame);
    }
    catch(error) {
      report(`Scene frame: ${error}; fix the reported object and rerun the scene command.`);
    }
  }
  requestAnimationFrame(frame);
}
catch(error) {
  status.textContent = `Scene could not start: ${error}; check that the scene command is still serving its cooked asset store, fix the reported cause and rerun it.`;
}
