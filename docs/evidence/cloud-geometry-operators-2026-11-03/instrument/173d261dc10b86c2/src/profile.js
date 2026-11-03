// One profile decision for GPU parameters, CPU preparation and observers.
export function sourceProfile(ui, family) {
  if (!ui.thermals) return 0;
  if (!ui.materialCore) return 1;
  if (family !== 1 || !ui.connectedFeed) return 2;
  return ui.frontMorph ? 4 : 3;
}
