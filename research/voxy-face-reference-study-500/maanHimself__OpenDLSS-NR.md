# Facial rendering source scan: maanHimself/OpenDLSS-NR

Repository: https://github.com/maanHimself/OpenDLSS-NR

Pinned source revision: 9d08f4184bbcb9d858e2fb7a7834ec0837a9d2f1

Discovery topic: DLSS

Description: A Vulkan reimplementation of NVIDIA's DLSS 5 Neural Rendering network, bit-exact against the original.

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE", "NOTICE"]

Topic paths: {"shader": ["demo/shaders/nr_common.glsl", "demo/shaders/nr_composite.comp", "demo/shaders/nr_preprocess.comp", "demo/shaders/velocity_unpack.comp", "ports/browser-webgpu/demo/src/webgi-addon/passes/CopyShader.ts", "ports/browser-webgpu/demo/src/webgi-addon/shaders/poissonDiskSamples.glsl", "ports/browser-webgpu/demo/src/webgi-addon/shaders/randomHelpers.glsl", "ports/browser-webgpu/demo/src/webgi-addon/shaders/reflectorSample.glsl", "ports/browser-webgpu/demo/src/webgi-addon/shaders/ssVelocityFrag.glsl", "ports/browser-webgpu/demo/src/webgi-addon/shaders/ssVelocityVert.glsl", "ports/browser-webgpu/shaders/frame.wgsl", "ports/browser-webgpu/shaders/gemm_f16.wgsl", "ports/browser-webgpu/shaders/numerics.wgsl", "ports/browser-webgpu/shaders/ops.wgsl", "ports/browser-webgpu/shaders/preprocess.wgsl", "ports/browser-webgpu/shaders/selftest.wgsl", "ports/browser-webgpu/shaders/vit.wgsl", "scripts/build_shaders.ps1", "shaders/attention_common.glsl", "shaders/common.glsl", "shaders/fused_block32.comp", "shaders/gemm_f16.comp", "shaders/gemm_fp8.comp", "shaders/gemm_mlp.comp", "shaders/gemm_reduce.comp", "shaders/global_attend.comp", "shaders/global_attention.comp", "shaders/global_normalize.comp", "shaders/ops.comp", "shaders/preprocess.comp"], "dlss": ["ports/browser-webgpu/demo/src/webgi-addon/DlssBridgePlugin.ts", "ports/browser-webgpu/demo/src/webgi-addon/DlssGroundPlugin.ts", "ports/browser-webgpu/demo/src/webgi-addon/DlssReflector2.ts", "ports/browser-webgpu/demo/src/webgi-addon/DlssVelocityBufferPlugin.ts", "ports/browser-webgpu/demo/src/webgi-addon/passes/DlssVelocityPass.ts"]}

## Source file: shaders/common.glsl

{
  "path": "shaders/common.glsl",
  "blob_sha": "eca93373570e7beb922594f10eba5af97b60664d",
  "sha256": "e8b0928a841d31983b0e0a57fa183c77578a8e45a9e83ed4391aad18f515b842",
  "lines": 134,
  "bytes": 6030,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 3,
    "dlss": 3
  },
  "url": "https://github.com/maanHimself/OpenDLSS-NR/blob/9d08f4184bbcb9d858e2fb7a7834ec0837a9d2f1/shaders/common.glsl",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "float roundF16(float value) {"
    },
    {
      "line": 88,
      "text": "float e4m3HwToF32(uint code) {"
    },
    {
      "line": 112,
      "text": "float e4m3ToF32(uint bits) {"
    },
    {
      "line": 126,
      "text": "float mpCubicSilu(float value) {"
    }
  ]
}

