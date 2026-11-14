# TEST-SUPPORT log

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes; theme T6. Decided: the
  gate deals in wgpu types only, because a crate that depends on `thrum-render` cannot hand its unit tests a value
  typed by it; the device comes from `HOST`'s recipe. Test files are not all moved to one layout.
