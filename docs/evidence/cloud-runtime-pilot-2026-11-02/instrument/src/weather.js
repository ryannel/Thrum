// CPU world authority. Optical render/source backlogs never advance or stop this clock.
const RANGES = {
  moist: [0, 1],
  dry: [0, 1],
  lift: [0, 2],
  wind: [-12, 12],
  storm: [0, 1],
};
export class Weather {
  constructor(seed = 71, initial = {}) {
    this.seed = seed;
    this.tick = 0;
    this.revision = 0;
    this.time = 0;
    this.frontX = 1e6;
    this.front = false;
    this.state = {
      moist: 0.85,
      dry: 0.08,
      lift: 1,
      wind: 3,
      storm: 0,
      ...initial,
    };
    this.target = { ...this.state };
    this.windIntegral = 0;
    this.commands = [];
    this.pending = [];
    this.history = [];
    this.events = [];
    this.remember();
  }
  command(patch) {
    for (const [k, v] of Object.entries(patch)) {
      if (
        !(k in RANGES) ||
        !Number.isFinite(v) ||
        v < RANGES[k][0] ||
        v > RANGES[k][1]
      )
        throw Error("Invalid weather command: " + k);
    }
    this.enqueue({ type: "controls", patch: { ...patch } });
  }
  enqueue(command) {
    if (this.pending.length >= 64)
      throw Error("Weather command capacity exceeded");
    this.pending.push(command);
  }
  sendFront() {
    this.enqueue({ type: "front", x: 3800, speed: 18 });
  }
  step() {
    // Integrate the preceding held interval before committing new commands.
    this.windIntegral += this.state.wind;
    if (this.front) this.frontX -= 18;
    for (const command of this.pending) {
      if (command.type === "front") {
        this.frontX = command.x;
        this.front = true;
      } else Object.assign(this.target, command.patch);
      this.revision++;
      this.commands.push({
        tick: this.tick + 1,
        revision: this.revision,
        ...command,
      });
    }
    this.pending.length = 0;
    for (const k of Object.keys(this.state)) {
      const tau = k === "wind" ? 4 : 12;
      this.state[k] += (this.target[k] - this.state[k]) * -Math.expm1(-1 / tau);
    }
    this.time++;
    this.tick++;
    if (this.state.storm > 0.5 && this.tick % 13 === 0)
      this.events.push({
        id: `lightning:${this.seed}:${this.tick}`,
        time: this.time,
        kind: "lightning-potential",
        support: [0, 0, 4000],
        activity: this.state.storm,
      });
    this.commands = this.commands.filter((c) => c.tick >= this.tick - 64);
    this.events = this.events.filter((c) => c.time >= this.time - 64);
    this.remember();
  }
  advanceTo(t, maxSteps = 128) {
    if (!Number.isFinite(t) || t < this.time)
      throw Error("Weather cannot rewind without a new world epoch");
    let steps = 0;
    while (this.time + 1 <= t + 1e-8 && steps < maxSteps) {
      this.step();
      steps++;
    }
    return steps;
  }
  remember() {
    const f = Object.freeze({
      time: this.time,
      weatherTick: this.tick,
      revision: this.revision,
      frontX: this.frontX,
      frontActive: this.front,
      seed: this.seed,
      windIntegral: this.windIntegral,
      rainFlux: this.state.storm * 0.002,
      latestLightning: this.events.at(-1)?.id ?? null,
      ...this.state,
    });
    this.history.push(f);
    if (this.history.length > 66) this.history.shift();
  }
  snapshot(time = this.time) {
    return this.sample(time);
  }
  sample(t) {
    if (
      !Number.isFinite(t) ||
      t < this.history[0].time - 1e-6 ||
      t >= this.time + 1 + 1e-6
    )
      throw Error("Source is outside bounded committed weather history");
    let a = this.history[0];
    for (const f of this.history) {
      if (f.time > t) break;
      a = f;
    }
    // Commands at the next barrier cannot change this half-open interval.
    const dt = Math.max(0, t - a.time);
    return {
      ...a,
      time: t,
      inputTick: a.weatherTick,
      frontX: a.frontActive ? a.frontX - 18 * dt : a.frontX,
      windIntegral: a.windIntegral + a.wind * dt,
    };
  }
}
export class WorldClock {
  constructor() {
    this.requested = 0;
    this.time = 0;
    this.epoch = 1;
  }
  step(realDelta, rate, playing, weather, advanceTarget = null) {
    if (advanceTarget !== null)
      this.requested = Math.max(this.requested, advanceTarget);
    else if (playing) this.requested += Math.max(0, realDelta) * rate;
    weather.advanceTo(this.requested, 128);
    this.time = Math.min(this.requested, weather.time + 0.999999);
    return this.time;
  }
  reset() {
    this.requested = 0;
    this.time = 0;
    this.epoch++;
  }
}
export function validateProvider(profile, provider) {
  const p = {
    volume: {
      visible: true,
      coefficients: true,
      runtime: true,
      attenuation: true,
    },
    shadow: {
      visible: false,
      coefficients: false,
      runtime: true,
      attenuation: true,
    },
    clear: {
      visible: false,
      coefficients: true,
      runtime: false,
      attenuation: false,
    },
  }[provider];
  if (!p) throw Error("Unknown provider");
  if (profile === "full" && (!p.visible || !p.coefficients || !p.runtime))
    throw Error(
      "Full sky requires live 3D coefficient and visible-cloud capabilities",
    );
  if (profile === "shadow" && !p.attenuation)
    throw Error("Shadow profile requires an attenuation provider");
  return { ...p };
}
export function reaction(q, S, D, dt) {
  const k = S + D;
  return k === 0 ? q : q * Math.exp(-k * dt) + (S / k) * -Math.expm1(-k * dt);
}
