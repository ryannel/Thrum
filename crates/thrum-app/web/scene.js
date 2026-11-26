import init, * as scene from './scene.js';
import { bindControls } from './thrum-controls.js';
import { mountPage } from './host-page.js';
import { mountInterfaceInput } from './interface-input.js';
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
  const create = scene.create_game_scene ?? scene.create_scene;
  const game = await create(canvas, query.get('store') ?? config.store);
  window.game = game;
  const recover = document.querySelector('#recover');
  document.title = game.title();
  canvas.setAttribute('aria-label', game.title());
  const input = typeof game.interface_pointer === 'function' ?
    mountInterfaceInput(canvas, {
      key: (...args) => game.interface_key(...args),
      pointer: (...args) => game.interface_pointer(...args),
      interrupt: (...args) => game.interface_interrupt(...args),
    }) : undefined;
  if (input) window.addEventListener('blur', input.interrupt);
  const view = bindControls(document.querySelector('#controls'), game, { onError: report });
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
  mountPage(game, {
    controlView: view,
    recoveryButton: recover,
    physicalKeys: !input,
    onHidden: input?.interrupt,
    beforeFrame() {
      if (!input) return;
      const { width, height } = canvas.getBoundingClientRect();
      const density = window.devicePixelRatio;
      const pixels = [width, height].map(side => Math.max(1, Math.round(side * density)));
      if (canvas.width !== pixels[0]) canvas.width = pixels[0];
      if (canvas.height !== pixels[1]) canvas.height = pixels[1];
      game.set_interface_output(density);
    },
    afterFrame() {
      const nextStatus = game.report();
      if (status.textContent !== nextStatus) status.textContent = nextStatus;
    },
    onGraphics({ error, recovering }) {
      if (error && !recovering) report(error);
    },
    onError(error) {
      report(`Scene frame: ${error}; fix the reported object and rerun the scene command.`);
    },
  });
}
catch(error) {
  status.textContent = `Scene could not start: ${error}; check that the scene command is still serving its cooked asset store, fix the reported cause and rerun it.`;
}
