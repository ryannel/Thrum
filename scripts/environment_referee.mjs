// Independent f64 referees. This file runs inside hardware Chrome's page.
async function environmentReferee() {
  const state = globalThis.__environment;
  const lent = state.groups.findLast(item => item.descriptor.label === 'lent environment read-only products');
  if (!lent || !globalThis.__environmentLibrary) throw new Error('Missing lent products');
  const device = lent.device;
  const library = globalThis.__environmentLibrary;
  const entry = binding => lent.descriptor.entries.find(item => item.binding === binding).resource;
  const frame = entry(0).buffer;
  const originalFrame = new Float32Array(state.buffers.get(frame).slice().buffer);
  const texture = binding => state.views.get(entry(binding)).texture;
  const rows = {};
  const failures = [];
  const report = { target: 'chrome-webgpu', rows, failures, adapter: device.limits.maxSampledTexturesPerShaderStage, reference: 'Independent f64 filtered decode and quadrature. Production charts. No Metal output used.' };
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const query = async (expression, queries, source = library, group = lent.value) => {
    device.pushErrorScope('validation');
    try {
    const module = device.createShaderModule({ code: `${source}\n@group(1) @binding(0) var<storage,read> queries:array<vec4<f32>>;\n@group(1) @binding(1) var<storage,read_write> results:array<vec4<f32>>;\n@compute @workgroup_size(64) fn observe(@builtin(global_invocation_id) id:vec3<u32>){if id.x>=arrayLength(&queries){return;} let q=queries[id.x]; results[id.x]=${expression};}` });
    const layout = device.createBindGroupLayout({ entries: [0, 1].map(binding => ({ binding, visibility: GPUShaderStage.COMPUTE, buffer: { type: binding === 0 ? 'read-only-storage' : 'storage' } })) });
    const pipeline = device.createComputePipeline({ layout: device.createPipelineLayout({ bindGroupLayouts: [lent.descriptor.layout, layout] }), compute: { module, entryPoint: 'observe' } });
    const bytes = new Float32Array(queries.flat());
    const input = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
    const output = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC });
    const readback = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
    device.queue.writeBuffer(input, 0, bytes);
    const group1 = device.createBindGroup({ layout, entries: [{ binding: 0, resource: { buffer: input } }, { binding: 1, resource: { buffer: output } }] });
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline); pass.setBindGroup(0, group); pass.setBindGroup(1, group1); pass.dispatchWorkgroups(Math.ceil(queries.length / 64)); pass.end();
    encoder.copyBufferToBuffer(output, 0, readback, 0, bytes.byteLength);
    device.queue.submit([encoder.finish()]);
    await readback.mapAsync(GPUMapMode.READ);
    const values = Array.from(new Float32Array(readback.getMappedRange().slice(0)));
    readback.unmap(); input.destroy(); output.destroy(); readback.destroy();
    return queries.map((_, i) => values.slice(i * 4, i * 4 + 4));
    } finally {
      const error = await device.popErrorScope();
      if (error) throw new Error('Environment consumer GPU validation failed: '+error.message);
    }
  };
  const exposedEntries = lent.descriptor.entries.map(item => ({ ...item }));
  const exposure = device.createBuffer({ size: 16, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  device.queue.writeBuffer(exposure, 0, new Float32Array([2, 0, 0, 0]));
  exposedEntries.find(item => item.binding === 10).resource = { buffer: exposure };
  const exposed = device.createBindGroup({ layout: lent.descriptor.layout, entries: exposedEntries });
  const half = bits => {
    const sign = bits & 0x8000 ? -1 : 1;
    const exponent = bits >> 10 & 31;
    return sign * (exponent ? (1 + (bits & 1023) / 1024) * 2 ** (exponent - 15) : (bits & 1023) * 2 ** -24);
  };
  const halfBits = value => {
    if (value === 0) return 0;
    const exponent = Math.max(-14, Math.floor(Math.log2(Math.abs(value))));
    const mantissa = Math.round(value / 2 ** (exponent - 10));
    return mantissa === 2048 ? (exponent + 16) * 1024 : (exponent + 14) * 1024 + mantissa;
  };
  const step = value => 2 ** (Math.max(-14, Math.floor(Math.log2(Math.abs(value)))) - 10);
  const readTexture = async target => {
    const stride = Math.ceil(target.width * 8 / 256) * 256;
    const bytes = stride * target.height * target.depthOrArrayLayers;
    const buffer = device.createBuffer({ size: bytes, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
    const encoder = device.createCommandEncoder();
    encoder.copyTextureToBuffer({ texture: target }, { buffer, bytesPerRow: stride, rowsPerImage: target.height }, [target.width, target.height, target.depthOrArrayLayers]);
    device.queue.submit([encoder.finish()]);
    await buffer.mapAsync(GPUMapMode.READ);
    const words = new Uint16Array(buffer.getMappedRange());
    const values = [];
    for (let layer = 0; layer < target.depthOrArrayLayers; layer++) for (let y = 0; y < target.height; y++) for (let x = 0; x < target.width; x++) {
      const offset = ((layer * target.height + y) * stride + x * 8) / 2;
      values.push(Array.from(words.subarray(offset, offset + 4), half));
    }
    buffer.unmap(); buffer.destroy(); return values;
  };
  const writeField = (target, evaluate) => {
    const data = new Uint16Array(target.width * target.height * target.depthOrArrayLayers * 4);
    for (let z = 0; z < target.depthOrArrayLayers; z++) for (let y = 0; y < target.height; y++) for (let x = 0; x < target.width; x++) data.set(evaluate(x, y, z).map(halfBits), ((z * target.height + y) * target.width + x) * 4);
    device.queue.writeTexture({ texture: target }, data, { bytesPerRow: target.width * 8, rowsPerImage: target.height }, [target.width, target.height, target.depthOrArrayLayers]);
  };
  const setFrame = (degrees, altitude = 2, azimuth = 0, lux = 1) => {
    const f = originalFrame.slice();
    const radians = degrees * Math.PI / 180;
    f.set([0, altitude, 0, altitude], 16);
    f.set([Math.cos(radians) * Math.cos(azimuth), Math.sin(radians), Math.cos(radians) * Math.sin(azimuth), lux], 20);
    f.set([1, 1, 1, 1], 24);
    const horizon = Math.asin(-Math.sqrt(altitude * (2 * 6371000 + altitude)) / (6371000 + altitude));
    f.set([Math.sin(horizon), horizon, Math.PI / 2 - horizon, Math.PI / 2 + horizon], 28);
    device.queue.writeBuffer(frame, 0, f);
    return f;
  };
  const lutPipelines = state.pipelines.filter(item => item.descriptor.label === 'atmosphere LUT').slice(-4);
  const lutGroups = state.groups.filter(item => item.descriptor.label === 'atmosphere LUT resources').slice(-4);
  const nightControl = lutGroups[2].descriptor.entries.find(item => item.binding === 7).resource.buffer;
  device.queue.writeBuffer(nightControl, 0, new Uint32Array([0]));
  const dispatch = (encoder, pipeline, group, size) => {
    const pass = encoder.beginComputePass(); pass.setPipeline(pipeline); pass.setBindGroup(0, group); pass.dispatchWorkgroups(...size); pass.end();
  };
  const produce = async () => {
    const encoder = device.createCommandEncoder();
    for (const index of [0, 1, 3, 2]) {
      const extents = [[256, 64], [512, 32], [192, 108], [128, 1]][index];
      dispatch(encoder, lutPipelines[index].value, lutGroups[index].value, index === 3 ? [128, 1, 1] : [Math.ceil(extents[0] / 8), Math.ceil(extents[1] / 8), 1]);
    }
    const aerial = state.pipelines.findLast(item => item.descriptor.compute.entryPoint === 'aerial_main');
    const aerialGroup = state.groups.findLast(item => item.descriptor.label === 'atmosphere aerial resources');
    if (!aerial || !aerialGroup) throw new Error('Production aerial pipeline missing');
    dispatch(encoder, aerial.value, aerialGroup.value, [4, 4, 1]);
    const diffuse = state.pipelines.findLast(item => item.descriptor.compute.entryPoint === 'diffuse_main');
    const diffuseGroup = state.groups.findLast(item => item.descriptor.label === 'atmosphere diffuse resources');
    if (diffuse && diffuseGroup) dispatch(encoder, diffuse.value, diffuseGroup.value, [1, 1, 1]);
    device.queue.submit([encoder.finish()]); await device.queue.onSubmittedWorkDone();
  };
  const skyUV = (direction, altitude = 2, azimuth = 0) => {
    const length = Math.hypot(...direction.slice(0, 3));
    const y = direction[1] / length;
    const horizon = -Math.sqrt(altitude * (2 * 6371000 + altitude)) / (6371000 + altitude);
    const tangent = Math.asin(horizon);
    const angle = Math.asin(Math.max(-1, Math.min(1, y)));
    const row = y >= horizon ? 54.5 + 53 * Math.sqrt(Math.max(0, Math.min(1, (angle - tangent) / (Math.PI / 2 - tangent)))) : 53.5 - 53 * Math.sqrt(Math.max(0, Math.min(1, (tangent - angle) / (tangent + Math.PI / 2))));
    let longitude = Math.atan2(direction[2], direction[0]) - azimuth;
    longitude = Math.atan2(Math.sin(longitude), Math.cos(longitude));
    if (direction[0] ** 2 + direction[2] ** 2 <= 1e-10) longitude = 0;
    return [longitude / (2 * Math.PI) + 0.5, row / 108];
  };
  const direction = (uv, altitude = 2, azimuth = 0) => {
    const tangent = Math.asin(-Math.sqrt(altitude * (2 * 6371000 + altitude)) / (6371000 + altitude));
    const q = uv[1] < 0.5 ? (53.5 - uv[1] * 108) / 53 : (uv[1] * 108 - 54.5) / 53;
    const angle = uv[1] < 0.5 ? tangent - (tangent + Math.PI / 2) * q * q : tangent + (Math.PI / 2 - tangent) * q * q;
    const phi = (uv[0] * 2 - 1) * Math.PI + azimuth;
    return [Math.cos(angle) * Math.cos(phi), Math.sin(angle), Math.cos(angle) * Math.sin(phi), 0].map(Math.fround);
  };
  const sample = (values, width, height, uv, low = 0, high = 0, wz = 0, scales = [1 / 1024, 0]) => {
    const x = Math.max(0, Math.min(width - 1, uv[0] * width - 0.5));
    const y = Math.max(0, Math.min(height - 1, uv[1] * height - 0.5));
    const ix = Math.floor(x), iy = Math.floor(y);
    const sum = [0, 0, 0], min = [Infinity, Infinity, Infinity], max = [-Infinity, -Infinity, -Infinity], quantum = [0, 0, 0];
    for (const [z, weightZ] of [[low, 1 - wz], [high, wz]]) for (const [row, weightY] of [[iy, 1 - (y - iy)], [Math.min(iy + 1, height - 1), y - iy]]) for (const [column, weightX] of [[ix, 1 - (x - ix)], [Math.min(ix + 1, width - 1), x - ix]]) {
      const texel = values[(z * height + row) * width + column];
      for (let c = 0; c < 3; c++) {
        const value = texel[c] * scales[0] + texel[3] * scales[1];
        sum[c] += value * weightX * weightY * weightZ;
        min[c] = Math.min(min[c], value); max[c] = Math.max(max[c], value);
        quantum[c] = Math.max(quantum[c], step(texel[c]) * scales[0] + step(texel[3]) * scales[1]);
      }
    }
    return { value: sum, bound: max.map((m, c) => (m - min[c]) / 32 + 1e-5 * m + quantum[c]) };
  };
  const within = (actual, expected) => actual.slice(0, 3).every((v, c) => Math.abs(v - expected.value[c]) <= expected.bound[c]);
  const assertContrast = (values,width,height,layers,scale) => {
    let minimum=Infinity;
    values.forEach((first,index)=>{
      const x=index%width,y=Math.floor(index/width)%height,z=Math.floor(index/(width*height));
      const neighbours=[x+1<width?index+1:null,y+1<height?index+width:null,z+1<layers?index+width*height:null].filter(v=>v!==null);
      for(const next of neighbours) for(let c=0;c<3;c++) {
        const quantum=Math.max(step(first[c]),step(values[next][c]))*scale;
        const delta=Math.abs(first[c]-values[next][c])*scale;
        minimum=Math.min(minimum,delta/quantum);
      }
    });
    assert(minimum>=32,`Known-field contrast only ${minimum} stored steps`);
    return minimum;
  };
  const run = async (name, work) => {
    try { rows[name] = { status: 'pass', ...await work() }; }
    catch (error) { rows[name] = { status: 'fail', error: String(error) }; failures.push(`${name}: ${error}`); }
  };
  assert((await query('vec4(environment_storage_scale())',[[0,0,0,0]],library,exposed))[0][0]===2,'Non-unit exposure fixture missing');
  await run('H2', async () => {
    let count = 0;
    const horizon = -Math.acos(6371000 / 6371002) * 180 / Math.PI;
    const views = [[90, 0], [15, 0], [15, 90], [15, 180], [1, 0], [1, 90], [1, 180], [-1, 0], [-15, 0], [horizon - 0.005, 0], [horizon + 0.005, 0]];
    const queries = views.map(([e, a]) => [Math.cos(e * Math.PI / 180) * Math.cos(a * Math.PI / 180), Math.sin(e * Math.PI / 180), Math.cos(e * Math.PI / 180) * Math.sin(a * Math.PI / 180), 0].map(Math.fround));
    for (const elevation of [45, 0, -3, -6, -9, -12]) {
      setFrame(elevation); await produce();
      const field = await readTexture(texture(4));
      const actual = await query('vec4(environment_sky_radiance(q.xyz),1.0)', queries);
      actual.forEach((value, i) => assert(within(value, sample(field, 192, 108, skyUV(queries[i]))), `H2 live elevation ${elevation} query ${i}: ${value}`));
      count += queries.length;
    }
    const old = 'textureSampleLevel(environment_sky,aerial_sampler,uv,0.0)';
    assert(library.includes(old), 'H2 wrong-lookup mutation missing');
    const wrong = [
      'textureLoad(environment_sky,vec2<i32>(floor(uv*size)),0)',
      'textureSampleLevel(environment_sky,aerial_sampler,vec2(uv.x,(floor(uv.y*size.y)+0.5)/size.y),0.0)',
      'textureSampleLevel(environment_sky,aerial_sampler,vec2((floor(uv.x*size.x)+0.5)/size.x,uv.y),0.0)',
    ];
    for (const azimuth of [0, 1]) {
      setFrame(0, 2, azimuth);
      writeField(texture(4), (x, y) => [1 + x % 2 + y % 2, 1 + x % 2 + 2 * (y % 2), 1 + 2 * (x % 2) + y % 2, 0]);
      const field = await readTexture(texture(4));
      assertContrast(field,192,108,1,1/1024);
      const queries = [];
      for (const y of [20, 52, 54, 80]) for (const wy of [0.25, 0.5, 0.75]) for (const wx of [0.25, 0.5, 0.75]) queries.push(direction([(20.5 + wx) / 192, (y + 0.5 + wy) / 108], 2, azimuth));
      for (const group of [lent.value, exposed]) {
        const actual = await query('vec4(environment_sky_radiance(q.xyz),1.0)', queries, library, group);
        actual.forEach((value, i) => assert(within(value, sample(field, 192, 108, skyUV(queries[i], 2, azimuth))), `H2 known field ${i}`));
        count += queries.length;
      }
      for (const [kind, replacement] of wrong.entries()) {
        const actual = await query('vec4(environment_sky_radiance(q.xyz),1.0)', queries, library.replace(old, replacement));
        actual.forEach((value, i) => assert(!within(value, sample(field, 192, 108, skyUV(queries[i], 2, azimuth))), `H2 wrong lookup ${kind} passed query ${i}`));
      }
    }
    // The independent eight-node spectral reference retained by P11.
    setFrame(-5.084449, 244.92267); await produce();
    const [e, a] = [20 * Math.PI / 180, 95.10459 * Math.PI / 180];
    const actual = (await query('vec4(environment_sky_radiance(q.xyz),1.0)', [[Math.cos(e) * Math.cos(a), Math.sin(e), Math.cos(e) * Math.sin(a), 0]]))[0];
    [2.982646e-5, 4.289131e-5, 8.114284e-5].forEach((v, c) => assert(Math.abs(actual[c] - v) <= 0.025 * v, `H2 P11 channel ${c}: ${actual[c]} vs ${v}`));
    return { queries: count + 1, wrong_lookups_rejected: 216, exposure: 2, bound: 'C/32 + 1e-5 M + Q' };
  });
  await run('H4', async () => {
    let count = 0;
    const queries = [];
    for (const low of [0, 15, 31]) for (const wz of [0.25, 0.5, 0.75]) for (const wy of [0.25, 0.5, 0.75]) for (const wx of [0.25, 0.5, 0.75]) {
      const first = 32000 * (low / 32) ** 2, second = 32000 * ((low + 1) / 32) ** 2;
      queries.push([(9.5 + wx) / 32, (11.5 + wy) / 32, first + (second - first) * wz, 0].map(Math.fround));
    }
    const expected = (field, q, scales) => {
      const d = Math.min(32000, q[2]);
      const low = Math.min(31, Math.floor(Math.sqrt(d / 32000) * 32));
      const first = 32000 * (low / 32) ** 2, second = 32000 * ((low + 1) / 32) ** 2;
      return sample(field, 32, 32, q, low, low + 1, Math.max(0, Math.min(1, (d - first) / (second - first))), scales);
    };
    const withClamp = [...queries, [10 / 32, 12 / 32, 40000, 0]];
    const tExpression = 'vec4(environment_aerial(vec3(0.0,2.0,-q.z),q.xy).transmission,1.0)';
    const sExpression = 'vec4(environment_aerial(vec3(0.0,2.0,-q.z),q.xy).scatter,1.0)';
    const fields = async () => {
      const scatter = await readTexture(texture(5)), transmission = await readTexture(texture(6));
      return [transmission, scatter.map((s, i) => [s[0], s[1], s[2], transmission[i][3]])];
    };
    for (const elevation of [2, 0, -2, -4, -6, -9]) {
      setFrame(elevation); await produce();
      const [transmission, scatter] = await fields();
      const t = await query(tExpression, withClamp), s = await query(sExpression, withClamp);
      withClamp.forEach((q, i) => {
        assert(within(t[i], expected(transmission, q, [1, 0])), `H4 live T ${elevation} query ${i}`);
        assert(within(s[i], expected(scatter, q, [1 / 1024, 1 / 65536])), `H4 live S ${elevation} query ${i}`);
      }); count += withClamp.length * 2;
    }
    for (const elevation of [2,0,-2,-4,-6,-9]) {
      setFrame(elevation);await produce();
      const fixed=[0,15,31].map(low=>[10.5/32,12.5/32,32000*(low/32)**2,0]);
      const [transmission,scatter]=await fields();
      const t=await query(tExpression,fixed),s=await query(sExpression,fixed);
      fixed.forEach((q,i)=>{
        for (const [value,field,scales] of [[t[i],transmission,[1,0]],[s[i],scatter,[1/1024,1/65536]]]) {
          expected(field,q,scales).value.forEach((v,c)=>assert(Math.abs(value[c]-v)<=1e-5*Math.max(1e-12,Math.abs(v)),`H4 fixed decode ${elevation} query ${i} channel ${c}`));
        }
      });
      count+=fixed.length*2;
    }
    writeField(texture(5), (x, y, z) => { x %= 2; y %= 2; z %= 2; return [1 + x + y + z, 1 + x + 2 * y + 3 * z, 1 + 3 * x + y + 2 * z, 0]; });
    writeField(texture(6), (x, y, z) => { x %= 2; y %= 2; z %= 2; return [0.25 + (x + y + z) / 16, 0.25 + (x + 2 * y + 3 * z) / 16, 0.25 + (3 * x + y + 2 * z) / 16, 0]; });
    const [transmission, scatter] = await fields();
    assertContrast(transmission,32,32,33,1);
    assertContrast(scatter,32,32,33,1/1024);
    for (const group of [lent.value, exposed]) {
      const t = await query(tExpression, withClamp, library, group), s = await query(sExpression, withClamp, library, group);
      withClamp.forEach((q, i) => {
        assert(within(t[i], expected(transmission, q, [1, 0])), `H4 known T ${i}`);
        assert(within(s[i], expected(scatter, q, [1 / 1024, 1 / 65536])), `H4 known S ${i}`);
      }); count += withClamp.length * 2;
    }
    const blend = 'let blend = clamp((distance - first_distance) / max(0.001, second_distance - first_distance), 0.0, 1.0);';
    assert(library.includes(blend), 'H4 wrong slice mutation missing');
    const nearest = library.replace('fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {', 'fn aerial_sample(distance_metres: f32, uv: vec2<f32>) -> AerialSample {\nlet nearest_uv=(floor(uv*vec2<f32>(textureDimensions(aerial_scatter)))+vec2(0.5))/vec2<f32>(textureDimensions(aerial_scatter));').replaceAll('uv, i32(low)', 'nearest_uv, i32(low)').replaceAll('uv, i32(high)', 'nearest_uv, i32(high)');
    for (const [kind, source] of [library.replace(blend, 'let blend = 0.0;'), library.replace(blend, 'let blend = 1.0;'), nearest].entries()) {
      const t = await query(tExpression, queries, source), s = await query(sExpression, queries, source);
      queries.forEach((q, i) => {
        assert(!within(t[i], expected(transmission, q, [1, 0])), `H4 wrong T lookup ${kind} passed ${i}`);
        assert(!within(s[i], expected(scatter, q, [1 / 1024, 1 / 65536])), `H4 wrong S lookup ${kind} passed ${i}`);
      });
    }
    return { queries: count, wrong_lookups_rejected: queries.length * 6, exposure: 2, bound: 'C/32 + 1e-5 M + Q' };
  });
  const rayleigh = [3.668017e-5,3.0196716e-5,2.5169671e-5,1.7741737e-5,1.3536325e-5,1.1368246e-5,9.719996e-6,7.82713e-6].map(Math.fround);
  const ozone = [5.059233e-8,1.3219056e-7,2.9659487e-7,1.0269233e-6,1.885528e-6,2.279152e-6,2.3073835e-6,1.770035e-6].map(Math.fround);
  const rgb = [[0.04619919,0.02697136,-0.05803062,-0.37374267,-0.058011713,0.25731087,0.49856007,0.66074353],[-0.042857476,-0.032444324,0.026634296,0.46285555,0.3781899,0.19463263,0.054261465,-0.04127202],[0.3275349,0.32686388,0.31622496,0.06472152,-0.020442596,-0.012335314,-0.004137213,0.0015698565]].map(row => row.map(Math.fround));
  const transmission = (point, sun) => {
    const radial = [point[0], point[1]+6371000, point[2]];
    const radius = Math.hypot(...radial), top = 6471000;
    const b = radial.reduce((sum, v, i) => sum + v*sun[i], 0);
    if (b < 0 && radius*radius-b*b < 6371000**2) return [0,0,0];
    const discriminant = b*b+top*top-radius*radius;
    if (discriminant<=0 || (radius>=top && b>=0)) return [1,1,1];
    const start = radius>top ? -b-Math.sqrt(discriminant) : 0, end = -b+Math.sqrt(discriminant), dt = (end-start)/8192;
    const columns = [0,0,0];
    for (let i=0;i<8192;i++) {
      const distance=start+(i+0.5)*dt;
      const h=Math.hypot(...radial.map((v,j)=>v+sun[j]*distance))-6371000;
      columns[0]+=Math.exp(-h/8000)*dt; columns[1]+=Math.exp(-h/1200)*dt; columns[2]+=Math.max(0,1-Math.abs(h-25000)/15000)*dt;
    }
    const spectral=rayleigh.map((r,i)=>Math.exp(-(r*columns[0]+Math.fround(4.4e-6)*columns[1]+ozone[i]*columns[2])));
    const result=rgb.map(weights=>weights.reduce((sum,w,i)=>sum+w*spectral[i],0));
    const low=Math.min(...result);
    if (low>=0) return result;
    const luminance=Math.max(0,0.2126*result[0]+0.7152*result[1]+0.0722*result[2]);
    return result.map(v=>Math.max(0,luminance+(v-luminance)*luminance/(luminance-low)));
  };
  await run('H1', async () => {
    let count=0;
    const near=[[0,0,0],[0,10,0],[0,245,0],[0,1500,0],[0,5000,0],[20000,245,0],[-20000,245,0]];
    for (const [altitude,points,elevations] of [[245,near,[-2,-0.6,0,1,30]],[100000,[[0,100000,0],[0,120000,0],[0,130000,0]],[-13,-10.07,-1,5]],[245,[[0,0,0],[0,0.1,0],[0,2,0],[0,6.15,0]],[0,0.5,1,2,5]]]) {
      for (const degrees of elevations) {
        const f=setFrame(degrees,altitude); await produce();
        const sun=Array.from(f.slice(20,23));
        for (const eye of [[0,245,16],[8132,123,-3211]]) {
          f.set([...eye,altitude],16); device.queue.writeBuffer(frame,0,f);
          const queries=points.map(p=>[eye[0]+p[0],eye[1]+p[1]-altitude,eye[2]+p[2],0]);
          for (const group of [lent.value,exposed]) {
            const actual=await query('vec4(sun_irradiance(q.xyz),1.0)',queries,library,group);
            points.forEach((p,i)=>{
              const expected=transmission(p,sun);
              expected.forEach((v,c)=>{
                const bound=points.length===4 ? [0.006,0.009,0.016][c]*Math.max(0.001,v) : 0.05*v+0.005;
                assert(Math.abs(actual[i][c]-v)<=bound,`H1 altitude ${altitude} elevation ${degrees} receiver ${i} channel ${c}: ${actual[i][c]} vs ${v}, bound ${bound}`);
              });
              if (expected.every(v=>v===0)) assert(actual[i].slice(0,3).every(v=>v===0),'H1 behind planet');
            }); count+=queries.length;
          }
        }
      }
    }
    return {queries:count,exposure:2};
  });
  const normals=[[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1],[1,1,1],[-1,1,1],[1,-1,1],[1,1,-1],[-1,-1,1],[-1,1,-1],[1,-1,-1],[-1,-1,-1]].map(n=>n.map(v=>v/Math.hypot(...n)));
  const integral = (pixels, azimuth, resolution) => {
    const result={mass:[0,0,0],coefficients:Array.from({length:9},()=>[0,0,0]),direct:Array.from({length:14},()=>[0,0,0])};
    const hm=-Math.sqrt(2*(2*6371000+2))/6371002;
    for (const [sign,span] of [[-1,1+hm],[1,1-hm]]) for (let y=0;y<resolution;y++) {
      const t=(y+0.5)/resolution,mu=hm+sign*span*t*t,weight=2*Math.PI/(resolution*resolution*2)*2*span*t;
      for (let x=0;x<resolution*2;x++) {
        const phi=2*Math.PI*(x+0.5)/(resolution*2),r=Math.sqrt(1-mu*mu);
        const n=[r*Math.cos(phi),mu,r*Math.sin(phi)];
        const light=sample(pixels,192,108,skyUV(n,2,azimuth)).value;
        const [nx,ny,nz]=n;
        const basis=[0.25,nx/2,ny/2,nz/2,15/32*(nx*nx-1/3),15/32*(ny*ny-1/3),15/16*nx*ny,15/16*nx*nz,15/16*ny*nz];
        for (let c=0;c<3;c++) {
          const energy=light[c]*weight; result.mass[c]+=energy;
          basis.forEach((b,j)=>{result.coefficients[j][c]+=energy*b;});
          normals.forEach((normal,j)=>{result.direct[j][c]+=energy*Math.max(0,n.reduce((sum,v,k)=>sum+v*normal[k],0));});
        }
      }
    }
    return result;
  };
  await run('H3', async () => {
    let count=0;
    for (const [degrees,azimuth] of [[90,0],[45,0],[45,Math.PI/2],[0,0],[-3,0],[-6,0],[-9,0],[-12,0]]) {
      setFrame(degrees,2,azimuth); await produce();
      const pixels=await readTexture(texture(4));
      const coarse=integral(pixels,azimuth,64),fine=integral(pixels,azimuth,128);
      const coefficients=await query('environment_light.sky_diffuse[u32(q.x)]',Array.from({length:9},(_,i)=>[i,0,0,0]));
      for (const group of [lent.value,exposed]) {
        const actual=await query('vec4(environment_diffuse(q.xyz),1.0)',normals.map(n=>[...n,0]),library,group);
        for (let c=0;c<3;c++) {
          const mass=fine.mass[c];
          coefficients.forEach((value,j)=>{
            const refinement=Math.abs(coarse.coefficients[j][c]-fine.coefficients[j][c]);
            assert(refinement<=0.0025*mass,'H3 coefficient unresolved');
            assert(Math.abs(value[c]-fine.coefficients[j][c])<=0.01*mass+2*refinement+1e-12,'H3 coefficient projection');
          });
          actual.forEach((value,i)=>{
            const refinement=Math.abs(coarse.direct[i][c]-fine.direct[i][c]);
            assert(refinement<=0.0025*mass,'H3 direct unresolved');
            const [x,y,z]=normals[i],basis=[1,x,y,z,x*x-z*z,y*y-z*z,x*y,x*z,y*z];
            const projection=basis.reduce((sum,b,j)=>sum+Math.abs(b)*Math.abs(coefficients[j][c]-fine.coefficients[j][c]),0);
            assert(Math.abs(value[c]-fine.direct[i][c])<=3*mass/32+projection+2*refinement+1e-12,`H3 direct ${degrees} ${azimuth} normal ${i} channel ${c}`);
          });
        } count+=normals.length;
      }
    }
    return {queries:count,exposure:2,reference:'f64 hemisphere midpoint 64/128, P6 projection and truncation bounds'};
  });
  const legendre = order => Array.from({length:order},(_,i)=>{
    let x=Math.cos(Math.PI*(i+0.75)/(order+0.5)),derivative;
    for (;;) {
      let previous=1,current=x;
      for (let degree=2;degree<=order;degree++) { const next=((2*degree-1)*x*current-(degree-1)*previous)/degree; previous=current; current=next; }
      derivative=order*(x*current-previous)/(x*x-1);
      const correction=current/derivative; x-=correction;
      if (Math.abs(correction)<1e-15) break;
    }
    return [(x+1)/2,1/((1-x*x)*derivative*derivative)];
  });
  const quadrature=legendre(256);
  const dfgReference=(noV,roughness)=>{
    const a=roughness**4,vr=Math.sqrt(1-noV*noV),result=[0,0];
    for (const [noL,w] of quadrature) for (const [azimuth,aw] of quadrature) {
      const lDotV=noL*noV+Math.sqrt(1-noL*noL)*vr*Math.cos(Math.PI*azimuth);
      const length=Math.sqrt(2+2*lDotV),noH=(noL+noV)/length,voH=length/2;
      const distribution=a/(Math.PI*(1-noH*noH+a*noH*noH)**2);
      const smith=noL*Math.sqrt(a+(1-a)*noV*noV)+noV*Math.sqrt(a+(1-a)*noL*noL);
      const measure=distribution*noL/(2*smith)*w*2*Math.PI*aw,power=(1-Math.max(0,Math.min(1,voH)))**5;
      result[0]+=measure*(1-power); result[1]+=measure*power;
    }
    return result;
  };
  await run('H5',async()=>{
    const groups=[];
    for (let axis=0;axis<3;axis++) for (const sign of [-1,1]) {const n=[0,0,0];n[axis]=sign;groups.push([n]);}
    for (let absent=0;absent<3;absent++) for (const a of [-1,1]) for (const b of [-1,1]) {
      const n=[0,0,0];n[(absent+1)%3]=a;n[(absent+2)%3]=b;
      groups.push([(absent+1)%3,(absent+2)%3].map(axis=>{const p=n.slice();p[axis]*=1.0001;return p;}));
    }
    for (const x of [-1,1]) for (const y of [-1,1]) for (const z of [-1,1]) groups.push([0,1,2].map(axis=>{const n=[x,y,z];n[axis]*=1.0001;return n;}));
    const pipeline=state.pipelines.findLast(item=>item.descriptor.label==='atmosphere sky specular prefilter');
    const mipGroups=state.groups.filter(item=>item.descriptor.label==='atmosphere specular mip').slice(-7);
    assert(pipeline && mipGroups.length===7,'H5 production prefilter missing');
    let count=0;
    for (const azimuth of [0,Math.PI/2]) {
      setFrame(0,2,azimuth);
      writeField(texture(4),(x,y)=>{
        const n=direction([(x+0.5)/192,(y+0.5)/108],2,azimuth);
        return [1024*(1.5+0.5*n[0]),1024*(1.5+0.5*n[1]),1024*(1.5+0.5*n[2]),4096];
      });
      const encoder=device.createCommandEncoder();
      for (let level=0;level<7;level++) {const width=64>>level;dispatch(encoder,pipeline.value,mipGroups[level].value,[Math.ceil(width/16),width,6]);}
      device.queue.submit([encoder.finish()]);
      for (const lux of [1,0,2]) {
        setFrame(0,2,azimuth,lux);
        for (const group of [lent.value,exposed]) for (let level=0;level<7;level++) {
          const queries=groups.flatMap(directions=>directions.map(d=>[...d,0.045+0.955*level/6]));
          const actual=await query('vec4(environment_reflection(q.xyz,q.w,0.5).radiance,1.0)',queries,library,group);
          let index=0;
          groups.forEach((directions,groupIndex)=>{
            const first=actual[index];
            directions.forEach(n=>{
              for(let c=0;c<3;c++) {
                const value=actual[index][c];
                if(level===0 || (level===6 && groupIndex<6)) {
                  const expected=lux*(1.5+(level===0 ? 0.5 : 1/3)*n[c]/Math.hypot(...n))+0.0625;
                  assert(Math.abs(value-expected)<=lux*(1/1024+0.019)+0.0625/1024,`H5 azimuth ${azimuth} lux ${lux} level ${level} group ${groupIndex} channel ${c}: ${value} vs ${expected}`);
                }
                assert(Math.abs(value-first[c])<lux*0.01+1e-6,'H5 seam');
                if(lux===0) assert(Math.abs(value-0.0625)<=0.0625/1024,'H5 absolute night');
              } index++;
            });
          }); count+=queries.length;
        }
      }
    }
    const nodes=[[0,32],[8,48],[32,32],[32,64],[64,32],[64,64],[0,64]];
    for (const group of [lent.value,exposed]) {
      const actual=await query('vec4(environment_reflection(vec3(0.0,1.0,0.0),q.y,q.x).split_sum,0.0,1.0)',nodes.map(([x,y])=>[x/64,0.045+0.955*y/64,0,0]),library,group);
      nodes.forEach(([x,y],i)=>{
        const expected=dfgReference(x/64,0.045+0.955*y/64);
        for(let c=0;c<2;c++) assert(Math.abs(actual[i][c]-expected[c])<0.006,`H5 DFG ${x} ${y}`);
        assert(actual[i][0]+actual[i][1]<=1.001,'H5 DFG energy');
      });count+=nodes.length;
    }
    setFrame(0);
    writeField(texture(4),()=>[1024,512,256,0]);
    const encoder=device.createCommandEncoder();
    for(let level=0;level<7;level++){const width=64>>level;dispatch(encoder,pipeline.value,mipGroups[level].value,[Math.ceil(width/16),width,6]);}
    device.queue.submit([encoder.finish()]);
    const furnaceQueries=[];
    for(const base of [0,0.18,0.65,1])for(const metal of [0,1])for(const rough of [0.045,0.5,1])for(const cosine of [-0.5,0,0.01,0.5,1])furnaceQueries.push([base,metal,rough,cosine]);
    let baseline;
    for(const lux of [1,0,2]) {
      setFrame(0,2,0,lux);
      const source=library+`
fn furnace(q:vec4<f32>)->vec4<f32>{
        if q.w<=0.0{return vec4(0.0,0.0,0.0,1.0);}
        let environment=environment_reflection(vec3(0.0,1.0,0.0),q.z,q.w);
        let ess=mix(vec3(0.04),vec3(q.x),q.y)*environment.split_sum.x+vec3(environment.split_sum.y);
        let diffuse=(1.0-q.y)*q.x*(vec3(1.0)-ess)*vec3(1.0,0.5,0.25)*${lux}.0;
        return vec4(diffuse+environment.radiance*ess,1.0);
      }`;
      for(const group of [lent.value,exposed]) {
        const values=await query('furnace(q)',furnaceQueries,source,group);
        values.forEach((value,i)=>value.slice(0,3).forEach((v,c)=>{
          const incident=[1,0.5,0.25][c];
          assert(v>=0&&v<=lux*incident*1.002+1e-6,`H5 furnace ${lux} ${i} ${c}`);
          if(furnaceQueries[i][3]<=0||lux===0)assert(v===0,'H5 zero source or backface');
          if(lux===2)assert(Math.abs(v-2*baseline[i][c])<2e-6,'H5 doubling');
        }));
        if(lux===1)baseline=values;
        count+=furnaceQueries.length;
      }
    }
    return {queries:count,exposure:2,raw_sun:[0,1,2],night:0.0625,reference:'P7 direction-coded sky and P8 independent Gauss-Legendre 256'};
  });
  await run('night', async () => {
    let count=0;
    device.queue.writeBuffer(nightControl,0,new Uint32Array([1]));
    const references=[
      [0,[1.710e-4,2.747452e-4,2.989057e-4,2.427922e-4,2.135841e-5],[2.135841e-5,1.034311e-4],6.709942e-4],
      [1800,[1.727028e-4,2.877571e-4,3.294886e-4,2.862687e-4,2.475446e-5],[9.189556e-5,9.973511e-5],6.912419e-4],
      [15000,[1.770471e-4,3.294208e-4,4.422802e-4,4.742667e-4,3.760431e-5],[1.184168e-4,1.197290e-4],7.570950e-4],
    ];
    for (const [height,rays,horizon,lux] of references) {
      const f=setFrame(-90,height,0,0);
      f.fill(0,0,16);f.set([1000,height,0,1],12);device.queue.writeBuffer(frame,0,f);
      await produce();
      const queries=[90,20,10,5,-30].map(e=>[Math.cos(e*Math.PI/180),Math.sin(e*Math.PI/180),0,0]);
      const tangent=-Math.sqrt(height*(2*6371000+height))/(6371000+height);
      for(const delta of [-1e-6,1e-6]){const y=tangent+delta;queries.push([Math.sqrt(1-y*y),y,0,0]);}
      for(const group of [lent.value,exposed]) {
        const sky=await query('vec4(environment_sky_radiance(q.xyz),1.0)',queries,library,group);
        [...rays,...horizon].forEach((expected,i)=>sky[i].slice(0,3).forEach(v=>assert(Math.abs(v-expected)<=0.10*expected,`H2 night height ${height} query ${i}: ${v} vs ${expected}`)));
        const diffuse=await query('vec4(environment_diffuse(q.xyz),1.0)',[[0,1,0,0]],library,group);
        diffuse[0].slice(0,3).forEach(v=>assert(Math.abs(v-lux)<=0.05*lux,`H3 night ${height}: ${v} vs ${lux}`));
        if(height===1800) {
          const aerial=await query('vec4(environment_aerial(vec3(q.x,1800.0,0.0),vec2(0.5)).scatter,1.0)',[[1000,0,0,0],[10000,0,0,0],[30000,0,0,0]],library,group);
          [1.2416652e-6,1.1780591e-5,3.150029e-5].forEach((expected,i)=>aerial[i].slice(0,3).forEach(v=>assert(Math.abs(v-expected)<=0.10*expected,`H4 night ${i}: ${v} vs ${expected}`)));
        }
        count+=queries.length+1;
      }
    }
    return {queries:count,exposure:2,reference:'Preserved P3/P4/P5 f64 finite shell and diffuse profile'};
  });
  if(rows.night.status!=='pass') for(const name of ['H2','H3','H4']) rows[name].status='fail';
  device.queue.writeBuffer(frame, 0, originalFrame);
  exposure.destroy();
  return report;
}
