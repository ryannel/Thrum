import { bindControls } from './thrum-controls.js';

// Drive the engine's wasm page interface. A page may decorate frames or provide
// its own status text without handling timestamps, visibility or replacement.
export function mountPage(game, {
  controls, controlView, recoveryButton, beforeFrame, afterFrame,
  canFrame = () => true, onHidden, onGraphics, onError = error => { throw error; },
  physicalKeys = true,
} = {}) {
  const view = controlView ?? (controls && bindControls(controls, game, { onError }));
  const listeners = new AbortController();
  const listen = (target, type, action) =>
    target.addEventListener(type, action, { signal: listeners.signal });
  let stopped = false;
  let suspended = false;
  let drawing = false;
  let raf = 0;
  let previousUnavailable = false;
  let requestError;
  const restoreLabel = recoveryButton?.textContent;
  const visible = () => !document.hidden && !suspended;

  function cancelPending() {
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }
  function stop() {
    stopped = true;
    cancelPending();
    listeners.abort();
  }
  function fail(error, phase) {
    stop();
    onError(error, phase);
  }
  function schedule() {
    if (!raf && !drawing && !stopped && visible() && canFrame()) {
      raf = requestAnimationFrame(draw);
    }
  }
  function graphics() {
    const fault = game.graphics_fault();
    const recovering = game.recovering();
    const error = game.graphics_recovery_error() ?? requestError;
    const unavailable = Boolean(fault || recovering);
    if (recoveryButton) {
      recoveryButton.hidden = !unavailable;
      recoveryButton.disabled = recovering;
      const label = recovering ? 'Restoring graphics…' : restoreLabel;
      if (recoveryButton.textContent !== label) recoveryButton.textContent = label;
    }
    onGraphics?.({ fault, recovering, error, unavailable, wasUnavailable: previousUnavailable });
    previousUnavailable = unavailable;
    if (!unavailable) requestError = undefined;
    return unavailable;
  }
  async function draw(now) {
    raf = 0;
    if (stopped || !visible() || !canFrame()) return;
    drawing = true;
    try {
      beforeFrame?.(now);
      game.frame(now);
      view?.refresh();
      const unavailable = graphics();
      const next = afterFrame?.(now, unavailable);
      if (next instanceof Promise) await next;
    } catch (error) {
      fail(error, 'frame');
    } finally {
      drawing = false;
    }
    schedule();
  }
  function visibility() {
    if (stopped) return;
    try {
      if (!visible()) {
        cancelPending();
        if (physicalKeys) game.focus_lost();
        onHidden?.();
      }
      game.set_visible(visible(), performance.now());
      schedule();
    } catch (error) {
      fail(error, 'lifecycle');
    }
  }
  if (recoveryButton) listen(recoveryButton, 'click', () => {
    requestError = undefined;
    try {
      game.request_graphics_recovery();
    } catch (error) {
      requestError = String(error);
    }
    graphics();
    schedule();
  });
  if (physicalKeys) {
    const acceptsKeys = event => !(event.target instanceof Element &&
      event.target.closest('input, button, select, textarea, [contenteditable]'));
    const key = event => {
      if (!acceptsKeys(event)) return;
      try {
        if (game.key(event.code, event.type === 'keydown', performance.now())) event.preventDefault();
      } catch (error) {
        fail(error, 'input');
      }
    };
    const release = () => {
      try {
        game.focus_lost();
      } catch (error) {
        fail(error, 'input');
      }
    };
    listen(window, 'keydown', key);
    listen(window, 'keyup', key);
    listen(window, 'blur', release);
    listen(document, 'focusin', event => { if (!acceptsKeys(event)) release(); });
  }
  listen(document, 'visibilitychange', visibility);
  visibility();
  return {
    cancelPending,
    schedule,
    setSuspended(value) { suspended = value; visibility(); },
    stop,
  };
}