```text

// Shared numeric helpers: the half / E4M3 publication points of the network, implemented so that every kernel
// (and the CPU reference in src/reference.cpp) rounds identically.
#ifndef DLSS_COMMON_GLSL
#define DLSS_COMMON_GLSL

#extension GL_EXT_shader_explicit_arithmetic_types : require
#extension GL_EXT_shader_16bit_storage : require
#extension GL_EXT_shader_8bit_storage : require

// f32 -> RNE f16 -> f32, on the bit pattern.
// Don't write float(float16_t(x)): the NVIDIA compiler drops it as a no-op and the rounding
// goes with it. That was the first parity bug, inside the SiLU.
float roundF16(float value) {
  uint bits = floatBitsToUint(value);
  uint sign = bits & 0x80000000u;
  uint magnitude = bits & 0x7fffffffu;
  if (magnitude >= 0x7f800000u) return value;                                   // inf / nan
  if (magnitude >= 0x477ff000u) return uintBitsToFloat(sign | 0x7f800000u);     // >= 65520 overflows to inf
  if (magnitude < 0x38800000u) {                                                // below 2^-14: half subnormal grid 2^-24
    float scaled = roundEven(uintBitsToFloat(magnitude) * 16777216.0);
    return uintBitsToFloat(sign) + uintBitsToFloat(sign | floatBitsToUint(scaled * 0.000000059604644775390625));
  }
  uint lsb = (magnitude >> 13u) & 1u;
  uint rounded = (magnitude + 0xfffu + lsb) & ~0x1fffu;
  return uintBitsToFloat(sign | rounded);
}

uint f16Bits(float16_t value) { return uint(float16BitsToUint16(value)); }
float16_t f16FromBits(uint bits) { return uint16BitsToFloat16(uint16_t(bits & 0xffffu)); }

uint shiftRne(uint value, uint shift) {
  uint quotient = value >> shift;
  uint remainderMask = (1u << shift) - 1u;
  uint remainder = value & remainderMask;
  uint halfway = 1u << (shift - 1u);
  return quotient + ((remainder > halfway || (remainder == halfway && (quotient & 1u) != 0u)) ? 1u : 0u);
}

// An already half-rounded value to its E4M3FN code: RNE, finite saturation at 448, NaN -> +0.
// The sign of a zero survives (code 0x80), as the hardware conversion does. It makes no numeric
// difference; parity compares bytes.
uint e4m3CodeFromF16Bits(uint halfBits) {
  bool isNan = (halfBits & 0x7c00u) == 0x7c00u && (halfBits & 0x03ffu) != 0u;
  if (isNan) return 0u;
  if ((halfBits & 0x7fffu) == 0u) return (halfBits >> 8u) & 0x80u;
  uint negative = (halfBits & 0x8000u) != 0u ? 0x80u : 0u;
  uint exponent = (halfBits >> 10u) & 0x1fu;
  uint mantissa = halfBits & 0x03ffu;
  uint code;
  if (exponent == 31u) {
    code = 0x7eu;                     // inf saturates
  } else if (exponent <= 8u) {        // E4M3 subnormal
    uint significand = exponent == 0u ? mantissa : 1024u + mantissa;
    uint shift = exponent == 0u ? 15u : 16u - exponent;
    code = min(shiftRne(significand, shift), 8u);
  } else {
    uint e4Exponent = exponent - 8u;
    uint e4Mantissa = shiftRne(mantissa, 7u);
    if (e4Mantissa == 8u) { e4Mantissa = 0u; e4Exponent += 1u; }
    code = (e4Exponent > 15u || (e4Exponent == 15u && e4Mantissa > 6u)) ? 0x7eu : ((e4Exponent << 3u) | e4Mantissa);
  }
  return negative | code;
}

uint e4m3CodeFromF32(float value) { return e4m3CodeFromF16Bits(f16Bits(float16_t(value))); }

#ifdef DLSS_E4M3_HW
// Hardware cvt.rn.satfinite.e4m3, equal to e4m3CodeFromF16Bits for every non-NaN half.
// The guard is the whole difference: the instruction turns a NaN into the E4M3 NaN code, and a NaN gets
// here every time a zero row is cosine normalized (0 * inf). +0 keeps that code out of every tensor.
uint8_t e4m3Hw(float16_t value) {
  if (isnan(value)) return uint8_t(0u);
  floate4m3_t converted;
  saturatedConvertEXT(converted, value);
  return floate4m3BitsToUintEXT(converted);
}

// Two halves -> two E4M3 codes packed little-endian, same rules as e4m3Hw.
uint16_t e4m3HwPair(f16vec2 value) {
  f16vec2 clean = mix(value, f16vec2(0.0), isnan(value));
  fe4m3vec2 converted;
  saturatedConvertEXT(converted, clean);
  u8vec2 codes = floate4m3BitsToUintEXT(converted);
  return uint16_t(uint(codes.x) | (uint(codes.y) << 8u));
}

// Hardware decode. NaN codes read back as 0, as in the reference.
float e4m3HwToF32(uint code) {
  float value = float(uintBitsToFloate4m3EXT(uint8_t(code & 0xffu)));
  return isnan(value) ? 0.0 : value;
}
#endif

// The network's cubic SiLU in half FMAs: identical to the f32 formulation (mpCubicSilu below) for all
// 65536 half inputs.
float16_t siluHalf(float16_t value) {
  float16_t bounded = clamp(value, float16_t(-4.0), float16_t(4.0));
  precise float16_t inner = fma(float16_t(-0.055908203125), abs(bounded), float16_t(0.447265625));
  precise float16_t polynomial = fma(bounded, inner, float16_t(0.89453125));
  precise float16_t result = value * polynomial;
  return result;
}

f16vec2 siluPair(f16vec2 value) {
  f16vec2 bounded = clamp(value, f16vec2(-4.0), f16vec2(4.0));
  precise f16vec2 inner = fma(f16vec2(-0.055908203125), abs(bounded), f16vec2(0.447265625));
  precise f16vec2 polynomial = fma(bounded, inner, f16vec2(0.89453125));
  precise f16vec2 result = value * polynomial;
  return result;
}

float e4m3ToF32(uint bits) {
  bool negative = (bits & 0x80u) != 0u;
  uint exponent = (bits >> 3u) & 0x0fu;
  uint mantissa = bits & 0x07u;
  float value;
  if (exponent == 0u) value = float(mantissa) * 0.001953125;
  else if (exponent == 15u && mantissa == 7u) value = 0.0;
  else value = (1.0 + float(mantissa) * 0.125) * uintBitsToFloat((exponent + 120u) << 23u);
  return negative ? -value : value;
}

// The f32 spelling of siluHalf: each step is one f32 operation rounded once to half.
// The explicit fma is not decoration. Without it the compiler splits the multiply-add and
// double-rounds, which was the second thing to break parity.
float mpCubicSilu(float value) {
  precise float bounded = roundF16(clamp(value, -4.0, 4.0));
  precise float absolute = roundF16(abs(bounded));
  precise float inner = roundF16(fma(-0.055908203125, absolute, 0.447265625));
  precise float polynomial = roundF16(fma(bounded, inner, 0.89453125));
  return roundF16(value * polynomial);
}

#endif


```

