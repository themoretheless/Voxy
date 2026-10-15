var<private> five_arithmetic_failure_code:u32;
var<private> five_arithmetic_failure_bits:u32;
var<private> five_arithmetic_failure_exponent:i32;
var<private> five_arithmetic_failure_other:u32;
fn five_record_arithmetic_failure(code:u32,bits:u32,exponent:i32,other:u32) {
    if five_arithmetic_failure_code==0u {
        five_arithmetic_failure_code=code;five_arithmetic_failure_bits=bits;
        five_arithmetic_failure_exponent=exponent;five_arithmetic_failure_other=other;
    }
}
// Inner product primitive for five-component arithmetic, not a contact solver.
// FMA product/error pairs follow 2MultFMA; exponent normalization prevents
// relevant components disappearing solely because of a shared operator scale.
struct FiveOperand { terms:array<f32,5>, exponent:i32, valid:u32 }
struct FiveProduct { terms:array<f32,50>, exponent:i32, error_bound:f32, valid:u32 }
fn five_normalize_operand(source:array<f32,5>)->FiveOperand {
    var out:FiveOperand;out.valid=1u;
    var leading=0.0;
    for(var i=0u;i<5u;i++) {
        let value=source[i];let bits=bitcast<u32>(value)&0x7fffffffu;
        if bits>=0x7f800000u || (bits!=0u && bits<0x00800000u) {out.valid=0u;return out;}
        leading=max(leading,abs(value));
    }
    if leading==0.0 {return out;}
    out.exponent=i32((bitcast<u32>(leading)>>23u)&255u)-127;
    for(var i=0u;i<5u;i++) {
        out.terms[i]=ldexp(source[i],-out.exponent);
        let bits=bitcast<u32>(out.terms[i])&0x7fffffffu;
        if bits>=0x7f800000u || (source[i]!=0.0 && bits<0x00800000u) {out.valid=0u;}
    }
    return out;
}
fn five_product_terms(a:FiveOperand,b:FiveOperand)->FiveProduct {
    var out:FiveProduct;out.valid=a.valid&b.valid;
    out.exponent=a.exponent+b.exponent;
    if out.valid==0u {return out;}
    var zero_a=true;var zero_b=true;
    for(var i=0u;i<5u;i++) {
        let x=a.terms[i];let y=b.terms[i];
        if x!=x || y!=y || abs(x)>3.402823466e38 || abs(y)>3.402823466e38 {out.valid=0u;return out;}
        zero_a=zero_a&&(x==0.0);zero_b=zero_b&&(y==0.0);
    }
    if zero_a||zero_b {out.exponent=0;return out;}
    // Each FMA product and residual can lose less than a smallest normal f32
    // when the backend flushes subnormals. Fifty such losses bound this primitive
    // only; later renormalization/addition needs its own propagated error bound.
    out.error_bound=50.0*bitcast<f32>(0x00800000u);
    for(var i=0u;i<5u;i++) {for(var j=0u;j<5u;j++) {
        let product=fma(a.terms[i],b.terms[j],0.0);
        let residual=fma(a.terms[i],b.terms[j],-product);
        let at=2u*(5u*i+j);
        out.terms[at]=product;out.terms[at+1u]=residual;
        if abs(product)>3.402823466e38 || abs(residual)>3.402823466e38
            || product!=product || residual!=residual {out.valid=0u;}
    }}
    return out;
}

