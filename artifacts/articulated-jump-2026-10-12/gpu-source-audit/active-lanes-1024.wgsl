// Experimental resident active_ids-set candidate. Native f64 admission owns publication.
// Fixed 64-lane reductions and twice modified Gram-Schmidt retain square-root factors.
// Qualification kernel: paired-f32 norm and vector normalization.
// Fixed 64-lane reduction tree; persistent storage is separate work.
@group(0) @binding(0) var<storage, read_write> data: array<u32>;
// Each workgroup owns one complete, disjoint operator in a batched payload.
var<workgroup> operator_base:u32;
fn rounded_add(a:f32,b:f32)->f32 { return fma(a,1.0,b); }
fn rounded_mul(a:f32,b:f32)->f32 { return fma(a,b,0.0); }
fn add(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let s=rounded_add(a.x,b.x); let v=rounded_add(s,-a.x);
    let left=rounded_add(a.x,-rounded_add(s,-v));
    let right=rounded_add(b.x,-v);
    let e=rounded_add(rounded_add(rounded_add(left,right),a.y),b.y);
    let hi=rounded_add(s,e);return vec2<f32>(hi,rounded_add(e,-rounded_add(hi,-s)));
}
fn neg(a:vec2<f32>)->vec2<f32> { return -a; }
fn mul(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let p=rounded_mul(a.x,b.x);
    let error=fma(a.x,b.x,-p);
    let cross=rounded_add(rounded_add(error,rounded_mul(a.x,b.y)),rounded_mul(a.y,b.x));
    let e=rounded_add(cross,rounded_mul(a.y,b.y));
    let hi=rounded_add(p,e);return vec2<f32>(hi,rounded_add(e,-rounded_add(hi,-p)));
}

fn load_pair(i:u32)->vec2<f32> {return vec2<f32>(bitcast<f32>(data[operator_base+i]),bitcast<f32>(data[operator_base+i+1u]));}
fn finite_pair(v:vec2<f32>)->bool {return all(abs(v)<=vec2<f32>(3.402823466e38));}
fn divide(a:vec2<f32>,b:vec2<f32>)->vec2<f32> {
    let q=a.x/b.x;
    let remainder=add(a,neg(mul(b,vec2<f32>(q,0.0))));
    return add(vec2<f32>(q,0.0),vec2<f32>((remainder.x+remainder.y)/b.x,0.0));
}
var<workgroup> partial:array<vec2<f32>,1024>;
var<workgroup> scalar:vec2<f32>;
var<workgroup> norm_scale:f32;
var<workgroup> row_count:u32;
var<workgroup> coordinate_count:u32;
var<workgroup> iteration_limit:u32;
var<workgroup> active_count:u32;
var<workgroup> built_count:u32;
var<workgroup> decision:u32;
var<workgroup> release_index:u32;
var<workgroup> fraction:vec2<f32>;
var<workgroup> active_ids:array<u32,512>;
var<workgroup> flags:array<u32,512>;
// Nonzero status always rejects publication. Bits identify the unchanged
// rejection gate: projection=1, norm sum=2, nonpositive squared norm=4, norm=8, division=16.
var<workgroup> invalid:atomic<u32>;
fn store_pair(offset:u32,v:vec2<f32>) {data[operator_base+offset]=bitcast<u32>(v.x);data[operator_base+offset+1u]=bitcast<u32>(v.y);}

