# Stone circle at dawn

Seven cooked standing stones and a fallen stone on lit ground under the physically based sky, with sliders for the
Sun and a camera orbiting the circle. It is the first fresh-agent G6 creator trial, ported to the sky kit's placement
builder, ground and `./dev scene`: 58 lines of game code ([src/main.rs](src/main.rs), counting non-blank,
non-comment lines), down from 114.

```sh
./dev scene examples/stones
```

It was made by following the [starter](../starter/README.md)'s "Make your own scene" steps, and runs, reloads and
captures exactly as the starter does. [stones.params.json](stones.params.json) puts the Sun 3° above the eastern
horizon with the camera in the west, facing the sunrise.

The assets are byte-for-byte copies of the starter's; their provenance is in the starter's README.