struct FiveAccumulation { terms:array<f32,50>, count:u32, underflow_units:u32 }
struct FiveRounded { terms:array<f32,5>, exponent:i32, error_bound:f32, valid:u32 }
struct FiveSumPair { x:f32, y:f32, underflow_units:u32 }
fn five_add_may_underflow(a:f32,b:f32,result:f32)->bool {
    let bits=bitcast<u32>(result)&0x7fffffffu;
    return (bits!=0u && bits<0x00800000u) || (bits==0u && a!=-b);
}
fn five_sum_error(a:f32,b:f32)->FiveSumPair {
    let sum=fma(a,1.0,b);let virtual_b=fma(sum,1.0,-a);
    let recovered_a=fma(sum,1.0,-virtual_b);
    let left=fma(a,1.0,-recovered_a);let right=fma(b,1.0,-virtual_b);
    let error=fma(left,1.0,right);
    let uncertain=five_add_may_underflow(a,b,sum)
        ||five_add_may_underflow(sum,-a,virtual_b)
        ||five_add_may_underflow(sum,-virtual_b,recovered_a)
        ||five_add_may_underflow(a,-recovered_a,left)
        ||five_add_may_underflow(b,-virtual_b,right)
        ||five_add_may_underflow(left,right,error);
    return FiveSumPair(sum,error,select(0u,16u,uncertain));
}
fn five_grow(e:ptr<function,FiveAccumulation>,term:f32) {
    let old_count=(*e).count;var count=0u;var q=term;
    for(var i=0u;i<old_count;i++) {
        let pair=five_sum_error(q,(*e).terms[i]);
        (*e).underflow_units+=pair.underflow_units;
        if pair.y!=0.0 {(*e).terms[count]=pair.y;count+=1u;}
        q=pair.x;
    }
    if q!=0.0 || count==0u {(*e).terms[count]=q;count+=1u;}
    (*e).count=count;
}
fn five_bound_add(a:f32,b:f32)->f32 {
    // Inflate positive additions before they are used as upper bounds.
    return fma(a,1.000000238418579,b*1.000000238418579);
}
fn five_compress_product(product:FiveProduct)->FiveRounded {
    var out:FiveRounded;out.valid=product.valid;out.exponent=product.exponent;
    if product.valid==0u {return out;}
    if product.error_bound==0.0 {
        var exact_zero=true;
        for(var i=0u;i<50u;i++) {exact_zero=exact_zero&&(product.terms[i]==0.0);}
        if exact_zero {out.exponent=0;return out;}
    }
    var e:FiveAccumulation;
    for(var i=0u;i<50u;i++) {five_grow(&e,product.terms[i]);}
    var underflow_units=e.underflow_units;
    var h:array<f32,50>;var bottom=e.count;var q=e.terms[e.count-1u];
    for(var index=e.count-1u;index>0u;index-=1u) {
        let pair=five_sum_error(q,e.terms[index-1u]);
        underflow_units+=pair.underflow_units;
        if pair.y!=0.0 {bottom-=1u;h[bottom]=pair.x;q=pair.y;} else {q=pair.x;}
    }
    bottom-=1u;h[bottom]=q;var top=0u;q=h[bottom];
    for(var index=bottom+1u;index<e.count;index+=1u) {
        let pair=five_sum_error(h[index],q);
        underflow_units+=pair.underflow_units;
        if pair.y!=0.0 {h[top]=pair.y;top+=1u;}
        q=pair.x;
    }
    h[top]=q;
    for(var i=0u;i<min(5u,top+1u);i++) {out.terms[i]=h[top-i];}
    // Budget only transforms with a possible flushed/subnormal intermediate.
    // Exact cancellation therefore retains a zero error bound.
    out.error_bound=five_bound_add(product.error_bound,f32(underflow_units)*bitcast<f32>(0x00800000u));
    if top+1u>5u {
        for(var i=0u;i<top+1u-5u;i++) {out.error_bound=five_bound_add(out.error_bound,abs(h[i]));}
    }
    for(var i=0u;i<5u;i++) {
        let bits=bitcast<u32>(out.terms[i])&0x7fffffffu;
        if bits>=0x7f800000u {out.valid=0u;}
    }
    if out.error_bound!=out.error_bound || out.error_bound>3.402823466e38 {out.valid=0u;}
    return five_normalize_rounded(out);
}

