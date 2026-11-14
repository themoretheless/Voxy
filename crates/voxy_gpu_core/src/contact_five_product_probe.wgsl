@group(0) @binding(0) var<storage,read_write> words:array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if arrayLength(&words)<4u || words[0]!=0x31413556u || words[1]!=9u || words[3]!=200u {return;}
    if id.x>=words[2] {return;}
    let base=4u+200u*id.x;
    if base+200u>arrayLength(&words) {return;}
    var a:array<f32,5>;var b:array<f32,5>;
    for(var i=0u;i<5u;i++) {a[i]=bitcast<f32>(words[base+i]);b[i]=bitcast<f32>(words[base+5u+i]);}
    var raw_a:FiveOperand;raw_a.terms=a;raw_a.valid=1u;
    var raw_b:FiveOperand;raw_b.terms=b;raw_b.valid=1u;
    let raw=five_product_terms(raw_a,raw_b);
    let normalized=five_product_terms(five_normalize_operand(a),five_normalize_operand(b));
    let rounded=five_compress_product(normalized);
    let left=five_from_operand(five_normalize_operand(a));let right=five_from_operand(five_normalize_operand(b));
    let addition=five_add(left,right);let subtraction=five_subtract(left,right);
    var uncertain_left=left;var uncertain_right=right;
    uncertain_left.error_bound=ldexp(1.0,-110);uncertain_right.error_bound=ldexp(1.0,-110);
    let propagated=five_add(uncertain_left,uncertain_right);
    var product_left=left;var product_right=right;
    product_left.error_bound=bitcast<f32>(words[base+154u]);product_right.error_bound=bitcast<f32>(words[base+155u]);
    let bounded_product=five_multiply(product_left,product_right);
    let exact_division=five_divide(left,right);words[base+172u]=five_division_failure_stage;
    let bounded_division=five_divide(product_left,product_right);words[base+173u]=five_division_failure_stage;
    let exact_root=five_sqrt(left);words[base+190u]=five_sqrt_failure_stage;
    words[base+192u]=five_arithmetic_failure_code;words[base+193u]=five_arithmetic_failure_bits;words[base+194u]=bitcast<u32>(five_arithmetic_failure_exponent);words[base+195u]=five_arithmetic_failure_other;
    let bounded_root=five_sqrt(product_left);words[base+191u]=five_sqrt_failure_stage;
    words[base+196u]=five_arithmetic_failure_code;words[base+197u]=five_arithmetic_failure_bits;words[base+198u]=bitcast<u32>(five_arithmetic_failure_exponent);words[base+199u]=five_arithmetic_failure_other;
    for(var i=0u;i<5u;i++) {words[base+174u+i]=bitcast<u32>(exact_root.terms[i]);words[base+182u+i]=bitcast<u32>(bounded_root.terms[i]);}
    words[base+179u]=bitcast<u32>(exact_root.exponent);words[base+180u]=bitcast<u32>(exact_root.error_bound);words[base+181u]=exact_root.valid;
    words[base+187u]=bitcast<u32>(bounded_root.exponent);words[base+188u]=bitcast<u32>(bounded_root.error_bound);words[base+189u]=bounded_root.valid;
    for(var i=0u;i<5u;i++) {words[base+156u+i]=bitcast<u32>(exact_division.terms[i]);words[base+164u+i]=bitcast<u32>(bounded_division.terms[i]);}
    words[base+161u]=bitcast<u32>(exact_division.exponent);words[base+162u]=bitcast<u32>(exact_division.error_bound);words[base+163u]=exact_division.valid;
    words[base+169u]=bitcast<u32>(bounded_division.exponent);words[base+170u]=bitcast<u32>(bounded_division.error_bound);words[base+171u]=bounded_division.valid;
    for(var i=0u;i<5u;i++) {words[base+146u+i]=bitcast<u32>(bounded_product.terms[i]);}
    words[base+151u]=bitcast<u32>(bounded_product.exponent);words[base+152u]=bitcast<u32>(bounded_product.error_bound);words[base+153u]=bounded_product.valid;
    for(var i=0u;i<5u;i++) {words[base+138u+i]=bitcast<u32>(propagated.terms[i]);}
    words[base+143u]=bitcast<u32>(propagated.exponent);words[base+144u]=bitcast<u32>(propagated.error_bound);words[base+145u]=propagated.valid;
    for(var i=0u;i<5u;i++) {words[base+122u+i]=bitcast<u32>(addition.terms[i]);words[base+130u+i]=bitcast<u32>(subtraction.terms[i]);}
    words[base+127u]=bitcast<u32>(addition.exponent);words[base+128u]=bitcast<u32>(addition.error_bound);words[base+129u]=addition.valid;
    words[base+135u]=bitcast<u32>(subtraction.exponent);words[base+136u]=bitcast<u32>(subtraction.error_bound);words[base+137u]=subtraction.valid;
    for(var i=0u;i<5u;i++) {words[base+114u+i]=bitcast<u32>(rounded.terms[i]);}
    words[base+119u]=bitcast<u32>(rounded.exponent);
    words[base+120u]=bitcast<u32>(rounded.error_bound);words[base+121u]=rounded.valid;
    for(var i=0u;i<50u;i++) {words[base+10u+i]=bitcast<u32>(raw.terms[i]);words[base+60u+i]=bitcast<u32>(normalized.terms[i]);}
    words[base+110u]=bitcast<u32>(normalized.exponent);
    words[base+111u]=bitcast<u32>(normalized.error_bound);
    words[base+112u]=normalized.valid;words[base+113u]=raw.valid;
}
