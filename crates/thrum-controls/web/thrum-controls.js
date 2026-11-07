// Browser view of a thrum-controls model: builds labelled sliders, toggles, text fields and
// readouts from `Controls::describe_json`, and applies `Controls::changes_json`. The model
// owns every value, unit and label; this file only draws them and reports edits.
//
//   import { mountControls } from "./thrum-controls.js";
//   const view = mountControls(section, game.controls_json(), {
//     edit: (key, value) => game.edit_control(key, value, performance.now()),
//     changes: () => game.control_changes(),
//   });
//   // each frame: view.refresh();
//
// Element ids default to the key with dots and underscores as dashes (`sky.solar_time` ->
// `sky-solar-time`); `ids` overrides them, for pages whose ids predate the model. Two keys
// that would share an id (`sky.a_b` and `sky_a.b`) are refused through `onError`; give one
// of them an `ids` entry. Slider values have an <output> with id `<id>-value`. A slider
// whose step is 0 is continuous (`step="any"`), as on the Mac.

const STYLE_ID = "thrum-controls-style";
const STYLE = `
.tc-panel { display: flex; flex-wrap: wrap; gap: .5rem 2rem; margin-top: 1rem; }
.tc-group { border: 0; margin: 0; padding: 0; min-width: 18rem; }
.tc-group > legend { padding: 0; margin-bottom: .25rem; font-size: .8rem; font-weight: 600; opacity: .65; }
.tc-row { display: grid; grid-template-columns: 7rem minmax(8rem, 12rem) 4rem; align-items: center; gap: .5rem; min-height: 1.75rem; }
.tc-row output { display: inline; margin: 0; font-variant-numeric: tabular-nums; }
.tc-row input[type=checkbox] { justify-self: start; width: 1.1rem; height: 1.1rem; }
.tc-row input[type=text] { font: inherit; }
.tc-row.tc-readout output { grid-column: 2 / span 2; }
`;

const heading = (group) => group.replaceAll("_", " ").replace(/^./, (first) => first.toUpperCase());
const defaultId = (key) => key.replace(/[^A-Za-z0-9]+/g, "-");

/**
 * Build widgets for every control into `container` and return a view.
 * @param {HTMLElement} container
 * @param {string} described JSON from `Controls::describe_json`.
 * @param {{edit: (key: string, value: number|boolean|string) => unknown,
 *          changes: () => string|undefined,
 *          ids?: Record<string, string>,
 *          onError?: (error: unknown) => void}} options
 */
export function mountControls(container, described, { edit, changes, ids = {}, onError = (error) => { throw error; } }) {
  if (!document.getElementById(STYLE_ID)) {
    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = STYLE;
    document.head.prepend(style);
  }
  const { controls } = JSON.parse(described);
  const widgets = new Map();
  const panel = document.createElement("div");
  panel.className = "tc-panel";
  const groups = new Map();
  const owners = new Map();
  for (const control of controls) {
    const id = ids[control.key] ?? defaultId(control.key);
    const owner = owners.get(id);
    if (owner !== undefined) {
      onError(new Error(`controls ${owner} and ${control.key} would share the element id "${id}"; give one of them an ids entry`));
      continue;
    }
    owners.set(id, control.key);
    let group = groups.get(control.group);
    if (!group) {
      group = document.createElement("fieldset");
      group.className = "tc-group";
      if (control.group) {
        const legend = document.createElement("legend");
        legend.textContent = heading(control.group);
        group.append(legend);
      }
      groups.set(control.group, group);
      panel.append(group);
    }
    const row = document.createElement(control.kind === "readout" ? "div" : "label");
    row.className = `tc-row tc-${control.kind}`;
    row.dataset.control = control.key;
    const name = document.createElement("span");
    name.textContent = control.label;
    row.append(name);
    const widget = { control, input: null, output: null };
    if (control.kind === "slider") {
      const input = document.createElement("input");
      Object.assign(input, { type: "range", id, min: control.min, max: control.max, step: control.step > 0 ? control.step : "any", value: control.value });
      const output = document.createElement("output");
      Object.assign(output, { id: `${id}-value`, htmlFor: id, textContent: control.text });
      input.addEventListener("input", () => send(control.key, input.valueAsNumber));
      row.append(input, output);
      Object.assign(widget, { input, output });
    } else if (control.kind === "toggle") {
      const input = document.createElement("input");
      Object.assign(input, { type: "checkbox", id, checked: control.value });
      input.addEventListener("change", () => send(control.key, input.checked));
      row.append(input);
      widget.input = input;
    } else if (control.kind === "text") {
      const input = document.createElement("input");
      Object.assign(input, { type: "text", id, value: control.value });
      input.addEventListener("change", () => send(control.key, input.value));
      row.append(input);
      widget.input = input;
    } else {
      const output = document.createElement("output");
      Object.assign(output, { id, textContent: control.text });
      output.setAttribute("aria-live", "off");
      row.append(output);
      widget.output = output;
    }
    widgets.set(control.key, widget);
    group.append(row);
  }
  container.append(panel);

  const apply = (json) => {
    if (!json) return;
    for (const { key, value, text } of JSON.parse(json).changed) {
      const widget = widgets.get(key);
      if (!widget) continue;
      const { control, input, output } = widget;
      if (control.kind === "slider" && input.valueAsNumber !== value) input.value = value;
      if (control.kind === "toggle") input.checked = value;
      if (control.kind === "text" && document.activeElement !== input) input.value = value;
      if (output && output.textContent !== text) output.textContent = text;
    }
  };
  function send(key, value) {
    try {
      edit(key, value);
      apply(changes());
    } catch (error) {
      onError(error);
    }
  }
  return {
    /** Show what changed in the model since the last refresh. Cheap when nothing did. */
    refresh() { apply(changes()); },
    /** Edit a control as a person would (display units), then refresh. */
    set(key, value) { send(key, value); },
    /** The input or output element for a control key. */
    element(key) { const widget = widgets.get(key); return widget?.input ?? widget?.output ?? null; },
  };
}