fn five_shift_bound(value:f32,exponent:i32)->f32 {
    if value==0.0 {return 0.0;}
    let shifted=ldexp(value,exponent);
    if shifted<bitcast<f32>(0x00800000u) {return bitcast<f32>(0x00800000u);}
    // A power-of-two shift with a normal finite result is exact.
    return shifted;
}
// Arithmetic may create a representable subnormal intermediate even though
// uploaded coefficients are normal. Recover its exact significand with integer
// bits before ldexp; do not ask the floating-point unit to consume a subnormal.
fn five_normalize_arithmetic(source:array<f32,5>)->FiveOperand {
    var out:FiveOperand;out.valid=1u;var leading=0u;var minimum_exponent=128;
    for(var i=0u;i<5u;i++) {
        let bits=bitcast<u32>(source[i])&0x7fffffffu;
        if bits>=0x7f800000u {five_record_arithmetic_failure(1u,bits,0,0u);out.valid=0u;return out;}
        leading=max(leading,bits);
        if bits!=0u {
            let exponent=select(-149+i32(31u-countLeadingZeros(max(bits,1u))),i32((bits>>23u)&255u)-127,bits>=0x00800000u);
            minimum_exponent=min(minimum_exponent,exponent);
        }
    }
    if leading==0u {return out;}
    if leading<0x00800000u {out.exponent=-149+i32(31u-countLeadingZeros(leading));}
    else {out.exponent=i32((leading>>23u)&255u)-127;}
    let leading_exponent=out.exponent;
    // A normalized leading component is optional; losing a low component is not.
    // Leave enough exponent headroom to keep every component normal and exact.
    out.exponent=min(out.exponent,minimum_exponent+126);
    if leading_exponent-out.exponent>48 {five_record_arithmetic_failure(8u,leading,out.exponent,u32(minimum_exponent));out.valid=0u;return out;}
    for(var i=0u;i<5u;i++) {
        let raw=bitcast<u32>(source[i]);let bits=raw&0x7fffffffu;
        if bits!=0u && bits<0x00800000u {
            let significand=f32(bits);
            let scaled=ldexp(significand,-149-out.exponent);
            out.terms[i]=select(scaled,-scaled,(raw&0x80000000u)!=0u);
        } else {out.terms[i]=ldexp(source[i],-out.exponent);}
        let normalized=bitcast<u32>(out.terms[i])&0x7fffffffu;
        if normalized>=0x7f800000u {five_record_arithmetic_failure(2u,raw,out.exponent,normalized);out.valid=0u;}
        if bits!=0u && normalized<0x00800000u {five_record_arithmetic_failure(3u,raw,out.exponent,normalized);out.valid=0u;}
    }
    return out;
}
fn five_normalize_rounded(value:FiveRounded)->FiveRounded {
    var out=value;
    if value.valid==0u {return out;}
    if value.error_bound<0.0 || value.error_bound!=value.error_bound
        || value.error_bound>3.402823466e38 {five_record_arithmetic_failure(4u,bitcast<u32>(value.error_bound),value.exponent,0u);out.valid=0u;return out;}
    let normalized=five_normalize_arithmetic(value.terms);
    out.terms=normalized.terms;out.exponent=value.exponent+normalized.exponent;
    out.valid=normalized.valid;
    out.error_bound=five_shift_bound(value.error_bound,-normalized.exponent);
    if out.error_bound>3.402823466e38 {five_record_arithmetic_failure(5u,bitcast<u32>(value.error_bound),normalized.exponent,bitcast<u32>(out.error_bound));out.valid=0u;}
    return out;
}
fn five_from_operand(value:FiveOperand)->FiveRounded {
    return FiveRounded(value.terms,value.exponent,0.0,value.valid);
}
fn five_negate(value:FiveRounded)->FiveRounded {
    var out=value;
    for(var i=0u;i<5u;i++) {out.terms[i]=-value.terms[i];}
    return out;
}
fn five_add(a:FiveRounded,b:FiveRounded)->FiveRounded {
    var terms:FiveProduct;terms.valid=a.valid&b.valid;
    if a.error_bound<0.0 || b.error_bound<0.0 || a.error_bound!=a.error_bound || b.error_bound!=b.error_bound
        || a.error_bound>3.402823466e38 || b.error_bound>3.402823466e38 {terms.valid=0u;}
    terms.exponent=max(a.exponent,b.exponent);
    if terms.valid==0u {return five_compress_product(terms);}
    if five_exact_zero(a) {return five_normalize_rounded(b);}
    if five_exact_zero(b) {return five_normalize_rounded(a);}
    let shift_a=a.exponent-terms.exponent;let shift_b=b.exponent-terms.exponent;
    terms.error_bound=five_bound_add(five_shift_bound(a.error_bound,shift_a),five_shift_bound(b.error_bound,shift_b));
    for(var i=0u;i<5u;i++) {
        let x=ldexp(a.terms[i],shift_a);let y=ldexp(b.terms[i],shift_b);
        terms.terms[i]=x;terms.terms[5u+i]=y;
        if a.terms[i]!=0.0 && abs(x)<bitcast<f32>(0x00800000u) {terms.error_bound=five_bound_add(terms.error_bound,bitcast<f32>(0x00800000u));}
        if b.terms[i]!=0.0 && abs(y)<bitcast<f32>(0x00800000u) {terms.error_bound=five_bound_add(terms.error_bound,bitcast<f32>(0x00800000u));}
    }
    return five_normalize_rounded(five_compress_product(terms));
}
fn five_subtract(a:FiveRounded,b:FiveRounded)->FiveRounded {return five_add(a,five_negate(b));}

