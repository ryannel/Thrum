# Exact frozen authored-source comparison

This is static-input evidence, not a complete cloud source or lifecycle.

1. Run `python3 restore-assets.py` to losslessly restore and hash-verify all four raw scalar payloads.
2. Run `node --test tests/master.test.mjs` for asset, transform and unchanged-renderer checks.
3. Run `python3 serve.py --port 8147` and open `http://localhost:8147/`. Exact browser build:adb13e8a53d0bd95.

The binary gzip files are exact transport copies. Generated raw `assets/*.r16f` and captures are ignored.
The parent README describes the actual GPU/images, sources, recipe, calibration, cost and qualification limits.
