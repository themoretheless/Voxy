// One bounded column per dispatch, then one solve dispatch. Input is immutable.
@group(0) @binding(0) var<storage,read_write> qr_words:array<u32>;
var<workgroup> qr_count:u32;var<workgroup> qr_width:u32;
var<workgroup> qr_column:u32;var<workgroup> qr_decision:u32;
var<workgroup> qr_invalid:atomic<u32>;
var<workgroup> qr_partial:array<FiveRounded,64>;
var<workgroup> qr_scalar:FiveRounded;
var<workgroup> qr_inverse:FiveRounded;
fn qr_zero()->FiveRounded {var out:FiveRounded;out.valid=1u;return out;}
fn qr_read(offset:u32)->FiveRounded {
    var out:FiveRounded;for(var i=0u;i<5u;i++){out.terms[i]=bitcast<f32>(qr_words[offset+i]);}
    out.exponent=bitcast<i32>(qr_words[offset+5u]);out.error_bound=bitcast<f32>(qr_words[offset+6u]);out.valid=qr_words[offset+7u];return out;
}
fn qr_write(offset:u32,value:FiveRounded){
    for(var i=0u;i<5u;i++){qr_words[offset+i]=bitcast<u32>(value.terms[i]);}
    qr_words[offset+5u]=bitcast<u32>(value.exponent);qr_words[offset+6u]=bitcast<u32>(value.error_bound);qr_words[offset+7u]=value.valid;
    if value.valid!=1u {atomicOr(&qr_invalid,1u);}
}
fn qr_input(offset:u32)->FiveRounded {
    var parts:array<f32,5>;for(var i=0u;i<5u;i++){parts[i]=bitcast<f32>(qr_words[offset+i]);}
    return five_from_operand(five_normalize_operand(parts));
}
fn qr_failure(code:u32,lane:u32){
    if lane==0u {qr_decision=atomicLoad(&qr_invalid);if qr_decision!=0u {qr_words[6]=code;}}
    storageBarrier();workgroupBarrier();
}
fn qr_reduce(lane:u32){
    workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u){if lane<stride {qr_partial[lane]=five_add(qr_partial[lane],qr_partial[lane+stride]);}workgroupBarrier();}
}
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32){
    if lane==0u {
        qr_count=qr_words[2];qr_width=qr_words[3];qr_column=qr_words[7];qr_decision=0u;atomicStore(&qr_invalid,0u);
        if qr_words[0]!=0x31513556u || qr_words[1]!=1u || qr_words[4]!=5u || qr_words[5]!=8u
            || qr_count==0u || qr_width==0u || qr_count>qr_width || qr_count>1024u || qr_words[15]!=arrayLength(&qr_words) {qr_words[6]=1u;qr_decision=1u;}
        if qr_words[6]!=0xffffffffu {qr_decision=1u;}
    }
    workgroupBarrier();
    if workgroupUniformLoad(&qr_decision)!=0u {return;}
    let count=workgroupUniformLoad(&qr_count);let width=workgroupUniformLoad(&qr_width);let column=workgroupUniformLoad(&qr_column);
    let input=qr_words[8];let bounds=qr_words[9];let q=qr_words[11];let r=qr_words[12];let state=qr_words[13];let reaction=qr_words[14];
    if column<count {
        if column==0u {for(var i=lane;i<count*count;i+=64u){qr_write(r+8u*i,qr_zero());}}
        for(var i=lane;i<width;i+=64u){qr_write(q+8u*(column*width+i),qr_input(input+5u*(column*width+i)));}
        qr_failure(2u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
        for(var sweep=0u;sweep<2u;sweep++) {for(var previous=0u;previous<column;previous++) {
            var sum=qr_zero();
            for(var i=lane;i<width;i+=64u){sum=five_add(sum,five_multiply(qr_read(q+8u*(previous*width+i)),qr_read(q+8u*(column*width+i))));}
            qr_partial[lane]=sum;qr_reduce(lane);
            if lane==0u {qr_scalar=qr_partial[0];qr_write(r+8u*(previous*count+column),five_add(qr_read(r+8u*(previous*count+column)),qr_scalar));}
            workgroupBarrier();
            let projection=qr_scalar;
            if !five_exact_zero(projection) {
                for(var i=lane;i<width;i+=64u){let at=q+8u*(column*width+i);qr_write(at,five_subtract(qr_read(at),five_multiply(qr_read(q+8u*(previous*width+i)),projection)));}
            }
            qr_failure(3u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
        }}
        var squared=qr_zero();
        for(var i=lane;i<width;i+=64u){let value=qr_read(q+8u*(column*width+i));squared=five_add(squared,five_multiply(value,value));}
        qr_partial[lane]=squared;qr_reduce(lane);
        if lane==0u {
            qr_scalar=five_sqrt(qr_partial[0]);
            if qr_scalar.valid!=1u || qr_scalar.terms[0]<=0.0 {atomicOr(&qr_invalid,1u);}
            qr_write(r+8u*(column*count+column),qr_scalar);
        }
        qr_failure(5u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
        if lane==0u {
            var one=qr_zero();one.terms[0]=1.0;qr_inverse=five_divide(one,qr_scalar);
            if qr_inverse.valid!=1u {
                for(var part=0u;part<5u;part++){qr_words[20u+part]=bitcast<u32>(qr_scalar.terms[part]);}
                qr_words[25]=bitcast<u32>(qr_scalar.exponent);qr_words[26]=bitcast<u32>(qr_scalar.error_bound);qr_words[27]=qr_scalar.valid;
                qr_words[28]=five_division_failure_stage;qr_words[29]=five_arithmetic_failure_code;qr_words[30]=five_arithmetic_failure_bits;qr_words[31]=bitcast<u32>(five_arithmetic_failure_exponent);
                atomicOr(&qr_invalid,1u);
            }
        }
        qr_failure(6u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
        let inverse=qr_inverse;
        for(var i=lane;i<width;i+=64u){let at=q+8u*(column*width+i);qr_write(at,five_multiply(qr_read(at),inverse));}
        qr_failure(7u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
        if lane==0u {qr_words[7]=column+1u;}return;
    }
    if lane==0u {
        for(var row=0u;row<count;row++) {
            var value=qr_input(bounds+5u*row);
            for(var previous=0u;previous<row;previous++){value=five_subtract(value,five_multiply(qr_read(r+8u*(previous*count+row)),qr_read(reaction+8u*previous)));}
            qr_write(reaction+8u*row,five_divide(value,qr_read(r+8u*(row*count+row))));
        }
    }
    qr_failure(8u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
    for(var i=lane;i<width;i+=64u){
        var value=qr_zero();for(var row=0u;row<count;row++){value=five_add(value,five_multiply(qr_read(q+8u*(row*width+i)),qr_read(reaction+8u*row)));}
        qr_write(state+8u*i,value);
    }
    qr_failure(9u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
    if lane==0u {
        for(var reverse=0u;reverse<count;reverse++) {
            let row=count-1u-reverse;var value=qr_read(reaction+8u*row);
            for(var next=row+1u;next<count;next++){value=five_subtract(value,five_multiply(qr_read(r+8u*(row*count+next)),qr_read(reaction+8u*next)));}
            qr_write(reaction+8u*row,five_divide(value,qr_read(r+8u*(row*count+row))));
        }
    }
    qr_failure(10u,lane);if workgroupUniformLoad(&qr_decision)!=0u{return;}
    if lane==0u {qr_words[7]=count+1u;qr_words[6]=0u;}
}