## Source file: shaders/publish.glsl

{
  "path": "shaders/publish.glsl",
  "blob_sha": "ce2df5fbd5d944303ae81c0c4e1ce48cfee804fb",
  "sha256": "5c46cdcd9e7a962cfbc64e4bcb45e661adc265a6f38c84e0aca86ebc310fed97",
  "lines": 49,
  "bytes": 3612,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 1,
    "dlss": 2
  },
  "url": "https://github.com/maanHimself/OpenDLSS-NR/blob/9d08f4184bbcb9d858e2fb7a7834ec0837a9d2f1/shaders/publish.glsl",
  "function_signature_candidates": []
}

```text

// Accumulator publication helpers (fragment level). The coopmat2 tensor stores and the plain
// coopMatStore of 8-bit accumulators to global memory compile to scattered narrow stores that
// cost ~8000 SM cycles per 16x32 tile (measured in gemm_mlp); these macros assemble the row tile in
// shared memory as 32-bit words and store whole 16-byte words.
//
// Accumulator layout (verified on Ada): lane (g = l>>2, t = l&3) holds rows g and
// g+8; element j of a row half is column 8*(j>>1) + 2t + (j&1). Requires GL_KHR_shader_subgroup_shuffle.
#ifndef DLSS_PUBLISH_GLSL
#define DLSS_PUBLISH_GLSL

// E4 codes of a NaN-free pair: the saturating hardware conversion (= common.glsl e4m3HwPair without the NaN
// guard, for accumulators whose NaNs the caller has already removed).
uint16_t e4m3PairFast(f16vec2 value) {
  fe4m3vec2 converted;
  saturatedConvertEXT(converted, value);
  u8vec2 codes = floate4m3BitsToUintEXT(converted);
  return uint16_t(uint(codes.x) | (uint(codes.y) << 8u));
}

// 16 x N f16 accumulator `acc` -> E4 row tile of N/4 32-bit words per row in `words32` at `base`
// (NaN -> 0). Lanes t, t^1 exchange one pair per column group so the even lane assembles the word of
// row g and the odd lane that of row g+8. Follow with subgroupBarrier() before reading the tile.
#define PUBLISH_E4_TILE(acc, N, words32, base, g, t)                                                   \
  {                                                                                                    \
    [[unroll]] for (uint j = 0u; j < (N) / 8u; ++j) {                                                  \
      f16vec2 v0 = f16vec2(acc[2u * j], acc[2u * j + 1u]);                                             \
      f16vec2 v1 = f16vec2(acc[(N) / 4u + 2u * j], acc[(N) / 4u + 2u * j + 1u]);                       \
      uint c0 = uint(e4m3PairFast(mix(v0, f16vec2(0.0), isnan(v0))));                                  \
      uint c1 = uint(e4m3PairFast(mix(v1, f16vec2(0.0), isnan(v1))));                                  \
      bool odd = ((t) & 1u) != 0u;                                                                     \
      uint recv = subgroupShuffleXor(odd ? c0 : c1, 1u);                                               \
      uint word = odd ? (recv | (c1 << 16u)) : (c0 | (recv << 16u));                                   \
      uint row = odd ? (g) + 8u : (g);                                                                 \
      words32[(base) + row * ((N) / 4u) + 2u * j + ((t) >> 1u)] = word;                                \
    }                                                                                                  \
  }

// 16 x N f16 accumulator -> f16 row tile of N/2 32-bit words per row (pairs are already words).
#define PUBLISH_F16_TILE(acc, N, words32, base, g, t)                                                  \
  {                                                                                                    \
    [[unroll]] for (uint h = 0u; h < 2u; ++h) {                                                        \
      [[unroll]] for (uint j = 0u; j < (N) / 8u; ++j) {                                                \
        words32[(base) + ((g) + 8u * h) * ((N) / 2u) + 4u * j + (t)] =                                 \
            packFloat2x16(f16vec2(acc[(N) / 4u * h + 2u * j], acc[(N) / 4u * h + 2u * j + 1u]));       \
      }                                                                                                \
    }                                                                                                  \
  }

#endif


```

