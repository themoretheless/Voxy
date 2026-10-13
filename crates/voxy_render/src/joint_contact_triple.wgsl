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
