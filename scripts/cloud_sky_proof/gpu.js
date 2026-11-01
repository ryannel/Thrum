// Retain the checked shader compiler and hashing helper from the bounded proof.
export {module as compileShader, hash} from '../cloud_coupled_proof/source.js';

export async function readBuffer(device, buffer, size) {
  const read = device.createBuffer({size, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ});
  const encoder = device.createCommandEncoder();
  encoder.copyBufferToBuffer(buffer, 0, read, 0, size);
  device.queue.submit([encoder.finish()]);
  await read.mapAsync(GPUMapMode.READ);
  const bytes = new Uint8Array(read.getMappedRange()).slice();
  read.unmap();
  read.destroy();
  return bytes;
}

export async function readTexture(device, texture, dimensions, bytesPerTexel = 8) {
  const [width, height, depth] = dimensions;
  const rowBytes = Math.ceil(width * bytesPerTexel / 256) * 256;
  const size = rowBytes * height * depth;
  const read = device.createBuffer({size, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ});
  const encoder = device.createCommandEncoder();
  encoder.copyTextureToBuffer({texture}, {buffer: read, bytesPerRow: rowBytes, rowsPerImage: height}, dimensions);
  device.queue.submit([encoder.finish()]);
  await read.mapAsync(GPUMapMode.READ);
  const mapped = new Uint8Array(read.getMappedRange());
  const result = new Uint8Array(width * height * depth * bytesPerTexel);
  for (let slice = 0; slice < depth; slice++) {
    for (let row = 0; row < height; row++) {
      const index = slice * height + row;
      result.set(mapped.subarray(index * rowBytes, index * rowBytes + width * bytesPerTexel), index * width * bytesPerTexel);
    }
  }
  read.unmap();
  read.destroy();
  return result;
}
