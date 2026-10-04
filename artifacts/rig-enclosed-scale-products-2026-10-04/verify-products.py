from fractions import Fraction
from pathlib import Path
import re
import struct

def decode(bits):
    return struct.unpack(">d", int(bits).to_bytes(8, "big"))[0]

products = {}
count = 0
log = Path(__file__).with_name("scale-products.log").read_text()
for group, step, factor, lo, hi in re.findall(r"scale-product:(\d+):(\d+):(\d+):(\d+):(\d+)", log):
    products[group] = products.get(group, Fraction(1)) * Fraction.from_float(decode(factor))
    assert Fraction.from_float(decode(lo)) <= products[group] <= Fraction.from_float(decode(hi))
    count += 1
assert count == 8
print(f"{count} exact rational product enclosures verified; no epsilon used")
