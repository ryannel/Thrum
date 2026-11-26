// Canvas contacts are sent in backing pixels, against the layout that last drew.
export function mountInterfaceInput(game, canvas) {
  const mouse = new Set();
  const touches = new Set();
  canvas.style.touchAction = 'none';

  const acceptsKeys = event => !(event.target instanceof Element &&
    event.target.closest('button, input, select, textarea, [contenteditable]'));
  const key = event => {
    if (!acceptsKeys(event)) return;
    const pasteShortcut = (event.metaKey || event.ctrlKey) && event.code === 'KeyV';
    if (!pasteShortcut && !event.metaKey && !event.ctrlKey && !event.altKey) event.preventDefault();
    game.menu_key(event.code, event.type === 'keydown', event.repeat, event.shiftKey, performance.now());
  };
  window.addEventListener('keydown', key);
  window.addEventListener('keyup', key);

  function point(event) {
    const rect = canvas.getBoundingClientRect();
    return new Float64Array([
      (event.clientX - rect.left) * canvas.width / rect.width,
      (event.clientY - rect.top) * canvas.height / rect.height,
    ]);
  }

  canvas.addEventListener('mousedown', event => {
    mouse.add(event.button);
    game.menu_pointer('mouse', 'down', event.button, point(event), performance.now());
  });
  window.addEventListener('mouseup', event => {
    if (mouse.delete(event.button)) {
      game.menu_pointer('mouse', 'up', event.button, point(event), performance.now());
    }
  });
  window.addEventListener('mousemove', event => {
    if (mouse.size > 0) {
      for (const button of mouse) game.menu_pointer('mouse', 'move', button, point(event), performance.now());
    } else if (event.target === canvas) {
      game.menu_pointer('mouse', 'hover', 0, point(event), performance.now());
    }
  });
  canvas.addEventListener('contextmenu', event => event.preventDefault());

  for (const [type, phase] of [
    ['pointerdown', 'down'], ['pointermove', 'move'], ['pointerup', 'up'],
    ['pointercancel', 'cancel'], ['lostpointercapture', 'cancel'],
  ]) {
    canvas.addEventListener(type, event => {
      if (event.pointerType === 'mouse') return;
      if (event.cancelable) event.preventDefault();
      if (phase === 'down') {
        canvas.setPointerCapture(event.pointerId);
        touches.add(event.pointerId);
      } else if (!touches.has(event.pointerId)) {
        return;
      }
      if (phase === 'up' || phase === 'cancel') touches.delete(event.pointerId);
      game.menu_pointer(event.pointerType, phase, event.pointerId, point(event), performance.now());
    });
  }

  window.addEventListener('paste', event => {
    if (!acceptsKeys(event)) return;
    const text = event.clipboardData?.getData('text/plain');
    if (text) {
      event.preventDefault();
      game.menu_text(text, true, performance.now());
    }
  });
  const fragment = () => {
    const value = location.hash.slice(1);
    if (value) {
      try {
        game.menu_text(decodeURIComponent(value), false, performance.now());
      } catch (error) {
        if (!(error instanceof URIError)) throw error;
      }
    }
  };
  window.addEventListener('hashchange', fragment);
  fragment();

  return {
    interrupt() {
      mouse.clear();
      touches.clear();
      game.menu_interrupt(performance.now());
    },
  };
}