fn five_bound_mul(a:f32,b:f32)->f32 {
    if a==0.0 || b==0.0 {return 0.0;}
    let product=fma(a,b,0.0);
    if product<bitcast<f32>(0x00800000u) {return bitcast<f32>(0x00800000u);}
    return product*1.000000238418579;
}
fn five_magnitude_bound(value:FiveRounded)->f32 {
    var magnitude=0.0;
    for(var i=0u;i<5u;i++) {magnitude=five_bound_add(magnitude,abs(value.terms[i]));}
    return magnitude;
}
fn five_multiply(source_a:FiveRounded,source_b:FiveRounded)->FiveRounded {
    let a=five_normalize_rounded(source_a);let b=five_normalize_rounded(source_b);
    var product=five_product_terms(FiveOperand(a.terms,a.exponent,a.valid),FiveOperand(b.terms,b.exponent,b.valid));
    if product.valid==0u {return five_compress_product(product);}
    if a.error_bound!=0.0 || b.error_bound!=0.0 {
        // |(A+da)(B+db)-AB| <= |A| Eb + |B| Ea + Ea Eb.
        // Errors are expressed in the same normalized exponent units as A/B.
        let cross_a=five_bound_mul(five_magnitude_bound(a),b.error_bound);
        let cross_b=five_bound_mul(five_magnitude_bound(b),a.error_bound);
        let cross_error=five_bound_mul(a.error_bound,b.error_bound);
        product.error_bound=five_bound_add(product.error_bound,five_bound_add(five_bound_add(cross_a,cross_b),cross_error));
        // A zero nominal operand can still have a nonzero uncertainty interval.
        product.exponent=a.exponent+b.exponent;
    }
    return five_normalize_rounded(five_compress_product(product));
}