## Source file: shaders/attention_common.glsl

{
  "path": "shaders/attention_common.glsl",
  "blob_sha": "0cc935a98ed0cdcc31ab3a0ed4b98b91202836b8",
  "sha256": "41fb2921967eda0ea12172aaae93020ab5d178f0711660b0dea59a14828ea24d",
  "lines": 206,
  "bytes": 10166,
  "selection_score": 15,
  "keyword_line_counts": {
    "dlss": 1
  },
  "url": "https://github.com/maanHimself/OpenDLSS-NR/blob/9d08f4184bbcb9d858e2fb7a7834ec0837a9d2f1/shaders/attention_common.glsl",
  "function_signature_candidates": [
    {
      "line": 25,
      "text": "void subgroupSync() { subgroupMemoryBarrierShared(); subgroupBarrier(); }"
    },
    {
      "line": 98,
      "text": "void scaleQuantize32(uvec4 w0, uvec4 w1, uvec4 w2, uvec4 w3, float16_t norm, float16_t extra, bool applyExtra,"
    },
    {
      "line": 121,
      "text": "void windowAttend(uint base, uint rowBase, uint KP, uint VP, uint priorOffset) {"
    }
  ]
}

```text

// Shared machinery of the fused window-attention kernels (fused_block32.comp,
// window_attention_fused.comp). The includer declares, before including:
//   layout(...) readonly buffer Prior { float16_t prior[]; };   // [heads][64][64] f16
//   shared uvec4 kv4[...];        // per window: Kp [64][32] E4 at KP, Vp [64][32] E4 at VP (physical-tiled rows)
//   shared uvec4 priv4[...];      // PRIV words per subgroup, phase-aliased:
//     E4 rows  @0..31   [16][32] E4, stride 2: Q (S GEMM input) -> attended (output)
//     F64      @32..175 [16][64] f16, stride 9: raw q|k staging, S rows (P E4 aliases words 0..3 of each row,
//                        the PV epilogue stages its tiles in words 4..7)
//     F32      @32..111 [16][32] f16, stride 5
//     HID      @32..111 [16][64] E4, stride 5
// and requires GL_EXT_float_e4m3, the cooperative matrix, subgroup shuffle and
// control-flow-attribute extensions plus common.glsl with DLSS_E4M3_HW.

const uint PRIV = 176u, F64 = 32u, F32 = 32u, HID = 32u;

#define ACC coopmat<float16_t, gl_ScopeSubgroup, 16, 16, gl_MatrixUseAccumulator>
#define MAT_A coopmat<floate4m3_t, gl_ScopeSubgroup, 16, 32, gl_MatrixUseA>
#define MAT_B coopmat<floate4m3_t, gl_ScopeSubgroup, 32, 16, gl_MatrixUseB>

uint tiledToken(uint token) {  // natural -> physical
  uint x = token & 7u, y = token >> 3u;
  return (y >> 2u) * 32u + (x >> 2u) * 16u + (y & 3u) * 4u + (x & 3u);
}

void subgroupSync() { subgroupMemoryBarrierShared(); subgroupBarrier(); }

// the exponential approximation on a packed pair: f32 affine, half clamp, then per half
// (bits << 5) + 0x8000 masked to 16 bits, which is (bits << 5) ^ 0x8000.
uint expWeightPair(uint scoreBits) {
  precise vec2 affine32 = fma(vec2(unpackFloat2x16(scoreBits)), vec2(0.044921875), vec2(1.30078125));
  f16vec2 affine = clamp(f16vec2(affine32), f16vec2(1.03125), f16vec2(1.5693359375));
  return ((packFloat2x16(affine) << 5u) & 0xffe0ffe0u) ^ 0x80008000u;
}

// Four E4M3 codes from two half pairs, little-endian.
uint e4x4(f16vec2 a, f16vec2 b) { return uint(e4m3HwPair(a)) | (uint(e4m3HwPair(b)) << 16u); }
uint e4x4Bits(uint a, uint b) { return e4x4(unpackFloat2x16(a), unpackFloat2x16(b)); }

// 16 halves (two words) -> 16 E4M3 codes (one word).
uvec4 quantize16(uvec4 w0, uvec4 w1) {
  return uvec4(e4x4Bits(w0.x, w0.y), e4x4Bits(w0.z, w0.w), e4x4Bits(w1.x, w1.y), e4x4Bits(w1.z, w1.w));
}

// 16 halves -> SiLU -> 16 E4M3 codes.
uvec4 hidden16(uvec4 w0, uvec4 w1) {
  return uvec4(e4x4(siluPair(unpackFloat2x16(w0.x)), siluPair(unpackFloat2x16(w0.y))),
               e4x4(siluPair(unpackFloat2x16(w0.z)), siluPair(unpackFloat2x16(w0.w))),
               e4x4(siluPair(unpackFloat2x16(w1.x)), siluPair(unpackFloat2x16(w1.y))),
               e4x4(siluPair(unpackFloat2x16(w1.z)), siluPair(unpackFloat2x16(w1.w))));
}

// round_f16(residual * scale) for eight residual/scale pairs (exact products in f32, one rounding).
uvec4 scaledResidual8(uvec4 residual, uvec4 scale) {
  uvec4 r;
  [[unroll]] for (uint i = 0u; i < 4u; ++i) {
    vec2 product = vec2(unpackFloat2x16(residual[i])) * vec2(unpackFloat2x16(scale[i]));
    r[i] = packFloat2x16(f16vec2(product));
  }
  return r;
}

// Same for eight E4M3 residual codes packed in two words.
uvec4 scaledResidualE4x8(uvec2 codes, uvec4 scale) {
  uvec4 r;
  [[unroll]] for (uint i = 0u; i < 4u; ++i) {
    uint pair = (codes[i >> 1u] >> ((i & 1u) * 16u)) & 0xffffu;
    vec2 product = vec2(e4m3HwToF32(pair & 0xffu), e4m3HwToF32(pair >> 8u)) * vec2(unpackFloat2x16(scale[i]));
    r[i] = packFloat2x16(f16vec2(product));
  }
  return r;
}

// Cosine normalization of one 32-channel vector held as 16 packed pairs:
// r[c] = fma(v[c], v[c], f16(v[c+16]^2)) for c < 16, tree stride 8/4/2/1, each
// level published as half exactly like the reference reduction array.
float16_t inverseNorm(uvec4 w0, uvec4 w1, uvec4 w2, uvec4 w3) {
  f16vec2 p[16];
  [[unroll]] for (uint i = 0u; i < 4u; ++i) {
    p[i] = unpackFloat2x16(w0[i]); p[4u + i] = unpackFloat2x16(w1[i]);
    p[8u + i] = unpackFloat2x16(w2[i]); p[12u + i] = unpackFloat2x16(w3[i]);
  }
  f16vec2 r[8];
  [[unroll]] for (uint m = 0u; m < 8u; ++m) {
    precise f16vec2 highSquare = p[m + 8u] * p[m + 8u];
    precise f16vec2 rm = fma(p[m], p[m], highSquare);
    r[m] = rm;
  }
  f16vec2 s8[4];
  [[unroll]] for (uint m = 0u; m < 4u; ++m) { precise f16vec2 s = r[m] + r[m + 4u]; s8[m] = s; }
  precise f16vec2 s4a = s8[0] + s8[2];
  precise f16vec2 s4b = s8[1] + s8[3];
  precise f16vec2 s2 = s4a + s4b;
  precise float16_t s1 = s2.x + s2.y;
  return float16_t(inversesqrt(float(s1)));
}

// 32 halves (four words) times a half (and optionally a second half), E4M3 codes of the products (two words).
void scaleQuantize32(uvec4 w0, uvec4 w1, uvec4 w2, uvec4 w3, float16_t norm, float16_t extra, bool applyExtra,
                     out uvec4 e0, out uvec4 e1) {
  uvec4 words[4] = uvec4[4](w0, w1, w2, w3);
  uint codes[8];
  [[unroll]] for (uint i = 0u; i < 8u; ++i) {
    f16vec2 a = unpackFloat2x16(words[i >> 1u][(i & 1u) * 2u]), b = unpackFloat2x16(words[i >> 1u][(i & 1u) * 2u + 1u]);
    precise f16vec2 na = a * norm;
    precise f16vec2 nb = b * norm;
    if (applyExtra) {
      precise f16vec2 sa = na * extra;
      precise f16vec2 sb = nb * extra;
      na = sa; nb = sb;
    }
    codes[i] = e4x4(na, nb);
  }
  e0 = uvec4(codes[0], codes[1], codes[2], codes[3]);
  e1 = uvec4(codes[4], codes[5], codes[6], codes[7]);
}

// Attention for one subgroup's 16 query rows: Q (E4 rows at `base`) against the
// window's Kp/Vp, prior tile row block at `priorOffset` (halves). S = QK^T + prior
// -> the specified softmax (exponential on read, half reciprocal, E4 weights) -> O = PV
// -> attended E4 [16][32] written back into the E4 rows.
void windowAttend(uint base, uint rowBase, uint KP, uint VP, uint priorOffset) {
  const uint lane = gl_SubgroupInvocationID;
  const uint lr = lane >> 1u, lh = lane & 1u;
  const uint sBase = base + F64;   // S rows [16][64] f16, 9 words per row
  MAT_A mq; coopMatLoad(mq, priv4, base, 2u, gl_CooperativeMatrixLayoutRowMajor);
  ACC priors[4];
  [[unroll]] for (uint kTile = 0u; kTile < 4u; ++kTile)
    coopMatLoad(priors[kTile], prior, priorOffset + kTile * 16u, 64u, gl_CooperativeMatrixLayoutRowMajor);
  [[unroll]] for (uint kTile = 0u; kTile < 4u; ++kTile) {
    MAT_B mb; coopMatLoad(mb, kv4, KP + kTile * 32u, 2u, gl_CooperativeMatrixLayoutColumnMajor);
    ACC mc = coopMatMulAdd(mq, mb, priors[kTile]);
    coopMatStore(mc, priv4, sBase + kTile * 2u, 9u, gl_CooperativeMatrixLayoutRowMajor);
  }
  subgroupSync();
  // Softmax: one lane pair per S row (lane = row*2 + hh, hh selects keys 0..31 or 32..63).
  // the specified softmax reduction: b[g][j] = e[g+16j] + e[g+16j+8]; t[g] = ((b0+b1)+b2)+b3;
  // total = (((t0+t2)+t4)+t6) + (((t1+t3)+t5)+t7). The exponentials stay in registers for the weights.
  {
    uint row = lr, hh = lh;
    uint src = sBase + row * 9u + hh * 4u;
    uvec4 s0 = priv4[src], s1 = priv4[src + 1u], s2 = priv4[src + 2u], s3 = priv4[src + 3u];
    uint e[16];   // packed exp weights of keys hh*32 + 2i, 2i+1
    [[unroll]] for (uint i = 0u; i < 4u; ++i) {
      e[i] = expWeightPair(s0[i]); e[4u + i] = expWeightPair(s1[i]);
      e[8u + i] = expWeightPair(s2[i]); e[12u + i] = expWeightPair(s3[i]);
    }
    // Own b values: this half holds j = 2*hh, 2*hh+1 for every g (local key = g + 16*(j & 1));
    // b[g][j] for g = 2m, 2m+1 live in one packed word.
    uint bOwn[8];   // [jLocal * 4 + m]
    [[unroll]] for (uint jl = 0u; jl < 2u; ++jl) {
      [[unroll]] for (uint m = 0u; m < 4u; ++m) {
        precise f16vec2 b = unpackFloat2x16(e[8u * jl + m]) + unpackFloat2x16(e[8u * jl + 4u + m]);
        bOwn[jl * 4u + m] = packFloat2x16(b);
      }
    }
    uint bOther[8];
    [[unroll]] for (uint i = 0u; i < 8u; ++i) bOther[i] = subgroupShuffleXor(bOwn[i], 1u);
    f16vec2 t[4];
    [[unroll]] for (uint m = 0u; m < 4u; ++m) {
      f16vec2 b0 = unpackFloat2x16(hh == 0u ? bOwn[m] : bOther[m]);
      f16vec2 b1 = unpackFloat2x16(hh == 0u ? bOwn[4u + m] : bOther[4u + m]);
      f16vec2 b2 = unpackFloat2x16(hh == 0u ? bOther[m] : bOwn[m]);
      f16vec2 b3 = unpackFloat2x16(hh == 0u ? bOther[4u + m] : bOwn[4u + m]);
      precise f16vec2 t01 = b0 + b1;
      precise f16vec2 t012 = t01 + b2;
      precise f16vec2 tm = t012 + b3;
      t[m] = tm;   // (t[2m], t[2m+1])
    }
    precise float16_t e01 = t[0].x + t[1].x;
    precise float16_t e012 = e01 + t[2].x;
    precise float16_t even = e012 + t[3].x;
    precise float16_t o01 = t[0].y + t[1].y;
    precise float16_t o012 = o01 + t[2].y;
    precise float16_t odd = o012 + t[3].y;
    precise float16_t total = even + odd;
    float16_t reciprocal = float16_t(1.0 / float(total));
    uint codes[8];
    [[unroll]] for (uint i = 0u; i < 8u; ++i) {
      precise f16vec2 wa = unpackFloat2x16(e[2u * i]) * reciprocal;
      precise f16vec2 wb = unpackFloat2x16(e[2u * i + 1u]) * reciprocal;
      codes[i] = e4x4(wa, wb);
    }
    subgroupSync();   // every lane has read its S words before P overwrites words 0..3 of the rows
    priv4[sBase + row * 9u + hh * 2u] = uvec4(codes[0], codes[1], codes[2], codes[3]);
    priv4[sBase + row * 9u + hh * 2u + 1u] = uvec4(codes[4], codes[5], codes[6], codes[7]);
  }
  subgroupSync();
  // O = P V over the 64 physical keys as two k32 steps; both 16x16 tiles staged in words 4..7 of the S rows.
  ACC mo[2];
  [[unroll]] for (uint cTile = 0u; cTile < 2u; ++cTile) {
    mo[cTile] = ACC(0.0);
    [[unroll]] for (uint kb = 0u; kb < 64u; kb += 32u) {
      MAT_A mp; coopMatLoad(mp, priv4, sBase + (kb >> 4u), 9u, gl_CooperativeMatrixLayoutRowMajor);
      MAT_B mv; coopMatLoad(mv, kv4, VP + kb * 2u + cTile, 2u, gl_CooperativeMatrixLayoutRowMajor);
      mo[cTile] = coopMatMulAdd(mp, mv, mo[cTile]);
    }
  }
  [[unroll]] for (uint cTile = 0u; cTile < 2u; ++cTile)
    coopMatStore(mo[cTile], priv4, sBase + 4u + cTile * 2u, 9u, gl_CooperativeMatrixLayoutRowMajor);
  subgroupSync();
  {
    uint src = sBase + lr * 9u + 4u + lh * 2u;
    priv4[base + lr * 2u + lh] = quantize16(priv4[src], priv4[src + 1u]);   // attended E4 [16][32]
  }
  subgroupSync();
}


```

