// Coalesced UI reset transaction; explicit seed/clear wins over structural recovery.
export function mergeReset(previous, request) {
  return {
    clear: request.preserveClear && previous ? previous.clear : request.clear,
    newWorld: Boolean(request.newWorld || previous?.newWorld),
  };
}