fn five_lower_magnitude(value:FiveRounded)->f32 {
    var tail=0.0;
    for(var i=1u;i<5u;i++) {tail=five_bound_add(tail,abs(value.terms[i]));}
    let nominal=max(0.0,(abs(value.terms[0])-tail)*0.999999761581421);
    return max(0.0,(nominal-value.error_bound)*0.999999761581421);
}
fn five_bound_divide(a:f32,b:f32)->f32 {
    if a==0.0 {return 0.0;}
    let ratio=a/b;
    if ratio<bitcast<f32>(0x00800000u) {return bitcast<f32>(0x00800000u);}
    return ratio*1.000000238418579;
}
var<private> five_division_failure_stage:u32;
fn five_divide(source_a:FiveRounded,source_b:FiveRounded)->FiveRounded {
    five_division_failure_stage=0u;
    let a=five_normalize_rounded(source_a);let b=five_normalize_rounded(source_b);
    var quotient:FiveRounded;quotient.valid=a.valid&b.valid;
    quotient.exponent=a.exponent-b.exponent;
    if quotient.valid==0u {five_division_failure_stage=1u;return quotient;}
    let denominator_lower=five_lower_magnitude(b);
    if denominator_lower<=0.0 || b.terms[0]==0.0 {quotient.valid=0u;five_division_failure_stage=2u;return quotient;}
    quotient.terms[0]=a.terms[0]/b.terms[0];
    quotient=five_normalize_rounded(quotient);
    var nominal_a=a;nominal_a.error_bound=0.0;
    var nominal_b=b;nominal_b.error_bound=0.0;
    var residual=five_subtract(nominal_a,five_multiply(nominal_b,quotient));
    if residual.valid==0u {quotient.valid=0u;five_division_failure_stage=3u;return quotient;}
    for(var iteration=0u;iteration<7u;iteration++) {
        // Stop only after the carried residual certifies more than 106 bits.
        // Further corrections can shrink its centre below its uncertainty and
        // overflow the error representation without improving the certificate.
        let residual_upper=five_bound_add(five_magnitude_bound(residual),residual.error_bound);
        let candidate_error=five_shift_bound(five_bound_divide(residual_upper,five_lower_magnitude(nominal_b)),residual.exponent-b.exponent-quotient.exponent);
        if candidate_error<=ldexp(five_lower_magnitude(quotient),-108) || residual.terms[0]==0.0 {break;}
        var correction:FiveRounded;correction.valid=1u;
        correction.terms[0]=residual.terms[0]/b.terms[0];
        correction.exponent=residual.exponent-b.exponent;
        quotient.error_bound=0.0;
        let updated=five_add(quotient,correction);
        if updated.valid==0u {quotient.valid=0u;five_division_failure_stage=20u+iteration;return quotient;}
        // Update A-BQ at its own exponent instead of repeatedly subtracting
        // nearly equal large A and BQ. Account for rounding Q+c into its five
        // stored terms: B*(Q+c-Qnew) belongs in the residual interval.
        residual=five_subtract(residual,five_multiply(nominal_b,correction));
        if residual.valid==0u {quotient.valid=0u;five_division_failure_stage=10u+iteration;return quotient;}
        let rounded_update=five_bound_mul(five_magnitude_bound(nominal_b),updated.error_bound);
        residual.error_bound=five_bound_add(residual.error_bound,five_shift_bound(rounded_update,b.exponent+updated.exponent-residual.exponent));
        residual=five_normalize_rounded(residual);
        quotient=updated;quotient.error_bound=0.0;
        if residual.valid==0u {quotient.valid=0u;five_division_failure_stage=30u;return quotient;}
    }
    // Keep uncertainty terms in quotient units. Scaling wide input intervals
    // into the exponent of a tiny converged residual can overflow needlessly.
    let residual_upper=five_bound_add(five_magnitude_bound(residual),residual.error_bound);
    let nominal_error=five_shift_bound(five_bound_divide(residual_upper,denominator_lower),residual.exponent-b.exponent-quotient.exponent);
    let numerator_error=five_shift_bound(five_bound_divide(a.error_bound,denominator_lower),a.exponent-b.exponent-quotient.exponent);
    let denominator_error=five_bound_divide(five_bound_mul(five_magnitude_bound(quotient),b.error_bound),denominator_lower);
    quotient.error_bound=five_bound_add(nominal_error,five_bound_add(numerator_error,denominator_error));
    quotient=five_normalize_rounded(quotient);
    if quotient.valid==0u {five_division_failure_stage=31u;}
    return quotient;
}