## Source file: ports/browser-webgpu/shaders/ops.wgsl

{
  "path": "ports/browser-webgpu/shaders/ops.wgsl",
  "blob_sha": "c3f3fb976c7965e13ee9dbecae501727065868ec",
  "sha256": "b31e8ea58383177d185bb0b53551665799328ad20aa05a691320e577b26a99ff",
  "lines": 130,
  "bytes": 5855,
  "selection_score": 15,
  "keyword_line_counts": {},
  "url": "https://github.com/maanHimself/OpenDLSS-NR/blob/9d08f4184bbcb9d858e2fb7a7834ec0837a9d2f1/ports/browser-webgpu/shaders/ops.wgsl",
  "function_signature_candidates": [
    {
      "line": 34,
      "text": "fn half_at(buffer: ptr<storage, array<u32>, read>, index: u32) -> f32 {"
    },
    {
      "line": 37,
      "text": "fn e4_at(buffer: ptr<storage, array<u32>, read>, index: u32) -> f32 {"
    },
    {
      "line": 40,
      "text": "fn aux_half(index: u32) -> f32 {"
    },
    {
      "line": 46,
      "text": "fn linear_index(id: vec3<u32>) -> u32 { return id.x + id.y * 65535u * 64u; }"
    },
    {
      "line": 48,
      "text": "fn publish(index: u32, values: array<f32, 4>) {"
    },
    {
      "line": 60,
      "text": "fn convert_f32_to_f16(@builtin(global_invocation_id) id : vec3<u32>) {"
    },
    {
      "line": 70,
      "text": "fn downsample(@builtin(global_invocation_id) id : vec3<u32>) {"
    },
    {
      "line": 99,
      "text": "fn upsample_residual(@builtin(global_invocation_id) id : vec3<u32>) {"
    },
    {
      "line": 117,
      "text": "fn post_blend(@builtin(global_invocation_id) id : vec3<u32>) {"
    }
  ]
}