fn factor_column(column:u32,lane:u32,width:u32,count:u32,input:u32,q:u32,r:u32) {
    for(var i=lane;i<width;i+=1024u) {store_pair(q+(column*width+i)*2u,load_pair(input+(active_ids[column]*width+i)*2u));}
    for(var j=lane;j<count;j+=1024u) {store_pair(r+(column*count+j)*2u,vec2<f32>(0.0));}
    storageBarrier();workgroupBarrier();
    // Twice modified Gram-Schmidt: every projection updates the current vector
    // before the next basis product. Basis and triangular coefficients stay resident.
    for(var sweep=0u;sweep<2u;sweep++) {
        for(var j=0u;j<column;j++) {
            var sum=vec2<f32>(0.0);
            for(var i=lane;i<width;i+=1024u) {sum=add(sum,mul(load_pair(q+(j*width+i)*2u),load_pair(q+(column*width+i)*2u)));}
            partial[lane]=sum;workgroupBarrier();
            for(var stride=512u;stride>0u;stride/=2u) {
                if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
                workgroupBarrier();
            }
            if lane==0u {scalar=partial[0];store_pair(r+(column*count+j)*2u,add(load_pair(r+(column*count+j)*2u),scalar));}
            workgroupBarrier();
            for(var i=lane;i<width;i+=1024u) {
                let offset=q+(column*width+i)*2u;
                let value=add(load_pair(offset),neg(mul(load_pair(q+(j*width+i)*2u),scalar)));
                if !finite_pair(value) {atomicOr(&invalid,1u);}
                store_pair(offset,value);
            }
            storageBarrier();workgroupBarrier();
        }
    }
    var sum=vec2<f32>(0.0);
    for(var i=lane;i<width;i+=1024u) {let v=load_pair(q+(column*width+i)*2u);sum=add(sum,mul(v,v));}
    partial[lane]=sum;workgroupBarrier();
    for(var stride=512u;stride>0u;stride/=2u) {
        if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
        workgroupBarrier();
    }
    if lane==0u {
        norm_scale=1.0;
        // Squaring a representable residual can underflow on GPU hardware.
        // Re-evaluate only that range using a positive max-component scale.
        if finite_pair(partial[0]) && partial[0].x>=0.0 && partial[0].x<1.17549435e-38 {norm_scale=0.0;}
    }
    workgroupBarrier();
    if workgroupUniformLoad(&norm_scale)==0.0 {
        var maximum=0.0;
        for(var i=lane;i<width;i+=1024u) {
            let v=load_pair(q+(column*width+i)*2u);
            maximum=max(maximum,max(abs(v.x),abs(v.y)));
        }
        partial[lane]=vec2<f32>(maximum,0.0);workgroupBarrier();
        for(var stride=512u;stride>0u;stride/=2u) {
            if lane<stride {partial[lane].x=max(partial[lane].x,partial[lane+stride].x);}
            workgroupBarrier();
        }
        if lane==0u {norm_scale=partial[0].x;}
        workgroupBarrier();
        let scale=workgroupUniformLoad(&norm_scale);
        if scale>0.0 {
            sum=vec2<f32>(0.0);
            for(var i=lane;i<width;i+=1024u) {
                let v=divide(load_pair(q+(column*width+i)*2u),vec2<f32>(scale,0.0));
                sum=add(sum,mul(v,v));
            }
            partial[lane]=sum;workgroupBarrier();
            for(var stride=512u;stride>0u;stride/=2u) {
                if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
                workgroupBarrier();
            }
        }
    }
    if lane==0u {
        let total=partial[0];scalar=vec2<f32>(1.0,0.0);
        // Match the native positive-norm criterion; original f64 backward
        // accuracy and physical admission still reject unusable candidates.
        if !finite_pair(total)||total.x<=0.0 {
            if !finite_pair(total) {atomicOr(&invalid,2u);}
            if total.x<=0.0 {atomicOr(&invalid,4u);}
        }
        else {
            let first=sqrt(total.x);
            let error=add(total,neg(mul(vec2<f32>(first,0.0),vec2<f32>(first,0.0))));
            var norm=add(vec2<f32>(first,0.0),vec2<f32>((error.x+error.y)/(2.0*first),0.0));
            norm=add(norm,divide(add(total,neg(mul(norm,norm))),mul(vec2<f32>(2.0,0.0),norm)));
            if norm_scale!=1.0 {norm=mul(norm,vec2<f32>(norm_scale,0.0));}
            if !finite_pair(norm)||norm.x<=0.0 {atomicOr(&invalid,8u);}
            else {scalar=norm;store_pair(r+(column*count+column)*2u,norm);}
        }
    }
    workgroupBarrier();
    for(var i=lane;i<width;i+=1024u) {
        let offset=q+(column*width+i)*2u;let v=divide(load_pair(offset),scalar);
        if !finite_pair(v) {atomicOr(&invalid,16u);}
        store_pair(offset,v);
    }
    storageBarrier();workgroupBarrier();
}
@compute @workgroup_size(1024)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
    let batched=data[0]==0xffffffffu;
    if !batched&&group.x!=0u {return;}
    if batched&&group.x>=data[1] {return;}
    if lane==0u {
        operator_base=0u;
        if batched {operator_base=data[4u+group.x];}
        coordinate_count=data[operator_base];row_count=data[operator_base+1u];iteration_limit=data[operator_base+2u];
    }
    workgroupBarrier();
    let width=workgroupUniformLoad(&coordinate_count);let count=workgroupUniformLoad(&row_count);let limit=workgroupUniformLoad(&iteration_limit);
    let input=4u;let bounds=input+2u*count*width;let tolerance=bounds+2u*count;
    let q=tolerance+2u;let r=q+2u*count*width;let z=r+2u*count*count;
    let candidate_reactions=z+2u*count;let candidate=candidate_reactions+2u*count;
    let reactions=candidate+2u*width;let state=reactions+2u*count;
    let gaps=state+2u*width;let status=gaps+2u*count;
    if lane==0u {active_count=0u;built_count=0u;decision=1u;atomicStore(&invalid,0u);data[operator_base+status]=1u;}
    for(var i=lane;i<count;i+=1024u) {flags[i]=0u;store_pair(reactions+2u*i,vec2<f32>(0.0));}
    for(var i=lane;i<width;i+=1024u) {store_pair(state+2u*i,vec2<f32>(0.0));}
    storageBarrier();workgroupBarrier();
    for(var iteration=0u;iteration<limit;iteration++) {
        let selected=workgroupUniformLoad(&active_count);
        if selected>0u {
            let built=workgroupUniformLoad(&built_count);
            for(var column=built;column<selected;column++) {factor_column(column,lane,width,count,input,q,r);}
            if lane==0u {built_count=selected;decision=atomicLoad(&invalid);}
            workgroupBarrier();
            if workgroupUniformLoad(&decision)!=0u {return;}
            if lane==0u {
                // R-transpose z = active_ids bounds; fixed R stride survives append/release.
                for(var i=0u;i<selected;i++) {
                    var sum=vec2<f32>(0.0);
                    for(var j=0u;j<i;j++) {sum=add(sum,mul(load_pair(r+2u*(i*count+j)),load_pair(z+2u*j)));}
                    let value=divide(add(load_pair(bounds+2u*active_ids[i]),neg(sum)),load_pair(r+2u*(i*count+i)));
                    if !finite_pair(value) {atomicStore(&invalid,1u);}
                    store_pair(z+2u*i,value);
                }
                for(var reverse=0u;reverse<selected;reverse++) {
                    let i=selected-1u-reverse;var sum=vec2<f32>(0.0);
                    for(var j=i+1u;j<selected;j++) {sum=add(sum,mul(load_pair(r+2u*(j*count+i)),load_pair(candidate_reactions+2u*j)));}
                    let value=divide(add(load_pair(z+2u*i),neg(sum)),load_pair(r+2u*(i*count+i)));
                    if !finite_pair(value) {atomicStore(&invalid,1u);}
                    store_pair(candidate_reactions+2u*i,value);
                }
                release_index=count;fraction=vec2<f32>(1.0,0.0);
                for(var i=0u;i<selected;i++) {
                    let next=load_pair(candidate_reactions+2u*i);
                    if next.x+next.y<0.0 {
                        let old=load_pair(reactions+2u*active_ids[i]);let value=divide(old,add(old,neg(next)));
                        if !finite_pair(value)||value.x+value.y<0.0||value.x+value.y>1.0 {atomicStore(&invalid,1u);}
                        if release_index==count||value.x+value.y<fraction.x+fraction.y {release_index=i;fraction=value;}
                    }
                }
                decision=atomicLoad(&invalid);
            }
            storageBarrier();workgroupBarrier();
            if workgroupUniformLoad(&decision)!=0u {return;}
            for(var i=lane;i<width;i+=1024u) {
                var sum=vec2<f32>(0.0);
                for(var j=0u;j<selected;j++) {sum=add(sum,mul(load_pair(q+2u*(j*width+i)),load_pair(z+2u*j)));}
                if !finite_pair(sum) {atomicOr(&invalid,1u);}
                store_pair(candidate+2u*i,sum);
            }
            storageBarrier();workgroupBarrier();
            let released=workgroupUniformLoad(&release_index);let step=workgroupUniformLoad(&fraction);
            if released<count {
                for(var i=lane;i<width;i+=1024u) {let old=load_pair(state+2u*i);store_pair(state+2u*i,add(old,mul(step,add(load_pair(candidate+2u*i),neg(old)))));}
                for(var i=lane;i<selected;i+=1024u) {
                    let old=load_pair(reactions+2u*active_ids[i]);let value=add(old,mul(step,add(load_pair(candidate_reactions+2u*i),neg(old))));
                    if !finite_pair(value) {atomicOr(&invalid,1u);}
                    store_pair(reactions+2u*active_ids[i],select(value,vec2<f32>(0.0),value.x+value.y<0.0));
                }
                storageBarrier();workgroupBarrier();
                if lane==0u {
                    let id=active_ids[released];store_pair(reactions+2u*id,vec2<f32>(0.0));flags[id]=0u;
                    for(var i=released;i+1u<selected;i++) {active_ids[i]=active_ids[i+1u];}
                    active_count=selected-1u;built_count=min(built_count,released);
                }
                storageBarrier();workgroupBarrier();
                continue;
            }
            for(var i=lane;i<width;i+=1024u) {store_pair(state+2u*i,load_pair(candidate+2u*i));}
            for(var i=lane;i<count;i+=1024u) {store_pair(reactions+2u*i,vec2<f32>(0.0));}
            storageBarrier();workgroupBarrier();
            for(var i=lane;i<selected;i+=1024u) {store_pair(reactions+2u*active_ids[i],load_pair(candidate_reactions+2u*i));}
            storageBarrier();workgroupBarrier();
        }
        for(var row=lane;row<count;row+=1024u) {
            var sum=vec2<f32>(0.0);
            for(var i=0u;i<width;i++) {sum=add(sum,mul(load_pair(input+2u*(row*width+i)),load_pair(state+2u*i)));}
            let gap=add(sum,neg(load_pair(bounds+2u*row)));
            if !finite_pair(gap) {atomicOr(&invalid,1u);}
            store_pair(gaps+2u*row,gap);
        }
        storageBarrier();workgroupBarrier();
        if lane==0u {
            let tol=load_pair(tolerance);let epsilon=tol.x+tol.y;var enter=count;var worst=0.0;var complete=true;
            for(var row=0u;row<count;row++) {
                let gap_pair=load_pair(gaps+2u*row);let gap=gap_pair.x+gap_pair.y;
                let reaction=load_pair(reactions+2u*row);let positive=reaction.x+reaction.y>0.0;
                if (positive&&abs(gap)>epsilon)||(!positive&&gap< -epsilon) {complete=false;}
                if flags[row]==0u&&gap< -epsilon&&(enter==count||gap<worst) {enter=row;worst=gap;}
            }
            decision=2u;
            if atomicLoad(&invalid)==0u {
                if complete {decision=0u;data[operator_base+status]=0u;}
                else if enter<count&&active_count<count {active_ids[active_count]=enter;flags[enter]=1u;active_count+=1u;decision=1u;}
            }
        }
        storageBarrier();workgroupBarrier();
        if workgroupUniformLoad(&decision)!=1u {return;}
    }
}
