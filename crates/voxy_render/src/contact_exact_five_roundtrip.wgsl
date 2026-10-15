// Versioned transfer probe only; no contact solution or numerical admission.
@group(0) @binding(0) var<storage,read_write> words:array<u32>;
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
    if arrayLength(&words)<16u {return;}
    if words[0]!=0x31433556u || words[1]!=1u || words[4]!=5u {return;}
    let count=words[6];let output=words[14];
    if output!=16u+count*5u || words[15]!=output+count*5u || arrayLength(&words)!=words[15] {return;}
    if id.x<count {
        for(var part=0u;part<5u;part++) {
            let value=bitcast<f32>(words[16u+id.x*5u+part]);
            words[output+id.x*5u+part]=bitcast<u32>(value);
        }
    }
    if id.x==0u {words[5]=0u;}
}
