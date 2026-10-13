#!/usr/bin/env python3
"""Exact saved triple-coordinate versus f64 audit; no physical admission claim."""
import argparse, bisect, functools, hashlib, json, math, struct
from fractions import Fraction
from pathlib import Path

if not __debug__:raise RuntimeError("Run this research audit without -O; validation assertions are required")

@functools.lru_cache(maxsize=200000)
def f64(bits):
    v=struct.unpack("<d",struct.pack("<Q",bits))[0]
    if not math.isfinite(v):raise ValueError("nonfinite f64")
    return Fraction.from_float(v)
@functools.lru_cache(maxsize=200000)
def f32(bits):
    v=struct.unpack("<f",struct.pack("<I",bits))[0]
    if not math.isfinite(v):raise ValueError("nonfinite f32")
    return Fraction.from_float(v)
def words(raw,size):
    if len(raw)%size:raise ValueError("unaligned file")
    return struct.unpack("<"+("I" if size==4 else "Q")*(len(raw)//size),raw)
def bits64(v):return struct.unpack("<Q",struct.pack("<d",v))[0]
def local_file(parent,name):
    if Path(name).name!=name:raise ValueError("capture must be a local basename")
    return parent/name

def audit(path):
    m=json.loads(path.read_text());assert m["schema"]=="voxy-triple-opaque-decode-audit-v1"
    parent=path.parent
    inp=local_file(parent,m["input_file"]).read_bytes();out=local_file(parent,m["output_file"]).read_bytes()
    raw_columns=local_file(parent,m["original_columns_file"]).read_bytes()
    iw=words(inp,4);cw=words(raw_columns,8);n=m["rows"];w=m["coordinates"]
    assert 0<n<=1024 and w>0 and iw[:2]==(n,w) and len(cw)==n*w
    ranges=m["readback_ranges_bytes"];mapping=[];offset=0;last=0
    for start,end in ranges:
        assert start%4==end%4==0 and 0<=start<end<=len(inp) and start>=last
        mapping.append((start//4,end//4,offset));offset+=end-start;last=end
    assert offset==len(out)
    starts=[r[0] for r in mapping]
    def output_word(index):
        slot=bisect.bisect_right(starts,index)-1
        assert slot>=0
        begin,end,offset=mapping[slot];assert begin<=index<end
        return struct.unpack_from("<I",out,offset+4*(index-begin))[0]
    def triple(word,index):return sum((f32(word(index+j)) for j in range(3)),Fraction())
    reference_row,reference_axis,reference_bits=m["column_reference_row_axis_bits"]
    assert cw[reference_row*w+reference_axis]==reference_bits
    column_scale=f64(reference_bits)/triple(lambda i:iw[i],4+3*(reference_row*w+reference_axis))
    tolerance_offset=4+3*n*w+3*n
    bound_scale=f64(m["tolerance_bits"])/triple(lambda i:iw[i],tolerance_offset)
    state_scale=bound_scale/column_scale
    assert column_scale>0 and bound_scale>0
    for scale in (column_scale,bound_scale):
        assert scale.numerator&(scale.numerator-1)==0 and scale.denominator&(scale.denominator-1)==0
    # Independently check EVERY packed coefficient against the actual API f64.
    for i,original in enumerate(cw):assert triple(lambda j:iw[j],4+3*i)*column_scale==f64(original)
    assert len(m["decoded_coordinates_bits"])==w and len(m["decoded_reactions_bits"])==n and len(m["effective_bounds_bits"])==n
    bounds_offset=4+3*n*w
    for row,bits in enumerate(m["effective_bounds_bits"]):assert triple(lambda j:iw[j],bounds_offset+3*row)*bound_scale==f64(bits)
    state=len(iw)-1-3*n-3*w;reaction=state+3*w
    assert output_word(len(iw)-1)==0 and 0<output_word(3)<=iw[2]
    expanded=[triple(output_word,state+3*i)*state_scale for i in range(w)]
    decoded=[f64(v) for v in m["decoded_coordinates_bits"]]
    reactions=[f64(v) for v in m["decoded_reactions_bits"]]
    wrong_coordinates=sum(bits64(float(x))!=bits and not (x==0 and f64(bits)==0) for x,bits in zip(expanded,m["decoded_coordinates_bits"]))
    expanded_reactions=[triple(output_word,reaction+3*i)*state_scale/column_scale for i in range(n)]
    wrong_reactions=sum(bits64(float(x))!=bits and not (x==0 and f64(bits)==0) for x,bits in zip(expanded_reactions,m["decoded_reactions_bits"]))
    tol=f64(m["tolerance_bits"]);expanded_ok=True;decoded_ok=True;rows=[]
    for row in range(n):
        a=b=Fraction()
        for axis in range(w):
            c=f64(cw[row*w+axis])
            if c:a+=c*expanded[axis];b+=c*decoded[axis]
        bound=f64(m["effective_bounds_bits"][row]);a-=bound;b-=bound
        active=reactions[row]>0
        expanded_ok &= abs(a)<=tol if active else a>= -tol
        decoded_ok &= abs(b)<=tol if active else b>= -tol
        if active or row==2:
            rows.append({"row":row,"active":active,"expanded_gap":float(a),"decoded_gap":float(b),"projection_of_decode_error":float(b-a),
                "expanded_gap_exact":[str(a.numerator),str(a.denominator)],"decoded_gap_exact":[str(b.numerator),str(b.denominator)]})
    return {"capture":path.name,"sequence":m["sequence"],"rows":n,"coordinates":w,
        "input_sha256":hashlib.sha256(inp).hexdigest(),"output_sha256":hashlib.sha256(out).hexdigest(),"all_original_coefficients_exact":True,"all_original_bounds_exact":True,
        "nonzero_coordinate_rounding_mismatches":wrong_coordinates,"nonzero_reaction_rounding_mismatches":wrong_reactions,
        "expanded_original_coordinate_gate":bool(expanded_ok),"decoded_original_coordinate_gate":bool(decoded_ok),
        "max_coordinate_quantization_error":float(max(abs(a-b) for a,b in zip(expanded,decoded))),"active_and_failed_row_details":rows,
        "scope":"exact rational original coordinate audit only; excludes triangular backtransform and original physical admission; zero signs ignored"}

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("captures",type=Path);parser.add_argument("--output",type=Path,required=True);args=parser.parse_args()
    results=[]
    for path in sorted(args.captures.glob("*.json")):
        result=audit(path);results.append(result)
        print(json.dumps({k:result[k] for k in ["sequence","expanded_original_coordinate_gate","decoded_original_coordinate_gate","nonzero_coordinate_rounding_mismatches"]}),flush=True)
    assert results
    args.output.write_text(json.dumps(results,indent=2)+"\n")
if __name__=="__main__":main()
