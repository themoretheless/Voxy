"""Check the Rust certificate arithmetic against exact rational witness distances."""
from fractions import Fraction
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import tempfile

DENOMINATOR = 1 << 24
SEED = 20261002
CASES = 256


def f32(bits):
    return struct.unpack("!f", struct.pack("!I", bits))[0]


def exact_squared(source, target, weights):
    maximum = Fraction(0)
    for corner in range(3):
        squared = Fraction(0)
        for axis in range(3):
            mapped = sum(
                Fraction(target[index * 3 + axis]) * weights[corner * 3 + index]
                / DENOMINATOR for index in range(3)
            )
            squared += (Fraction(source[corner * 3 + axis]) - mapped) ** 2
        maximum = max(maximum, squared)
    return maximum


def main():
    root = Path(__file__).resolve().parents[1]
    modules = "\n".join(
        f'#[path={json.dumps(str(root / "crates/voxy_render/src" / (name + ".rs")))}] mod {name}; pub use {name}::*;'
        for name in ("lod", "lod_certificate", "certified_lod", "lod_witness", "lod_subdivision", "lod_search")
    )
    harness = '''MODULES
use crate::lod_certificate as certificate;
use std::io::{self, BufRead};
fn main() {
 for line in io::stdin().lock().lines() {
  let input: Vec<u32> = line.unwrap().split_whitespace().map(|s| s.parse().unwrap()).collect();
  let points = |offset| std::array::from_fn::<_, 3, _>(|i| std::array::from_fn(|j| f32::from_bits(input[offset+i*3+j])));
  let a = points(0); let b = points(9);
  let weights = |offset| std::array::from_fn(|i| std::array::from_fn(|j| input[offset+i*3+j]));
  let forward = LodTriangleWitness { target_triangle: 0, weights: weights(18) };
  let reverse = LodTriangleWitness { target_triangle: 0, weights: weights(27) };
  let value = certify_lod_error(LodSurface { positions: &a, indices: &[0,1,2] }, LodSurface { positions: &b, indices: &[0,1,2] }, &[forward], &[reverse]).unwrap();
  let (proxy, radius) = certificate::barycentric_proxy(a, forward.weights[0]);
  let low = std::array::from_fn(|axis| b.iter().map(|p| f64::from(p[axis])).fold(f64::INFINITY,f64::min));
  let high = std::array::from_fn(|axis| b.iter().map(|p| f64::from(p[axis])).fold(f64::NEG_INFINITY,f64::max));
  let lower = lod_search::box_lower_squared(low,high,a);
  println!("{} {} {} {} {} {}", value.to_bits(), radius.to_bits(), proxy[0].to_bits(), proxy[1].to_bits(), proxy[2].to_bits(), lower.to_bits());
 }
}
'''.replace("MODULES", modules)
    rng = random.Random(SEED)
    records = []
    for case in range(CASES):
        bits = [(rng.randrange(2) << 31) | (rng.randrange(255) << 23)
                | rng.randrange(1 << 23) for _ in range(18)]
        # Include signed zeros and subnormal cancellation alongside full exponents.
        if case % 8 == 0:
            bits[:6] = [0, 0x80000000, 1, 0x80000001, 0x7f7fffff, 0xff7fffff]
        weights = []
        for _ in range(6):
            first = rng.randrange(DENOMINATOR + 1)
            second = rng.randrange(DENOMINATOR - first + 1)
            weights.extend([first, second, DENOMINATOR - first - second])
        records.append(bits + weights)
    with tempfile.TemporaryDirectory(prefix="voxy-lod-numeric-") as directory:
        work = Path(directory)
        (work / "oracle.rs").write_text(harness)
        subprocess.run(["rustc", "--edition", "2024", str(work / "oracle.rs"),
                        "-o", str(work / "oracle")], check=True, timeout=60)
        result = subprocess.run(
            [str(work / "oracle")], input="\n".join(" ".join(map(str, r)) for r in records),
            text=True, capture_output=True, check=True, timeout=30,
        )
    values = result.stdout.splitlines()
    if len(values) != CASES:
        raise RuntimeError("missing Rust results")
    for case, (record, value) in enumerate(zip(records, values)):
        output = list(map(int, value.split()))
        bound = struct.unpack("!d", struct.pack("!Q", output[0]))[0]
        source = list(map(f32, record[:9]))
        target = list(map(f32, record[9:18]))
        squared = max(exact_squared(source, target, record[18:27]),
                      exact_squared(target, source, record[27:36]))
        if not math.isfinite(bound) or bound < 0 or Fraction(bound) ** 2 < squared:
            raise RuntimeError(f"underestimated witness distance in case {case}")
        radius = struct.unpack("!d", struct.pack("!Q", output[1]))[0]
        proxy = list(map(f32, output[2:5]))
        exact_proxy_error = sum((
            sum(Fraction(source[index * 3 + axis]) * record[18 + index]
                / DENOMINATOR for index in range(3)) - Fraction(proxy[axis])
        ) ** 2 for axis in range(3))
        if not math.isfinite(radius) or radius < 0 or Fraction(radius) ** 2 < exact_proxy_error:
            raise RuntimeError(f"underestimated subdivision proxy radius in case {case}")
        lower = struct.unpack("!d", struct.pack("!Q", output[5]))[0]
        low = [min(target[index * 3 + axis] for index in range(3)) for axis in range(3)]
        high = [max(target[index * 3 + axis] for index in range(3)) for axis in range(3)]
        exact_lower = max(sum(max(
            Fraction(low[axis]) - Fraction(source[corner * 3 + axis]),
            Fraction(source[corner * 3 + axis]) - Fraction(high[axis]), Fraction(0),
        ) ** 2 for axis in range(3)) for corner in range(3))
        if not math.isfinite(lower) or lower < 0 or Fraction(lower) > exact_lower:
            raise RuntimeError(f"overestimated search-box lower bound in case {case}")
    print(json.dumps({"cases": CASES, "seed": SEED, "underestimates": 0,
                      "scope": "exact rational witness, proxy and search-box arithmetic; not all-input formal proof"}))


if __name__ == "__main__":
    main()
