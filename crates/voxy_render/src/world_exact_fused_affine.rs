//! Experimental source construction only; original predicate files stay intact.
pub(super) fn source(original: String, cached: bool) -> String {
    let target = if cached {
        "exact_axis_original"
    } else {
        "exact_axis"
    };
    let declaration = format!("fn {target}(");
    assert_eq!(original.matches(&declaration).count(), 1);
    let reference = original.replace(&declaration, "fn exact_axis_reference(");
    let wrapper = r"
fn EXACT_FUSED_AXIS_TARGET(slot:u32,vertex:u32,axis:u32,origin_hi:u32,origin_lo:u32)->ExInt {
 return affine_fused_axis(
  vec3<u32>(exact_instances[slot*28u+axis],exact_instances[slot*28u+4u+axis],exact_instances[slot*28u+8u+axis]),
  vec3<u32>(exact_vertices[vertex*12u],exact_vertices[vertex*12u+1u],exact_vertices[vertex*12u+2u]),
  exact_instances[slot*28u+12u+axis],origin_hi,origin_lo);
}
".replace("EXACT_FUSED_AXIS_TARGET", target);
    [
        reference,
        include_str!("world_exact_fused_affine.wgsl").to_owned(),
        wrapper,
    ]
    .join("\n")
}