var<private> five_sqrt_failure_stage:u32;
fn five_exact_zero(value:FiveRounded)->bool {
    if value.error_bound!=0.0 {return false;}
    for(var i=0u;i<5u;i++) {if value.terms[i]!=0.0 {return false;}}
    return true;
}
fn five_sqrt(source:FiveRounded)->FiveRounded {
    five_sqrt_failure_stage=0u;five_arithmetic_failure_code=0u;
    let a=five_normalize_rounded(source);
    var root:FiveRounded;root.valid=a.valid;
    if root.valid==0u {five_sqrt_failure_stage=1u;return root;}
    if a.terms[0]<0.0 {root.valid=0u;five_sqrt_failure_stage=2u;return root;}
    var nominal_a=a;nominal_a.error_bound=0.0;
    var radius:FiveRounded;radius.valid=1u;radius.terms[0]=a.error_bound;radius.exponent=a.exponent;
    let domain_lower=five_subtract(nominal_a,radius);
    if domain_lower.valid==0u {root.valid=0u;five_sqrt_failure_stage=3u;return root;}
    if domain_lower.terms[0]<0.0 {root.valid=0u;five_sqrt_failure_stage=2u;return root;}
    if !five_exact_zero(domain_lower) && five_lower_magnitude(domain_lower)<=0.0 {root.valid=0u;five_sqrt_failure_stage=3u;return root;}
    if five_exact_zero(a) {return root;}
    let half_exponent=a.exponent>>1u;
    let odd=a.exponent-2*half_exponent;
    root.exponent=half_exponent;root.terms[0]=sqrt(ldexp(a.terms[0],odd));
    root=five_normalize_rounded(root);
    if root.valid==0u || root.terms[0]<=0.0 {root.valid=0u;five_sqrt_failure_stage=4u;return root;}
    var residual=five_subtract(nominal_a,five_multiply(root,root));
    if residual.valid==0u {root.valid=0u;five_sqrt_failure_stage=10u;return root;}
    for(var iteration=0u;iteration<4u;iteration++) {
        let root_lower=five_lower_magnitude(root);
        let residual_upper=five_bound_add(five_magnitude_bound(residual),residual.error_bound);
        let nominal_error=five_shift_bound(five_bound_divide(residual_upper,root_lower),residual.exponent-2*root.exponent);
        // A certified residual ends refinement. A zero stored residual has no
        // further candidate correction; the final bound still governs precision.
        if nominal_error<=ldexp(root_lower,-108) || residual.terms[0]==0.0 {break;}
        var twice_root=root;twice_root.exponent+=1;
        var chosen_residual=residual;chosen_residual.error_bound=0.0;
        var correction=five_divide(chosen_residual,twice_root);
        if correction.valid==0u {root.valid=0u;five_sqrt_failure_stage=20u+iteration;return root;}
        // A computed correction is a chosen exact candidate value. Its division
        // error is not assumed zero in the final answer: the carried A-x^2
        // residual certifies the resulting stored root independently.
        correction.error_bound=0.0;
        let updated=five_add(root,correction);
        if updated.valid==0u {root.valid=0u;five_sqrt_failure_stage=30u+iteration;return root;}
        let residual_step=five_add(five_multiply(twice_root,correction),five_multiply(correction,correction));
        residual=five_subtract(residual,residual_step);
        if residual.valid==0u {root.valid=0u;five_sqrt_failure_stage=40u+iteration;return root;}
        let rounded_update=five_bound_mul(five_bound_add(2.0*five_magnitude_bound(updated),updated.error_bound),updated.error_bound);
        residual.error_bound=five_bound_add(residual.error_bound,five_shift_bound(rounded_update,2*updated.exponent-residual.exponent));
        residual=five_normalize_rounded(residual);
        root=updated;root.error_bound=0.0;
        if residual.valid==0u {root.valid=0u;five_sqrt_failure_stage=50u+iteration;return root;}
    }
    // |x-sqrt(A)| = |x^2-A|/(x+sqrt(A)) <= |x^2-A|/x for A>=0,x>0.
    let root_lower=five_lower_magnitude(root);
    let residual_upper=five_bound_add(five_magnitude_bound(residual),residual.error_bound);
    let nominal_error=five_shift_bound(five_bound_divide(residual_upper,root_lower),residual.exponent-2*root.exponent);
    let input_error=five_shift_bound(five_bound_divide(a.error_bound,root_lower),a.exponent-2*root.exponent);
    root.error_bound=five_bound_add(nominal_error,input_error);
    root=five_normalize_rounded(root);
    if root.valid==0u {five_sqrt_failure_stage=60u;}
    return root;
}