```text

// The elementwise steps between the matrix multiplies: the format conversions, the 2x2 pool that moves down a
// level, and the two merges that bring a skip connection back in on the way up.
//
// None of them is a reshape. Each one is arithmetic with its own rounding schedule, and each is part of the
// result - the pool's `((a+b)+(c+d)) * 0.25` in halves, and the merges' single f32 rounding of a product plus
// a residual before the half publication.

struct Params {
  count : u32,         // output values
  channels : u32,
  in_width : u32,
  in_height : u32,
  out_width : u32,
  out_height : u32,
  aux_a : u32,         // half index of the first scale vector
  aux_b : u32,
  flags : u32,
  pad0 : u32,
  pad1 : u32,
  pad2 : u32,
};

const FLAG_DUAL = 1u;   // also publish the raw half output

@group(0) @binding(0) var<uniform> params : Params;
@group(0) @binding(1) var<storage, read> in_f32 : array<f32>;
@group(0) @binding(2) var<storage, read> in_f16 : array<u32>;
@group(0) @binding(3) var<storage, read> in_e4 : array<u32>;
@group(0) @binding(4) var<storage, read> skip_e4 : array<u32>;
@group(0) @binding(8) var<storage, read> aux : array<u32>;      // half pairs
@group(0) @binding(6) var<storage, read_write> out_e4 : array<u32>;
@group(0) @binding(7) var<storage, read_write> out_f16 : array<u32>;

fn half_at(buffer: ptr<storage, array<u32>, read>, index: u32) -> f32 {
  return f16_to_f32(((*buffer)[index >> 1u] >> ((index & 1u) * 16u)) & 0xffffu);
}
fn e4_at(buffer: ptr<storage, array<u32>, read>, index: u32) -> f32 {
  return decode_e4m3(((*buffer)[index >> 2u] >> ((index & 3u) * 8u)) & 0xffu);
}
fn aux_half(index: u32) -> f32 {
  return f16_to_f32((aux[index >> 1u] >> ((index & 1u) * 16u)) & 0xffffu);
}

/// A 1D invocation index. A dispatch wider than 65535 workgroups is folded into the second dimension, which is
/// the only way to cover a full-resolution field: 576x512 alone is 294912 rows.
fn linear_index(id: vec3<u32>) -> u32 { return id.x + id.y * 65535u * 64u; }

fn publish(index: u32, values: array<f32, 4>) {
  var word = 0u;
  for (var i = 0u; i < 4u; i = i + 1u) { word = word | (encode_e4m3(f16_bits(values[i])) << (i * 8u)); }
  out_e4[index >> 2u] = word;
  if ((params.flags & FLAG_DUAL) != 0u) {
    out_f16[index >> 1u] = f16_bits(values[0]) | (f16_bits(values[1]) << 16u);
    out_f16[(index >> 1u) + 1u] = f16_bits(values[2]) | (f16_bits(values[3]) << 16u);
  }
}

/// The f32 input features, published to the half grid for the adapter's A operand.
@compute @workgroup_size(64)
fn convert_f32_to_f16(@builtin(global_invocation_id) id : vec3<u32>) {
  let base = linear_index(id) * 4u;
  if (base >= params.count) { return; }
  out_f16[base >> 1u] = f16_bits(in_f32[base]) | (f16_bits(in_f32[base + 1u]) << 16u);
  out_f16[(base >> 1u) + 1u] = f16_bits(in_f32[base + 2u]) | (f16_bits(in_f32[base + 3u]) << 16u);
}

/// The 2x2 box pool that takes a stage down a level: three half adds as (a+b)+(c+d), one half multiply by
/// 0.25, then the E4M3 publication. Every intermediate is a half, and the association is not free.
@compute @workgroup_size(64)
fn downsample(@builtin(global_invocation_id) id : vec3<u32>) {
  let base = linear_index(id) * 4u;
  if (base >= params.count) { return; }
  let c = base % params.channels;
  let pixel = base / params.channels;
  let ox = pixel % params.out_width;
  let oy = pixel / params.out_width;
  let sx = ox * 2u;
  let sy = oy * 2u;

  var values : array<f32, 4>;
  for (var i = 0u; i < 4u; i = i + 1u) { values[i] = 0.0; }
  if (sx + 1u < params.in_width && sy + 1u < params.in_height) {
    let i00 = (sy * params.in_width + sx) * params.channels + c;
    let i10 = i00 + params.channels;
    let i01 = ((sy + 1u) * params.in_width + sx) * params.channels + c;
    let i11 = i01 + params.channels;
    for (var i = 0u; i < 4u; i = i + 1u) {
      let top = round_f16(half_at(&in_f16, i00 + i) + half_at(&in_f16, i10 + i));
      let bottom = round_f16(half_at(&in_f16, i01 + i) + half_at(&in_f16, i11 + i));
      values[i] = round_f16(round_f16(top + bottom) * 0.25);
    }
  }
  publish(base, values);
}

/// A decoder stage's entry: the level below, doubled, plus the encoder skip times a learned per-channel scale.
/// The residual product is not published before the add - the whole expression rounds once.
@compute @workgroup_size(64)
fn upsample_residual(@builtin(global_invocation_id) id : vec3<u32>) {
  let base = linear_index(id) * 4u;
  if (base >= params.count) { return; }
  let c = base % params.channels;
  let pixel = base / params.channels;
  let source = (((pixel / params.out_width) >> 1u) * params.in_width +
                ((pixel % params.out_width) >> 1u)) * params.channels + c;
  var values : array<f32, 4>;
  for (var i = 0u; i < 4u; i = i + 1u) {
    values[i] = round_f16(half_at(&in_f16, source + i) +
                          e4_at(&skip_e4, base + i) * aux_half(params.aux_a + c + i));
  }
  publish(base, values);
}

/// The last merge before block 70: the level-0 decoder output doubled and scaled, plus block 0's own output
/// scaled. The first product is published to half, the second is not - it is folded into the final rounding.
@compute @workgroup_size(64)
fn post_blend(@builtin(global_invocation_id) id : vec3<u32>) {
  let base = linear_index(id) * 4u;
  if (base >= params.count) { return; }
  let c = base % params.channels;
  let pixel = base / params.channels;
  let source = (((pixel / params.out_width) >> 1u) * params.in_width +
                ((pixel % params.out_width) >> 1u)) * params.channels + c;
  var values : array<f32, 4>;
  for (var i = 0u; i < 4u; i = i + 1u) {
    let upsampled = round_f16(e4_at(&in_e4, source + i) * aux_half(params.aux_a + c + i));
    values[i] = round_f16(upsampled + e4_at(&skip_e4, base + i) * aux_half(params.aux_b + c + i));
  }
  publish(base, values);
}


```
