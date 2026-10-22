#!/usr/bin/env python3
"""Independent integer oracle for raw GPU exact-affine cache captures.
No expected result is uploaded to the GPU. No tolerance or float operations.
"""
import argparse,json,struct,hashlib
from pathlib import Path


def integer(bits,base=-149):
    e=(bits>>23)&255
    assert e!=255,'nonfinite original coefficient'
    mantissa=(bits&0x7fffff)|(0x800000 if e else 0)
    power=e-150 if e else -149
    assert power>=base
    return (-1 if bits>>31 else 1)*(mantissa<<(power-base))


def pack(value):
    if value==0:return (32768,0,0,0)
    magnitude=abs(value);assert magnitude.bit_length()<=2048
    low=((magnitude&-magnitude).bit_length()-1)//16
    significant=magnitude>>(low*16);live=(significant.bit_length()+15)//16
    if live>6:return None
    return (32768|(int(value<0)<<14)|(live<<8)|low,
            significant&0xffffffff,(significant>>32)&0xffffffff,(significant>>64)&0xffffffff)


def record(matrix,point):
    local=[integer(v) for v in point];words=[]
    for axis in range(3):
        value=integer(matrix[12+axis],-298)+sum(integer(matrix[k*4+axis])*local[k] for k in range(3))
        encoded=pack(value)
        if encoded is None:return (0,)*12,False
        words.extend(encoded)
    return tuple(words),True


def fixture_meshes(root):
    source=json.loads((root/'fixture-source.json').read_text())
    for mesh in source['geometry']:
        yield mesh['positions'],[sum(mesh['world'],[])],None,None


def exported_meshes(root):
    manifest=json.loads((root/'manifest.json').read_text())
    assert manifest['schema']=='voxy-full-world-ray-geometry-v1' and not manifest['geometry_quantization']
    for mesh in manifest['meshes']:
        vertices=(root/mesh['vertices']['file']).read_bytes();instances=(root/mesh['instances']['file']).read_bytes()
        assert len(vertices)==mesh['vertex_count']*48 and len(instances)==mesh['instance_count']*112
        yield [row[:3] for row in struct.iter_unpack('<12I',vertices)],list(struct.iter_unpack('<28I',instances)),vertices,instances


def audit(capture,source_export=None):
    description=json.loads(capture.read_text());assert description['schema']=='voxy-exact-vertex-cache-capture-v1'
    root=capture.parent;data=(root/description['buffer_file']).read_bytes();prefix=(root/description['source_prefix_file']).read_bytes();h=description['header_word']
    assert len(data)%4==0 and data[:h*4]==prefix and len(prefix)==h*4
    header=struct.unpack_from('<16I',data,h*4)
    assert header[0]==int(not description['ready_cleared_for_fallback_test'])
    assert header[4]==12 and header[5]==0 and header[6]==header[2]
    assert header[8:10]==tuple(description['geometry_revision'])
    expected_prefix=[0];slot=world_vertex=source_vertex=admitted=fallback=0
    gpu_instances=(root/description['source_instances_file']).read_bytes() if 'source_instances_file' in description else None
    instance_offset=0
    meshes=exported_meshes(source_export) if source_export else fixture_meshes(root)
    for positions,matrices,all_vertex_bytes,all_instance_bytes in meshes:
        if gpu_instances is not None:
            assert all_instance_bytes is not None,'instance payload verification needs source export'
            assert gpu_instances[instance_offset:instance_offset+len(all_instance_bytes)]==all_instance_bytes,'original GPU instance coefficients differ from source export'
            instance_offset+=len(all_instance_bytes)
        if all_vertex_bytes is not None:
            assert prefix[source_vertex*48:source_vertex*48+len(all_vertex_bytes)]==all_vertex_bytes
        else:
            for local,point in enumerate(positions):
                assert struct.unpack_from('<3I',prefix,(source_vertex+local)*48)==tuple(point)
        for matrix in matrices:
            expected_prefix.append(expected_prefix[-1]+len(positions))
            for local,point in enumerate(positions):
                expected,fits=record(matrix,point);at=header[3]+world_vertex*12
                actual=struct.unpack_from('<12I',data,at*4)
                assert actual==expected,('exact cache mismatch',slot,local,actual,expected)
                admitted+=int(fits);fallback+=int(not fits);world_vertex+=1
            slot+=1
        source_vertex+=len(positions)
    assert slot==header[1] and world_vertex==header[2]
    if gpu_instances is not None:assert instance_offset==len(gpu_instances)
    actual_prefix=struct.unpack_from('<'+'I'*len(expected_prefix),data,(h+16)*4)
    assert actual_prefix==tuple(expected_prefix)
    assert header[3]==(h+16+len(expected_prefix)+3)&~3
    assert len(data)==(header[3]+world_vertex*12)*4
    return {'capture':capture.name,'source_slots':slot,'world_vertices':world_vertex,'all_12_record_words_checked':world_vertex*12,'lossless_admitted_vertices':admitted,'complete_original_fallback_vertices':fallback,'source_prefix_bytes_bit_identical':len(prefix),'original_GPU_instance_bytes_checked':instance_offset,'buffer_sha256':hashlib.sha256(data).hexdigest(),'ready_cleared':description['ready_cleared_for_fallback_test'],'passed':True}


def main():
    parser=argparse.ArgumentParser();parser.add_argument('root',type=Path);parser.add_argument('--source-export',type=Path);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    captures=sorted(args.root.glob('vertex-cache-*.json'));assert captures,'no actual cache capture descriptors'
    reports=[audit(p,args.source_export) for p in captures]
    proof={'status':'all_captured_exact_vertex_records_match_independent_original_source_integer_oracle','captures':reports,'rational_or_float_tolerance_used':False,'performance_qualified':False,'minimum_120_fps_qualified':False}
    args.output.write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps(proof,indent=2))
if __name__=='__main__':main()
