from fractions import Fraction as F
import struct

def bits32(bits):
    return F.from_float(struct.unpack('!f',struct.pack('!I',bits))[0])
wall_center=F(8388611,128)
wall_half=bits32(0x3c23d70a)
body_half=bits32(0x3a83126f)
wall_left=wall_center-wall_half
contact=wall_left-body_half
# Exact nearest-even rounding on the f32 lattice above 2**16.
scaled=contact*128
low=scaled.numerator//scaled.denominator
remainder=scaled-low
rounded=low+(remainder>F(1,2) or (remainder==F(1,2) and low%2==1))
published=F(rounded,128)
assert published==F(65536)+F(1,64)
assert contact+body_half==wall_left
assert published+body_half>wall_left
safe=F(65536)+F(1,128)
assert safe+body_half<wall_left
print('PASS: exact dyadic source dimensions; contact is safe before narrowing and penetrates after nearest-even f32 publication')
print('PASS: preceding f32 center retains disjoint interiors')
