//! Optional source rewrite of exact multiplication; one canonical arithmetic owner.
//! Standalone Metal parity covers flags, noncanonical zeros and every scratch word.
pub(super) fn source(original: &str) -> String {
    let start = original
        .find("fn ex_mul(")
        .expect("canonical exact multiplication");
    let end = original[start + 1..]
        .find("\nfn ")
        .map_or(original.len(), |offset| start + 1 + offset);
    let body = &original[start..end];
    let outer = " for(var i=0u;i<a.length;i+=1u) {";
    let inner = "for(var j=0u;j<b.length;j+=1u)";
    assert_eq!(body.matches(outer).count(), 1);
    assert_eq!(body.matches(inner).count(), 1);
    let changed = body.replace(outer, &format!(
        " // Keep original behavior for non-canonical all-zero operands as well.\n let first_right=ex_low_word(b);let right_begin=select(first_right,0u,first_right==b.length);\n{outer}"
    )).replace(inner, "for(var j=right_begin;j<b.length;j+=1u)");
    format!("{}{}{}", &original[..start], changed, &original[end..])
}
