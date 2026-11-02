import assert from "node:assert/strict";
import {
  floatToHalf,
  halfToFloat,
  airTransmittance,
} from "../src/reference.js";
for (let h = 0; h < 65536; h++) {
  const v = halfToFloat(h);
  if (!Number.isNaN(v)) assert.equal(floatToHalf(v), h);
}
for (const v of [0.4, 0.49999, 0.9999, 0.00002, -2, 4, 65504])
  assert(
    Math.abs(halfToFloat(floatToHalf(v)) - v) <=
      Math.max(Math.abs(v) / 1024, 2 ** -24),
  );
assert.deepEqual(airTransmittance(0, -0.5), [0, 0, 0]);
for (const h of [0, 1000, 10000]) {
  const t = airTransmittance(h, 0.5);
  assert(t.every((v) => v >= 0 && v <= 1));
  assert(t[0] > t[2]);
}
console.log(
  "IEEE half conversion roundtrips and independent air reference checks passed",
);
