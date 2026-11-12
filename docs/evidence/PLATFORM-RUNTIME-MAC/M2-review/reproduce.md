Commands executed by the independent reviewer at 53137ce1506a37ec27c04082701d0e9ac1061bd9

```
RUSTC_WRAPPER='' cargo build --locked --offline --release -p v1-game -p thrum-cook
target/release/thrum-cook cook examples/v1-game/assets/showcase docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/valley
python3 -m unittest discover -s scripts -p 'test_mac_m2*.py' -v
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-off-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111/off-traced --output docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/known-off.json
python3 scripts/mac_m2_presented.py --input docs/evidence/PLATFORM-RUNTIME-MAC/m2-presented-on-traced.tsv.gz --arm docs/evidence/render/mac-m2-20261111 --output docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/known-on.json
python3 scripts/mac_m2_presented.py --input docs/evidence/render/mac-m2-presented-20261111/attempt-3-off/presented.xml --arm docs/evidence/render/mac-m2-presented-20261111/attempt-3-off --output docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/retained-off.json
python3 scripts/mac_m2_presented.py --input docs/evidence/render/mac-m2-presented-20261111/resumed-on-traced/presented.xml --arm docs/evidence/render/mac-m2-presented-20261111/resumed-on-traced --output docs/evidence/PLATFORM-RUNTIME-MAC/M2-review/retained-on.json
```

Historical known answers return 2 for incomplete coverage; later witnesses return 0. This is the expected distinction.

Native runs use the installed 1.7.1 `build/scripts/agent_loop.py lock gpu --worktree . --` prefix. GPU access runs outside the sandbox. No trace allowance is renewed by this review.

The two retained 10 fps strips sample seconds 3–7.1 of their original host-time movies, using `ffmpeg -ss 3 -i <preview-1x.mp4> -t 4.1 -vf 'fps=10,scale=384:216,tile=5x9' -frames:v 1 <strip.png>`. Read left to right then top to bottom; unused final tiles are black padding. These are sampled visual inspection, not watched real-time motion, lossless source verification, or panel proof.
