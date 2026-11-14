#include <metal_stdlib>
using namespace metal;
kernel void toggle(texture2d<half, access::read> previous [[texture(0)]],
                   texture2d<half, access::write> output [[texture(1)]],
                   texture2d<half, access::write> next [[texture(2)]]) {
    uint bit = uint(previous.read(uint2(0)).x);
    half4 value = bit == 0u ? half4(1.0h) : half4(0.0h);
    output.write(value, uint2(0));
    next.write(value, uint2(0));
}
