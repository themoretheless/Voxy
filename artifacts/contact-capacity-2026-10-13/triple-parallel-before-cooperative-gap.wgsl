// Experimental floating-point expansion primitives. No physical admission.
// Grow-expansion / error-free sum approach: Shewchuk, robust arithmetic (1997).
// A grow inserts at most one term. Addition inserts six values; multiplication
// inserts nine product/error pairs. All 18 terms fit, including nonfinite paths.
struct Expansion {terms:array<f32,18>,count:u32}
fn triple_sum_error(a:f32,b:f32)->vec2<f32> {
    let s=fma(a,1.0,b);let v=fma(s,1.0,-a);
    let left=fma(a,1.0,-fma(s,1.0,-v));let right=fma(b,1.0,-v);
    return vec2<f32>(s,fma(left,1.0,right));
}
fn triple_grow(e:ptr<function,Expansion>,value:f32) {
    // Each output index is <= the input index already read. Compact in place
    // without changing the error-free addition order or touching unread terms.
    let original_count=(*e).count;var count=0u;var q=value;
    for(var i=0u;i<original_count;i++) {
        let sum=triple_sum_error(q,(*e).terms[i]);
        if sum.y!=0.0 {(*e).terms[count]=sum.y;count+=1u;}
        q=sum.x;
    }
    if q!=0.0 || count==0u {(*e).terms[count]=q;count+=1u;}
    (*e).count=count;
}
fn triple_finish(e:Expansion)->vec3<f32> {
    // Compress before selecting three terms: grow-expansion output can contain
    // several overlapping low terms whose sum changes the leading components.
    var h:array<f32,18>;var bottom=e.count;var q=e.terms[e.count-1u];
    for(var index=e.count-1u;index>0u;index-=1u) {
        let pair=triple_sum_error(q,e.terms[index-1u]);
        if pair.y!=0.0 {bottom-=1u;h[bottom]=pair.x;q=pair.y;}
        else {q=pair.x;}
    }
    bottom-=1u;h[bottom]=q;var top=0u;q=h[bottom];
    for(var index=bottom+1u;index<e.count;index+=1u) {
        let pair=triple_sum_error(h[index],q);
        if pair.y!=0.0 {h[top]=pair.y;top+=1u;}
        q=pair.x;
    }
    h[top]=q;
    var out=vec3<f32>(0.0);
    for(var i=0u;i<min(3u,top+1u);i++) {out[i]=h[top-i];}
    return out;
}
fn triple_add(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {
    if all(a==vec3<f32>(0.0)) {return b;}
    if all(b==vec3<f32>(0.0)) {return a;}
    var e:Expansion;
    for(var i=0u;i<3u;i++) {triple_grow(&e,a[2u-i]);}
    for(var i=0u;i<3u;i++) {triple_grow(&e,b[2u-i]);}
    return triple_finish(e);
}
fn triple_mul(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {
    // Exact zeros only. A nonfinite other operand must still propagate failure.
    if (all(a==vec3<f32>(0.0))&&all(abs(b)<=vec3<f32>(3.402823466e38)))
        ||(all(b==vec3<f32>(0.0))&&all(abs(a)<=vec3<f32>(3.402823466e38))) {return vec3<f32>(0.0);}
    var e:Expansion;
    for(var i=0u;i<3u;i++) {for(var j=0u;j<3u;j++) {
        let x=a[2u-i];let y=b[2u-j];let product=fma(x,y,0.0);
        let error=fma(x,y,-product);
        triple_grow(&e,error);triple_grow(&e,product);
    }}
    return triple_finish(e);
}

// Experimental residual-corrected division. Caller must supply a nonzero,
// normalized denominator and keep intermediate values within f32 exponent range.
fn triple_div(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {
    var remainder=a;var quotient=vec3<f32>(0.0);
    for(var iteration=0u;iteration<4u;iteration++) {
        let correction=vec3<f32>(remainder.x/b.x,0.0,0.0);
        quotient=triple_add(quotient,correction);
        remainder=triple_add(remainder,-triple_mul(b,correction));
    }
    return quotient;
}

// Nonnegative normalized input; zero is exact. Negative inputs are outside
// this primitive's contract and must be rejected by the calling solver.
fn triple_sqrt(a:vec3<f32>)->vec3<f32> {
    if all(a==vec3<f32>(0.0)) {return vec3<f32>(0.0);}
    var root=vec3<f32>(sqrt(a.x),0.0,0.0);
    for(var iteration=0u;iteration<3u;iteration++) {
        let residual=triple_add(a,-triple_mul(root,root));
        let correction=triple_div(residual,root*2.0);
        root=triple_add(root,correction);
    }
    return root;
}

// Experimental 64-lane triple candidate; native original physics owns admission.
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
var<workgroup> operator_base:u32;
fn add(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {return triple_add(a,b);}
fn neg(a:vec3<f32>)->vec3<f32> {return -a;}
fn mul(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {return triple_mul(a,b);}
fn divide(a:vec3<f32>,b:vec3<f32>)->vec3<f32> {return triple_div(a,b);}
fn load_pair(i:u32)->vec3<f32> {return vec3<f32>(bitcast<f32>(data[operator_base+i]),bitcast<f32>(data[operator_base+i+1u]),bitcast<f32>(data[operator_base+i+2u]));}
fn store_pair(i:u32,v:vec3<f32>) {for(var part=0u;part<3u;part++) {data[operator_base+i+part]=bitcast<u32>(v[part]);}}
fn finite_pair(v:vec3<f32>)->bool {return all(abs(v)<=vec3<f32>(3.402823466e38));}
var<workgroup> partial:array<vec3<f32>,64>;
var<workgroup> scalar:vec3<f32>;
var<workgroup> norm_scale:f32;
var<workgroup> row_count:u32;
var<workgroup> coordinate_count:u32;
var<workgroup> iteration_limit:u32;
var<workgroup> active_count:u32;
var<workgroup> built_count:u32;
var<workgroup> decision:u32;
var<workgroup> release_index:u32;
var<workgroup> fraction:vec3<f32>;
var<workgroup> active_ids:array<u32,1024>;
var<workgroup> flags:array<u32,1024>;
var<workgroup> invalid:atomic<u32>;
fn factor_column(column:u32,lane:u32,width:u32,count:u32,input:u32,q:u32,r:u32) {
    for(var i=lane;i<width;i+=64u) {store_pair(q+(column*width+i)*3u,load_pair(input+(active_ids[column]*width+i)*3u));}
    for(var j=lane;j<count;j+=64u) {store_pair(r+(column*count+j)*3u,vec3<f32>(0.0));}
    storageBarrier();workgroupBarrier();
    // Twice modified Gram-Schmidt: every projection updates the current vector
    // before the next basis product. Basis and triangular coefficients stay resident.
    for(var sweep=0u;sweep<2u;sweep++) {
        for(var j=0u;j<column;j++) {
            var sum=vec3<f32>(0.0);
            for(var i=lane;i<width;i+=64u) {sum=add(sum,mul(load_pair(q+(j*width+i)*3u),load_pair(q+(column*width+i)*3u)));}
            partial[lane]=sum;workgroupBarrier();
            for(var stride=32u;stride>0u;stride/=2u) {
                if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
                workgroupBarrier();
            }
            if lane==0u {scalar=partial[0];store_pair(r+(column*count+j)*3u,add(load_pair(r+(column*count+j)*3u),scalar));}
            workgroupBarrier();
            // The preceding workgroup barrier publishes the same scalar to every lane.
            let projection=scalar;
            if any(projection!=vec3<f32>(0.0)) {
                for(var i=lane;i<width;i+=64u) {
                    let offset=q+(column*width+i)*3u;
                    let value=add(load_pair(offset),neg(mul(load_pair(q+(j*width+i)*3u),scalar)));
                    if !finite_pair(value) {atomicOr(&invalid,1u);}
                    store_pair(offset,value);
                }
            }
            storageBarrier();workgroupBarrier();
        }
    }
    var sum=vec3<f32>(0.0);
    for(var i=lane;i<width;i+=64u) {let v=load_pair(q+(column*width+i)*3u);sum=add(sum,mul(v,v));}
    partial[lane]=sum;workgroupBarrier();
    for(var stride=32u;stride>0u;stride/=2u) {
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
        for(var i=lane;i<width;i+=64u) {
            let v=load_pair(q+(column*width+i)*3u);
            maximum=max(maximum,max(abs(v.x),abs(v.y)));
        }
        partial[lane]=vec3<f32>(maximum,0.0,0.0);workgroupBarrier();
        for(var stride=32u;stride>0u;stride/=2u) {
            if lane<stride {partial[lane].x=max(partial[lane].x,partial[lane+stride].x);}
            workgroupBarrier();
        }
        if lane==0u {norm_scale=partial[0].x;}
        workgroupBarrier();
        let scale=workgroupUniformLoad(&norm_scale);
        if scale>0.0 {
            sum=vec3<f32>(0.0);
            for(var i=lane;i<width;i+=64u) {
                let v=divide(load_pair(q+(column*width+i)*3u),vec3<f32>(scale,0.0,0.0));
                sum=add(sum,mul(v,v));
            }
            partial[lane]=sum;workgroupBarrier();
            for(var stride=32u;stride>0u;stride/=2u) {
                if lane<stride {partial[lane]=add(partial[lane],partial[lane+stride]);}
                workgroupBarrier();
            }
        }
    }
    if lane==0u {
        let total=partial[0];scalar=vec3<f32>(1.0,0.0,0.0);
        // Match the native positive-norm criterion; original f64 backward
        // accuracy and physical admission still reject unusable candidates.
        if !finite_pair(total)||total.x<=0.0 {
            if !finite_pair(total) {atomicOr(&invalid,2u);}
            if total.x<=0.0 {atomicOr(&invalid,4u);}
        }
        else {
            var norm=triple_sqrt(total);
            if norm_scale!=1.0 {norm=mul(norm,vec3<f32>(norm_scale,0.0,0.0));}
            if !finite_pair(norm)||norm.x<=0.0 {atomicOr(&invalid,8u);}
            else {scalar=norm;store_pair(r+(column*count+column)*3u,norm);}
        }
    }
    workgroupBarrier();
    for(var i=lane;i<width;i+=64u) {
        let offset=q+(column*width+i)*3u;let v=divide(load_pair(offset),scalar);
        if !finite_pair(v) {atomicOr(&invalid,16u);}
        store_pair(offset,v);
    }
    storageBarrier();workgroupBarrier();
}
@compute @workgroup_size(64)
fn cs_main(@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>) {
    if lane==0u {operator_base=0u;coordinate_count=data[1];row_count=data[0];iteration_limit=data[2];}
    workgroupBarrier();
    let width=workgroupUniformLoad(&coordinate_count);let count=workgroupUniformLoad(&row_count);let limit=workgroupUniformLoad(&iteration_limit);
    let input=4u;let bounds=input+3u*count*width;let tolerance=bounds+3u*count;
    let q=tolerance+3u;let r=q+3u*count*width;
    let candidate=r+3u*count*count;let candidate_reactions=candidate+3u*width;
    let z=candidate_reactions;let state=candidate_reactions+3u*count;let reactions=state+3u*width;
    // Candidate reactions are no longer needed after copying live reactions.
    // Reuse that scratch for gaps so retained Q/R prefixes remain immutable.
    let gaps=candidate_reactions;let status=reactions+3u*count;
    if lane==0u {active_count=0u;built_count=0u;decision=1u;atomicStore(&invalid,0u);data[operator_base+status]=1u;}
    for(var i=lane;i<count;i+=64u) {flags[i]=0u;store_pair(reactions+3u*i,vec3<f32>(0.0));}
    for(var i=lane;i<width;i+=64u) {store_pair(state+3u*i,vec3<f32>(0.0));}
    storageBarrier();workgroupBarrier();
    for(var iteration=0u;iteration<limit;iteration++) {
        if lane==0u {data[3]=iteration+1u;}
        let selected=workgroupUniformLoad(&active_count);
        if selected>0u {
            let built=workgroupUniformLoad(&built_count);
            for(var column=built;column<selected;column++) {factor_column(column,lane,width,count,input,q,r);}
            if lane==0u {built_count=selected;decision=atomicLoad(&invalid);}
            workgroupBarrier();
            if workgroupUniformLoad(&decision)!=0u {
                if lane==0u {data[operator_base+status]=0x100u|atomicLoad(&invalid);}
                return;
            }
            if lane==0u {
                // R-transpose z = active_ids bounds; fixed R stride survives append/release.
                for(var i=0u;i<selected;i++) {
                    var sum=vec3<f32>(0.0);
                    for(var j=0u;j<i;j++) {sum=add(sum,mul(load_pair(r+3u*(i*count+j)),load_pair(z+3u*j)));}
                    let value=divide(add(load_pair(bounds+3u*active_ids[i]),neg(sum)),load_pair(r+3u*(i*count+i)));
                    if !finite_pair(value) {atomicStore(&invalid,1u);}
                    store_pair(z+3u*i,value);
                }
            }
            storageBarrier();workgroupBarrier();
            for(var i=lane;i<width;i+=64u) {
                var sum=vec3<f32>(0.0);
                for(var j=0u;j<selected;j++) {sum=add(sum,mul(load_pair(q+3u*(j*width+i)),load_pair(z+3u*j)));}
                if !finite_pair(sum) {atomicOr(&invalid,1u);}
                store_pair(candidate+3u*i,sum);
            }
            storageBarrier();workgroupBarrier();
            if lane==0u {
                for(var reverse=0u;reverse<selected;reverse++) {
                    let i=selected-1u-reverse;var sum=vec3<f32>(0.0);
                    for(var j=i+1u;j<selected;j++) {sum=add(sum,mul(load_pair(r+3u*(j*count+i)),load_pair(candidate_reactions+3u*j)));}
                    let value=divide(add(load_pair(z+3u*i),neg(sum)),load_pair(r+3u*(i*count+i)));
                    if !finite_pair(value) {atomicStore(&invalid,1u);}
                    store_pair(candidate_reactions+3u*i,value);
                }
                release_index=count;fraction=vec3<f32>(1.0,0.0,0.0);
                for(var i=0u;i<selected;i++) {
                    let next=load_pair(candidate_reactions+3u*i);
                    if next.x+next.y+next.z<0.0 {
                        let old=load_pair(reactions+3u*active_ids[i]);let value=divide(old,add(old,neg(next)));
                        if !finite_pair(value)||value.x+value.y+value.z<0.0||value.x+value.y+value.z>1.0 {atomicStore(&invalid,1u);}
                        if release_index==count||value.x+value.y+value.z<fraction.x+fraction.y+fraction.z {release_index=i;fraction=value;}
                    }
                }
                decision=atomicLoad(&invalid);
            }
            storageBarrier();workgroupBarrier();
            if workgroupUniformLoad(&decision)!=0u {
                if lane==0u {data[operator_base+status]=0x100u|atomicLoad(&invalid);}
                return;
            }
            let released=workgroupUniformLoad(&release_index);let step=workgroupUniformLoad(&fraction);
            if released<count {
                for(var i=lane;i<width;i+=64u) {let old=load_pair(state+3u*i);store_pair(state+3u*i,add(old,mul(step,add(load_pair(candidate+3u*i),neg(old)))));}
                for(var i=lane;i<selected;i+=64u) {
                    let old=load_pair(reactions+3u*active_ids[i]);let value=add(old,mul(step,add(load_pair(candidate_reactions+3u*i),neg(old))));
                    if !finite_pair(value) {atomicOr(&invalid,1u);}
                    store_pair(reactions+3u*active_ids[i],select(value,vec3<f32>(0.0),value.x+value.y+value.z<0.0));
                }
                storageBarrier();workgroupBarrier();
                if lane==0u {
                    let id=active_ids[released];store_pair(reactions+3u*id,vec3<f32>(0.0));flags[id]=0u;
                    for(var i=released;i+1u<selected;i++) {active_ids[i]=active_ids[i+1u];}
                    active_count=selected-1u;built_count=min(built_count,released);
                }
                storageBarrier();workgroupBarrier();
                continue;
            }
            for(var i=lane;i<width;i+=64u) {store_pair(state+3u*i,load_pair(candidate+3u*i));}
            for(var i=lane;i<count;i+=64u) {store_pair(reactions+3u*i,vec3<f32>(0.0));}
            storageBarrier();workgroupBarrier();
            for(var i=lane;i<selected;i+=64u) {store_pair(reactions+3u*active_ids[i],load_pair(candidate_reactions+3u*i));}
            storageBarrier();workgroupBarrier();
        }
        for(var row=lane;row<count;row+=64u) {
            var sum=vec3<f32>(0.0);
            for(var i=0u;i<width;i++) {sum=add(sum,mul(load_pair(input+3u*(row*width+i)),load_pair(state+3u*i)));}
            let gap=add(sum,neg(load_pair(bounds+3u*row)));
            if !finite_pair(gap) {atomicOr(&invalid,1u);}
            store_pair(gaps+3u*row,gap);
        }
        storageBarrier();workgroupBarrier();
        if lane==0u {
            let tol=load_pair(tolerance);let epsilon=tol.x+tol.y+tol.z;var enter=count;var worst=0.0;var complete=true;
            for(var row=0u;row<count;row++) {
                let gap_pair=load_pair(gaps+3u*row);let gap=gap_pair.x+gap_pair.y+gap_pair.z;
                let reaction=load_pair(reactions+3u*row);let positive=reaction.x+reaction.y+reaction.z>0.0;
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
        if workgroupUniformLoad(&decision)!=1u {
            if lane==0u && decision!=0u {data[operator_base+status]=select(2u,0x100u|atomicLoad(&invalid),atomicLoad(&invalid)!=0u);}
            return;
        }
    }
    if lane==0u {data[operator_base+status]=14u;}
}
