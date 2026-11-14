# TEST-SUPPORT log

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes; theme T6. Decided: the
  gate deals in wgpu types only, because a crate that depends on `thrum-render` cannot hand its unit tests a value
  typed by it; the device comes from `HOST`'s recipe. Test files are not all moved to one layout.
- 2026-11-15 Revised after [the first design review](../../evidence/TEST-SUPPORT/design-review.md) (four Must
  Fix). A skip is written to the standard error handle, which the harness does not capture
  ([probe](../../evidence/TEST-SUPPORT/planning/skip-line-probe.md)), and adoption is proved by run results under
  `THRUM_REQUIRE_GPU=1`, not by test lists. A test's device is the recipe's by profile and restriction, settled
  with `HOST`, so the sea's reduced-limit tests keep their inputs. Fixtures move to a second package,
  `thrum-fixtures`, which the proving ground may take as a normal dependency; the rule is `test-crate-edges`,
  settled with `GAME-SPLIT`. M2 now waits for `GAME-SPLIT:M2`.
