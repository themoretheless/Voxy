// One independent nine-diagonal native Cosserat linear system per invocation.
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
fn load(i:u32)->f32 { return bitcast<f32>(data[i]); }
fn save(i:u32,v:f32) { data[i]=bitcast<u32>(v); }
@compute @workgroup_size(32)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if id.x>=data[0] { return; }
    let n=data[1]; let first=data[2]; let end=data[3];
    let base=4u+id.x*(n*10u+1u); let rhs=base+n*9u; let status=rhs+n;
    data[status]=0u;
    for(var i=first;i<end;i++) {
        let start=max(first,select(0u,i-8u,i>=8u));
        for(var j=start;j<=i;j++) {
            var sum=load(base+i*9u+i-j);
            let begin=max(start,select(0u,j-8u,j>=8u));
            for(var k=begin;k<j;k++) { sum-=load(base+i*9u+i-k)*load(base+j*9u+j-k); }
            if i==j {
                if !(sum>0.0) { data[status]=1u; return; }
                save(base+i*9u,sqrt(sum));
            } else { save(base+i*9u+i-j,sum/load(base+j*9u)); }
        }
    }
    for(var i=first;i<end;i++) {
        var value=load(rhs+i);
        let begin=max(first,select(0u,i-8u,i>=8u));
        for(var j=begin;j<i;j++) { value-=load(base+i*9u+i-j)*load(rhs+j); }
        save(rhs+i,value/load(base+i*9u));
    }
    for(var cursor=end;cursor>first;cursor--) {
        let i=cursor-1u; var value=load(rhs+i);
        for(var j=i+1u;j<min(i+9u,end);j++) { value-=load(base+j*9u+j-i)*load(rhs+j); }
        save(rhs+i,value/load(base+i*9u));
    }
}
